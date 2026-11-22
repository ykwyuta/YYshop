//! 自作の即時モード UI ツールキット。
//!
//! 画面コードは毎フレーム `ui.card(..)` `ui.toggle(..)` のように宣言的に書く。
//! スクロール位置やアニメーションなど、フレームをまたぐ状態は [`Memory`] に ID ごとに保持する。
//!
//! レイアウトは縦積み（カーソル方式）を基本に、`columns` で横に分割する。
//! カードの背景は「場所取り → 中身を描く → 高さが決まってから背景を埋める」方式で描く。

mod charts;
mod input;
mod theme;
mod widgets;

use std::collections::HashMap;

pub use charts::*;
pub use input::InputState;
pub use juku_render::{Color, DrawList, FontWeight, Rect, TextSystem};
pub use theme::Theme;
pub use widgets::*;

/// ウィジェット ID（文字列のハッシュ）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id(pub u64);

impl Id {
    pub fn new(s: &str) -> Id {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in s.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        Id(h)
    }

    pub fn with(self, n: u64) -> Id {
        let mut h = self.0 ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h = h.wrapping_mul(0x0100_0000_01b3);
        Id(h)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ScrollState {
    offset: f32,
    velocity: f32,
    content: f32,
    viewport: f32,
    dragging: bool,
}

/// フレームをまたいで保持する UI の状態。
#[derive(Default)]
pub struct Memory {
    floats: HashMap<Id, f32>,
    scrolls: HashMap<Id, ScrollState>,
    repaint: bool,
    /// 長押しリピートの次回発火時刻
    repeat_at: HashMap<Id, f64>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    /// アニメーション・慣性スクロール中なら true（次のフレームを描く必要がある）。
    pub fn take_repaint(&mut self) -> bool {
        std::mem::take(&mut self.repaint)
    }

    pub fn request_repaint(&mut self) {
        self.repaint = true;
    }

    pub fn reset_scroll(&mut self, id: Id) {
        self.scrolls.remove(&id);
    }
}

#[derive(Clone, Copy, Debug)]
struct Layout {
    x: f32,
    w: f32,
    y: f32,
    spacing: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub struct Ui<'a> {
    pub dl: &'a mut DrawList,
    pub text: &'a mut TextSystem,
    pub input: &'a InputState,
    pub mem: &'a mut Memory,
    pub theme: &'a Theme,
    /// false の間はクリックなどの入力を受け付けない（モーダル表示中の背面など）
    pub input_enabled: bool,
    screen: Rect,
    layouts: Vec<Layout>,
}

impl<'a> Ui<'a> {
    pub fn new(
        dl: &'a mut DrawList,
        text: &'a mut TextSystem,
        input: &'a InputState,
        mem: &'a mut Memory,
        theme: &'a Theme,
        screen: Rect,
    ) -> Self {
        Ui {
            dl,
            text,
            input,
            mem,
            theme,
            input_enabled: true,
            screen,
            layouts: vec![Layout { x: screen.x, w: screen.w, y: screen.y, spacing: 0.0 }],
        }
    }

    pub fn screen(&self) -> Rect {
        self.screen
    }

    fn layout(&self) -> &Layout {
        self.layouts.last().expect("layout stack")
    }

    fn layout_mut(&mut self) -> &mut Layout {
        self.layouts.last_mut().expect("layout stack")
    }

    /// 現在のレイアウトの幅。
    pub fn width(&self) -> f32 {
        self.layout().w
    }

    pub fn cursor(&self) -> (f32, f32) {
        let l = self.layout();
        (l.x, l.y)
    }

    pub fn set_spacing(&mut self, s: f32) {
        self.layout_mut().spacing = s;
    }

    /// 高さ `h` の領域を確保して返す。
    pub fn alloc(&mut self, h: f32) -> Rect {
        let l = self.layout_mut();
        let r = Rect::new(l.x, l.y, l.w, h);
        l.y += h + l.spacing;
        r
    }

    pub fn space(&mut self, h: f32) {
        self.layout_mut().y += h;
    }

    /// 子レイアウトを開始する。
    pub fn begin(&mut self, x: f32, y: f32, w: f32, spacing: f32) {
        self.layouts.push(Layout { x, w, y, spacing });
    }

    /// 子レイアウトを終了し、到達した y（最後の間隔を除く）を返す。
    pub fn end(&mut self) -> f32 {
        let l = self.layouts.pop().expect("layout stack");
        l.y - l.spacing
    }

    /// 内側に余白をとった縦積み領域。
    pub fn padded<R>(&mut self, pad_x: f32, pad_y: f32, f: impl FnOnce(&mut Ui) -> R) -> R {
        let (x, y) = self.cursor();
        let w = self.width();
        let spacing = self.layout().spacing;
        self.begin(x + pad_x, y + pad_y, w - pad_x * 2.0, spacing);
        let r = f(self);
        let end = self.end();
        let h = (end - y + pad_y).max(0.0);
        self.alloc(h);
        r
    }

    /// 横に `n` 等分した列。`f(ui, i)` を列ごとに呼ぶ。行の高さは最も高い列に合わせる。
    pub fn columns(&mut self, n: usize, gap: f32, mut f: impl FnMut(&mut Ui, usize)) {
        let weights = vec![1.0; n];
        self.columns_weighted(&weights, gap, |ui, i| f(ui, i));
    }

    pub fn columns_weighted(&mut self, weights: &[f32], gap: f32, mut f: impl FnMut(&mut Ui, usize)) {
        let (x, y) = self.cursor();
        let w = self.width();
        let spacing = self.layout().spacing;
        let total: f32 = weights.iter().sum();
        let avail = w - gap * (weights.len().saturating_sub(1)) as f32;
        let mut cx = x;
        let mut max_end = y;
        for (i, wt) in weights.iter().enumerate() {
            let cw = avail * wt / total;
            self.begin(cx, y, cw, spacing);
            f(self, i);
            let end = self.end();
            max_end = max_end.max(end);
            cx += cw + gap;
        }
        self.alloc((max_end - y).max(0.0));
    }

    // ------------------------------------------------------------------
    // 入力
    // ------------------------------------------------------------------

    fn pointer_in(&self, r: Rect) -> bool {
        let Some((px, py)) = self.input.pointer else { return false };
        if !r.contains(px, py) {
            return false;
        }
        match self.dl.current_clip() {
            Some(c) => c.contains(px, py),
            None => true,
        }
    }

    /// `r` の中で押して離した（ドラッグではない）。
    pub fn clicked(&self, r: Rect) -> bool {
        if !self.input_enabled || !self.input.released || self.input.is_drag() {
            return false;
        }
        let Some((ox, oy)) = self.input.press_origin else { return false };
        self.pointer_in(r) && r.contains(ox, oy)
    }

    /// `r` が押されている最中。
    pub fn pressed(&self, r: Rect) -> bool {
        if !self.input_enabled || !self.input.down || self.input.is_drag() {
            return false;
        }
        let Some((ox, oy)) = self.input.press_origin else { return false };
        self.pointer_in(r) && r.contains(ox, oy)
    }

    pub fn hovered(&self, r: Rect) -> bool {
        self.input_enabled && self.pointer_in(r)
    }

    /// 押しっぱなしで連続発火する（ステッパー用）。
    pub fn repeat_clicked(&mut self, id: Id, r: Rect) -> bool {
        if self.clicked(r) {
            let fired = self.mem.repeat_at.remove(&id).map(|t| t < 0.0).unwrap_or(false);
            return !fired;
        }
        if self.pressed(r) {
            self.mem.request_repaint();
            let now = self.input.time;
            match self.mem.repeat_at.get(&id).copied() {
                None => {
                    self.mem.repeat_at.insert(id, now + 0.45);
                }
                Some(t) if t >= 0.0 && now >= t => {
                    // 負の値 = 既にリピート発火した印（離したときのクリックを無効にする）
                    self.mem.repeat_at.insert(id, -(now + 0.08));
                    return true;
                }
                Some(t) if t < 0.0 && now >= -t => {
                    self.mem.repeat_at.insert(id, -(now + 0.08));
                    return true;
                }
                _ => {}
            }
        } else {
            self.mem.repeat_at.remove(&id);
        }
        false
    }

    // ------------------------------------------------------------------
    // アニメーション
    // ------------------------------------------------------------------

    /// `target` に向かって滑らかに近づく値。
    pub fn animate(&mut self, id: Id, target: f32, speed: f32) -> f32 {
        let dt = self.input.dt.clamp(0.0, 0.1);
        let v = self.mem.floats.entry(id).or_insert(target);
        let k = 1.0 - (-speed * dt).exp();
        *v += (target - *v) * k;
        if (target - *v).abs() < 0.001 {
            *v = target;
        } else {
            self.mem.repaint = true;
        }
        *v
    }

    // ------------------------------------------------------------------
    // スクロール
    // ------------------------------------------------------------------

    /// 縦スクロール領域。`viewport` の中に `f` で中身を描く。
    pub fn scroll_area(&mut self, id: Id, viewport: Rect, spacing: f32, f: impl FnOnce(&mut Ui)) {
        let mut st = self.mem.scrolls.get(&id).copied().unwrap_or_default();
        let max_off = (st.content - viewport.h).max(0.0);
        let dt = self.input.dt.clamp(0.0, 0.1);

        let origin_inside = self.input.press_origin.map(|(x, y)| viewport.contains(x, y)).unwrap_or(false);
        if self.input_enabled && self.input.down && origin_inside && self.input.is_drag() {
            let dy = self.input.delta.1;
            st.offset -= dy;
            st.velocity = if dt > 0.0 { -dy / dt } else { 0.0 };
            st.dragging = true;
            self.mem.repaint = true;
        } else {
            if st.dragging {
                st.dragging = false;
            }
            if st.velocity.abs() > 5.0 {
                st.offset += st.velocity * dt;
                st.velocity *= (1.0 - 4.0 * dt).max(0.0);
                self.mem.repaint = true;
            } else {
                st.velocity = 0.0;
            }
        }
        if self.input_enabled && self.pointer_in(viewport) && self.input.wheel.1 != 0.0 {
            st.offset -= self.input.wheel.1;
            st.velocity = 0.0;
            self.mem.repaint = true;
        }
        // 端での跳ね返り
        if !st.dragging {
            if st.offset < 0.0 {
                st.offset += (0.0 - st.offset) * (1.0 - (-18.0 * dt).exp());
                if st.offset > -0.5 {
                    st.offset = 0.0;
                }
                st.velocity = 0.0;
                self.mem.repaint = true;
            } else if st.offset > max_off {
                st.offset += (max_off - st.offset) * (1.0 - (-18.0 * dt).exp());
                if st.offset < max_off + 0.5 {
                    st.offset = max_off;
                }
                st.velocity = 0.0;
                self.mem.repaint = true;
            }
        }

        self.dl.push_clip(viewport);
        let start = viewport.y - st.offset;
        self.begin(viewport.x, start, viewport.w, spacing);
        f(self);
        let end = self.end();
        self.dl.pop_clip();

        st.content = end - start;
        st.viewport = viewport.h;
        // スクロールバー
        if st.content > viewport.h + 1.0 {
            let ratio = viewport.h / st.content;
            let bar_h = (viewport.h * ratio).max(24.0);
            let t = (st.offset / (st.content - viewport.h)).clamp(0.0, 1.0);
            let y = viewport.y + (viewport.h - bar_h) * t;
            self.dl.rect(Rect::new(viewport.right() - 4.0, y, 3.0, bar_h), self.theme.text_dim.with_alpha(0.35), 1.5);
        }
        self.mem.scrolls.insert(id, st);
    }

    // ------------------------------------------------------------------
    // テキスト
    // ------------------------------------------------------------------

    pub fn measure(&mut self, s: &str, size: f32, weight: FontWeight) -> f32 {
        self.text.measure(s, size, weight)
    }

    /// 幅 `w` に収まらなければ末尾を「…」で切る。
    pub fn fit_text(&mut self, s: &str, size: f32, weight: FontWeight, w: f32) -> String {
        if self.text.measure(s, size, weight) <= w {
            return s.to_string();
        }
        let ell = self.text.measure("…", size, weight);
        let mut out = String::new();
        let mut acc = 0.0;
        for ch in s.chars() {
            let a = self.text.advance(ch, size, weight);
            if acc + a + ell > w {
                break;
            }
            acc += a;
            out.push(ch);
        }
        out.push('…');
        out
    }

    /// 矩形の中に 1 行を描く（縦は中央揃え）。
    pub fn text_in(&mut self, r: Rect, s: &str, size: f32, weight: FontWeight, color: Color, align: Align) {
        let w = self.text.measure(s, size, weight);
        let lh = self.text.line_height(size);
        let x = match align {
            Align::Left => r.x,
            Align::Center => r.x + (r.w - w) * 0.5,
            Align::Right => r.right() - w,
        };
        let y = r.y + (r.h - lh) * 0.5;
        self.text.draw(self.dl, x, y, s, size, weight, color);
    }

    /// 現在の幅で折り返して描く。
    pub fn label_ex(&mut self, s: &str, size: f32, weight: FontWeight, color: Color, align: Align) -> Rect {
        let w = self.width();
        let lines = self.text.wrap(s, size, weight, w);
        let lh = self.text.line_height(size);
        let r = self.alloc(lh * lines.len() as f32);
        for (i, line) in lines.iter().enumerate() {
            let lr = Rect::new(r.x, r.y + lh * i as f32, r.w, lh);
            self.text_in(lr, line, size, weight, color, align);
        }
        r
    }

    pub fn label(&mut self, s: &str) -> Rect {
        let t = self.theme;
        self.label_ex(s, t.font_body, FontWeight::Regular, t.text, Align::Left)
    }

    pub fn heading(&mut self, s: &str) -> Rect {
        let t = self.theme;
        self.label_ex(s, t.font_heading, FontWeight::Bold, t.text, Align::Left)
    }

    pub fn caption(&mut self, s: &str) -> Rect {
        let t = self.theme;
        self.label_ex(s, t.font_small, FontWeight::Regular, t.text_dim, Align::Left)
    }
}
