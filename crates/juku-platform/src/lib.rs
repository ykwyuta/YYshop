//! Windows（ネイティブ）と WebAssembly の差分を吸収する層。
//!
//! - ウィンドウとイベントループ（winit）
//! - GPU の初期化（wgpu。Windows は DX12/Vulkan、Web は WebGPU か WebGL2）
//! - 入力の変換（物理座標 → 論理座標、マウス・タッチ・ホイール）
//! - 保存（Windows はファイル、Web は localStorage）
//!
//! `#[cfg(target_arch = "wasm32")]` はこのクレートの中に閉じ込め、上位には出さない。

mod storage;

use std::sync::Arc;

use juku_render::{wgpu, Color, DrawList, Rect, Renderer, TextSystem};
use juku_ui::{InputState, Memory, Theme, Ui};
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

pub use storage::{show_fatal, Storage};

/// アプリ本体（画面）が実装する。
pub trait App: 'static {
    /// 1 フレーム分の UI を組み立てる。
    fn update(&mut self, ui: &mut Ui, ctx: &mut Ctx);

    fn clear_color(&self, theme: &Theme) -> Color {
        theme.bg
    }
}

/// フレーム中にアプリが使えるプラットフォームの機能。
pub struct Ctx<'a> {
    pub storage: &'a mut Storage,
    pub is_web: bool,
}

impl Ctx<'_> {
    /// ブラウザのキャッシュを削除して再読み込みする（Web のみ。ネイティブでは何もしない）。
    /// セーブデータは消さない。
    pub fn force_reload(&mut self) {
        storage::force_reload();
    }
}

pub enum FontSource {
    /// 実行ファイルに埋め込んだフォント（Windows 版）
    Embedded { regular: &'static [u8], bold: &'static [u8] },
    /// 別ファイルとして取得するフォント（Web 版）
    Url { regular: String, bold: String },
}

pub struct Config {
    pub title: String,
    /// 初期ウィンドウサイズ（論理 px、ネイティブのみ）
    pub size: (f32, f32),
    pub fonts: FontSource,
    /// Web 版で描画先にする canvas の id
    pub canvas_id: String,
    /// ウィンドウ・タスクバーのアイコン（PNG、ネイティブのみ）
    pub icon_png: Option<&'static [u8]>,
}

/// PNG をウィンドウアイコンにする。
#[cfg(not(target_arch = "wasm32"))]
fn load_icon(bytes: &[u8]) -> Option<winit::window::Icon> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        _ => return None,
    };
    winit::window::Icon::from_rgba(rgba, info.width, info.height).ok()
}

struct Gfx {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
enum UserEvent {
    Ready(Box<Gfx>, Box<TextSystem>),
    Failed(String),
}

struct Handler<A: App> {
    make_app: Option<Box<dyn FnOnce(&mut Storage) -> A>>,
    app: Option<A>,
    config: Config,
    window: Option<Arc<Window>>,
    gfx: Option<Gfx>,
    text: Option<TextSystem>,
    #[allow(dead_code)]
    proxy: EventLoopProxy<UserEvent>,
    input: InputState,
    mem: Memory,
    theme: Theme,
    dl: DrawList,
    storage: Storage,
    start: Instant,
    /// 物理 px のポインタ位置
    last_pointer_px: Option<PhysicalPosition<f64>>,
    active_touch: Option<u64>,
}

/// アプリを起動する。ネイティブではこの関数から戻らない。
pub fn run<A: App>(config: Config, make_app: impl FnOnce(&mut Storage) -> A + 'static) {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let handler = Handler {
        make_app: Some(Box::new(make_app)),
        app: None,
        config,
        window: None,
        gfx: None,
        text: None,
        proxy,
        input: InputState::new(),
        mem: Memory::new(),
        theme: Theme::default(),
        dl: DrawList::new(),
        storage: Storage::new(),
        start: Instant::now(),
        last_pointer_px: None,
        active_touch: None,
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut handler = handler;
        event_loop.run_app(&mut handler).expect("event loop error");
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(handler);
    }
}

async fn init_gfx(instance: wgpu::Instance, window: Arc<Window>, size: PhysicalSize<u32>) -> Result<Gfx, String> {
    let surface = instance.create_surface(window).map_err(|e| format!("surface: {e}"))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            ..Default::default()
        })
        .await
        .map_err(|e| format!("adapter: {e}"))?;
    log::info!("GPU: {:?}", adapter.get_info());

    let limits = if cfg!(target_arch = "wasm32") {
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
    } else {
        wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits())
    };
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("juku device"),
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            ..Default::default()
        })
        .await
        .map_err(|e| format!("device: {e}"))?;

    // GPU の検証エラーは画面にも出す（スマホでは開発者ツールが見られないため）
    device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| {
        log::error!("GPU エラー: {e}");
        storage::show_fatal(&format!("GPU エラー：{e}"));
    }));

    let caps = surface.get_capabilities(&adapter);
    // 色をそのまま扱えるよう、sRGB でない形式を優先する
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|f| !f.is_srgb())
        .or_else(|| caps.formats.first().copied())
        .ok_or("no surface format")?;
    let mut config =
        surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).ok_or("surface not supported")?;
    config.format = format;
    config.present_mode = wgpu::PresentMode::AutoVsync;
    config.alpha_mode = caps.alpha_modes[0];
    surface.configure(&device, &config);
    let renderer = Renderer::new(&device, format);
    Ok(Gfx { surface, device, queue, config, renderer })
}

