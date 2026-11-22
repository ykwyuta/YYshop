//! Windows（ネイティブ）版のエントリポイント。フォントは実行ファイルに埋め込む。
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use juku_app::GameApp;
use juku_platform::{Config, FontSource, ImageSource};

static REGULAR: &[u8] = include_bytes!("../../../assets/fonts/NotoSansJP-Regular.subset.otf");
static BOLD: &[u8] = include_bytes!("../../../assets/fonts/NotoSansJP-Bold.subset.otf");
static ICON: &[u8] = include_bytes!("../../../assets/icon/icon-256.png");

fn main() {
    let config = Config {
        title: "合格の街 〜塾長シミュレーター〜".into(),
        size: (440.0, 900.0),
        fonts: FontSource::Embedded { regular: REGULAR, bold: BOLD },
        canvas_id: String::new(),
        icon_png: Some(ICON),
        images: ImageSource::Embedded(juku_app::images::embedded),
    };
    juku_platform::run(config, GameApp::new);
}
