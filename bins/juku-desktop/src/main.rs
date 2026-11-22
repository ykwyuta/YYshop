//! Windows（ネイティブ）版のエントリポイント。フォントは実行ファイルに埋め込む。
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use juku_app::GameApp;
use juku_platform::{Config, FontSource};

static REGULAR: &[u8] = include_bytes!("../../../assets/fonts/NotoSansJP-Regular.subset.otf");
static BOLD: &[u8] = include_bytes!("../../../assets/fonts/NotoSansJP-Bold.subset.otf");

fn main() {
    let config = Config {
        title: "合格の街 〜塾長シミュレーター〜".into(),
        size: (440.0, 900.0),
        fonts: FontSource::Embedded { regular: REGULAR, bold: BOLD },
        canvas_id: String::new(),
    };
    juku_platform::run(config, GameApp::new);
}