/// Web：ふだんは WebGL2 で描く。WebGPU はブラウザによる差がまだ大きく
/// （iOS の Safari などで真っ暗になることがある）、この 2D の画面では利点も小さいため、
/// URL に `?gpu=webgpu` をつけたときだけ試す。
///
/// canvas は一度 WebGPU のコンテキストを取ると WebGL2 に切り替えられないため、
/// 先に surface なしでアダプタの有無だけを確かめる。
#[cfg(target_arch = "wasm32")]
async fn init_gfx_web(window: Arc<Window>, size: PhysicalSize<u32>) -> Result<Gfx, String> {
    let make = |backends| {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        })
    };
    if storage::query_has("gpu=webgpu") {
        let webgpu = make(wgpu::Backends::BROWSER_WEBGPU);
        let probe = webgpu.request_adapter(&wgpu::RequestAdapterOptions::default()).await;
        if probe.is_ok() {
            drop(probe);
            log::info!("WebGPU で描画します");
            return init_gfx(webgpu, window, size).await;
        }
        log::info!("WebGPU が使えないため WebGL2 で描画します");
    }
    init_gfx(make(wgpu::Backends::GL), window, size).await
}

/// 描画先の物理ピクセルサイズ。
fn surface_size(window: &Window) -> PhysicalSize<u32> {
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        if let Some(canvas) = window.canvas() {
            let dpr = window.scale_factor();
            let w = (canvas.client_width() as f64 * dpr).round() as u32;
            let h = (canvas.client_height() as f64 * dpr).round() as u32;
            if w > 0 && h > 0 {
                return PhysicalSize::new(w, h);
            }
        }
    }
    window.inner_size()
}

impl<A: App> Handler<A> {
    fn scale(&self) -> f64 {
        self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0)
    }

    fn request_redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn set_pointer(&mut self, pos: PhysicalPosition<f64>) {
        self.last_pointer_px = Some(pos);
        let s = self.scale();
        self.input.pointer_moved((pos.x / s) as f32, (pos.y / s) as f32);
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(g) = &mut self.gfx {
            if size.width > 0 && size.height > 0 {
                g.config.width = size.width;
                g.config.height = size.height;
                g.surface.configure(&g.device, &g.config);
            }
        }
        self.request_redraw();
    }

    fn redraw(&mut self) {
        let (Some(window), Some(gfx), Some(text)) = (&self.window, &mut self.gfx, &mut self.text) else {
            return;
        };
        if self.app.is_none() {
            if let Some(make) = self.make_app.take() {
                self.app = Some(make(&mut self.storage));
            }
        }
        let Some(app) = &mut self.app else { return };

        let scale = window.scale_factor() as f32;
        // 描画先のサイズが変わっていたら作り直す（Web では Resized が来ない場合がある）
        let want = surface_size(window);
        if want.width > 0 && want.height > 0 && (want.width, want.height) != (gfx.config.width, gfx.config.height) {
            gfx.config.width = want.width;
            gfx.config.height = want.height;
            gfx.surface.configure(&gfx.device, &gfx.config);
        }
        let size = (gfx.config.width, gfx.config.height);
        text.set_scale(scale);
        self.input.begin_frame(self.start.elapsed().as_secs_f64());
        self.dl.clear();
        let screen = Rect::new(0.0, 0.0, size.0 as f32 / scale, size.1 as f32 / scale);
        {
            let mut ctx = Ctx { storage: &mut self.storage, is_web: cfg!(target_arch = "wasm32") };
            let mut ui = Ui::new(&mut self.dl, text, &self.input, &mut self.mem, &self.theme, screen);
            app.update(&mut ui, &mut ctx);
        }
        let overflowed = text.take_overflowed();
        // クリックで状態が変わった結果を反映するため、入力があったフレームの後にもう 1 枚描く
        let had_input = self.input.has_activity();
        self.input.end_frame();

        let frame = match gfx.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                gfx.surface.configure(&gfx.device, &gfx.config);
                window.request_redraw();
                return;
            }
            _ => {
                window.request_redraw();
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let clear = app.clear_color(&self.theme);
        gfx.renderer.render(&gfx.device, &gfx.queue, &view, size, scale, &self.dl, text, clear);
        window.pre_present_notify();
        gfx.queue.present(frame);

        if self.mem.take_repaint() || overflowed || had_input {
            window.request_redraw();
        }
    }
}

