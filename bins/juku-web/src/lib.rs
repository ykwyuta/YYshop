//! WebAssembly 版のエントリポイント。フォントは別ファイルとして取得する（並行ダウンロード・長期キャッシュ）。

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// フォントなどのアセットのバージョン（tools/build-web.sh が内容ハッシュを渡す）。
#[cfg(target_arch = "wasm32")]
const ASSET_VERSION: &str = match option_env!("YYSHOP_ASSET_VERSION") {
    Some(v) => v,
    None => "dev",
};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    let config = juku_platform::Config {
        title: "合格の街 〜塾長シミュレーター〜".into(),
        size: (440.0, 900.0),
        fonts: juku_platform::FontSource::Url {
            regular: format!("assets/fonts/NotoSansJP-Regular.subset.otf?v={ASSET_VERSION}"),
            bold: format!("assets/fonts/NotoSansJP-Bold.subset.otf?v={ASSET_VERSION}"),
        },
        canvas_id: "app".into(),
        icon_png: None,
    };
    juku_platform::run(config, juku_app::GameApp::new);
}
