//! WebAssembly 版のエントリポイント。フォントは別ファイルとして取得する（並行ダウンロード・長期キャッシュ）。

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    let base = env!("CARGO_PKG_VERSION");
    let _ = base;
    let config = juku_platform::Config {
        title: "合格の街 〜塾長シミュレーター〜".into(),
        size: (440.0, 900.0),
        fonts: juku_platform::FontSource::Url {
            regular: "assets/fonts/NotoSansJP-Regular.subset.otf".into(),
            bold: "assets/fonts/NotoSansJP-Bold.subset.otf".into(),
        },
        canvas_id: "app".into(),
    };
    juku_platform::run(config, juku_app::GameApp::new);
}