impl<A: App> ApplicationHandler<UserEvent> for Handler<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        #[allow(unused_mut)]
        let mut attrs = Window::default_attributes().with_title(self.config.title.clone());
        #[cfg(not(target_arch = "wasm32"))]
        {
            attrs = attrs
                .with_inner_size(LogicalSize::new(self.config.size.0, self.config.size.1))
                .with_min_inner_size(LogicalSize::new(360.0, 560.0))
                .with_window_icon(self.config.icon_png.and_then(load_icon));
        }
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id(&self.config.canvas_id))
                .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok());
            attrs = attrs.with_canvas(canvas).with_prevent_default(true);
            let _ = LogicalSize::new(self.config.size.0, self.config.size.1);
        }
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        self.window = Some(window.clone());

        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
            event_loop.owned_display_handle(),
        )));

        let size = window.inner_size();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let FontSource::Embedded { regular, bold } = &self.config.fonts else {
                panic!("ネイティブ版はフォントを埋め込む必要があります");
            };
            let text = TextSystem::new(regular, bold).expect("font");
            let gfx = pollster::block_on(init_gfx(instance, window.clone(), size)).unwrap_or_else(|e| {
                panic!("GPU の初期化に失敗しました: {e}");
            });
            self.gfx = Some(gfx);
            self.text = Some(text);
            window.request_redraw();
        }
        #[cfg(target_arch = "wasm32")]
        {
            let proxy = self.proxy.clone();
            let (regular_url, bold_url) = match &self.config.fonts {
                FontSource::Url { regular, bold } => (regular.clone(), bold.clone()),
                FontSource::Embedded { .. } => (String::new(), String::new()),
            };
            let embedded = match &self.config.fonts {
                FontSource::Embedded { regular, bold } => Some((*regular, *bold)),
                _ => None,
            };
            wasm_bindgen_futures::spawn_local(async move {
                let fonts = match embedded {
                    Some((r, b)) => Ok((r.to_vec(), b.to_vec())),
                    None => {
                        let reload = storage::take_force_refetch();
                        let r = storage::fetch_bytes(&regular_url, reload).await;
                        let b = storage::fetch_bytes(&bold_url, reload).await;
                        r.and_then(|r| b.map(|b| (r, b)))
                    }
                };
                let result = match fonts {
                    Ok((r, b)) => match TextSystem::new(&r, &b) {
                        Ok(text) => {
                            init_gfx_web(window, size).await.map(|g| UserEvent::Ready(Box::new(g), Box::new(text)))
                        }
                        Err(e) => Err(format!("font: {e}")),
                    },
                    Err(e) => Err(e),
                };
                let ev = result.unwrap_or_else(UserEvent::Failed);
                let _ = proxy.send_event(ev);
            });
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Ready(gfx, text) => {
                self.gfx = Some(*gfx);
                self.text = Some(*text);
                if let Some(w) = &self.window {
                    let size = w.inner_size();
                    self.resize(size);
                }
                storage::hide_loading();
            }
            UserEvent::Failed(e) => {
                log::error!("起動に失敗しました: {e}");
                storage::show_error(&e);
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(w) = &self.window {
                    let size = w.inner_size();
                    self.resize(size);
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::CursorMoved { position, .. } => {
                if self.active_touch.is_none() {
                    self.set_pointer(position);
                    if self.input.down {
                        self.request_redraw();
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.input.pointer_left();
                self.request_redraw();
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                match state {
                    ElementState::Pressed => self.input.pointer_down(false),
                    ElementState::Released => self.input.pointer_up(),
                }
                self.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let s = self.scale() as f32;
                let (dx, dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x * 48.0, y * 48.0),
                    MouseScrollDelta::PixelDelta(p) => (p.x as f32 / s, p.y as f32 / s),
                };
                self.input.wheel(dx, dy);
                self.request_redraw();
            }
            WindowEvent::Touch(t) => {
                match t.phase {
                    TouchPhase::Started => {
                        if self.active_touch.is_none() {
                            self.active_touch = Some(t.id);
                            self.set_pointer(t.location);
                            self.input.pointer_down(true);
                        }
                    }
                    TouchPhase::Moved => {
                        if self.active_touch == Some(t.id) {
                            self.set_pointer(t.location);
                        }
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        if self.active_touch == Some(t.id) {
                            self.set_pointer(t.location);
                            self.input.pointer_up();
                            self.active_touch = None;
                        }
                    }
                }
                self.request_redraw();
            }
            _ => {}
        }
        let _ = self.last_pointer_px;
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    #[test]
    fn app_icon_decodes() {
        let bytes = include_bytes!("../../../assets/icon/icon-256.png");
        assert!(super::load_icon(bytes).is_some());
    }
}
