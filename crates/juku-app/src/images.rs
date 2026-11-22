//! 地図・街・校舎の画像。
//!
//! 絵は `tools/gen_images.py` がフラットなイラスト調で描く（一覧と描き方は `docs/images/README.md`）。
//! Windows 版は実行ファイルに埋め込み（隣に同じパスのファイルがあればそちらを優先）、Web 版は fetch する。

use juku_render::{Color, FontWeight, Rect};
use juku_ui::{Align, ImageState, Ui};

macro_rules! image_table {
    ($($path:literal),* $(,)?) => {
        /// ゲームが使う画像のパス。
        pub const ALL: &[&str] = &[$($path),*];

        /// 実行ファイルに埋め込んだ画像（Windows 版）。
        #[cfg(not(target_arch = "wasm32"))]
        pub fn embedded(path: &str) -> Option<&'static [u8]> {
            match path {
                $($path => Some(include_bytes!(concat!("../../../", $path))),)*
                _ => None,
            }
        }
    };
}

image_table!(
    "assets/images/map/shutoken_map.png",
    "assets/images/towns/town_kichijoji.png",
    "assets/images/towns/town_seijogakuenmae.png",
    "assets/images/towns/town_chofu.png",
    "assets/images/towns/town_fuchu.png",
    "assets/images/towns/town_futakotamagawa.png",
    "assets/images/towns/town_urawa.png",
    "assets/images/towns/town_omiya.png",
    "assets/images/towns/town_tokorozawa.png",
    "assets/images/towns/town_shibuya.png",
    "assets/images/towns/town_myogadani.png",
    "assets/images/towns/town_yokohama.png",
    "assets/images/towns/town_tamaplaza.png",
    "assets/images/towns/town_musashikosugi.png",
    "assets/images/towns/town_toyosu.png",
    "assets/images/towns/town_funabashi.png",
    "assets/images/towns/town_kashiwa.png",
    "assets/images/schools/school_kichijoji.png",
    "assets/images/schools/school_seijogakuenmae.png",
    "assets/images/schools/school_chofu.png",
    "assets/images/schools/school_fuchu.png",
    "assets/images/schools/school_futakotamagawa.png",
    "assets/images/schools/school_urawa.png",
    "assets/images/schools/school_omiya.png",
    "assets/images/schools/school_tokorozawa.png",
    "assets/images/schools/school_shibuya.png",
    "assets/images/schools/school_myogadani.png",
    "assets/images/schools/school_yokohama.png",
    "assets/images/schools/school_tamaplaza.png",
    "assets/images/schools/school_musashikosugi.png",
    "assets/images/schools/school_toyosu.png",
    "assets/images/schools/school_funabashi.png",
    "assets/images/schools/school_kashiwa.png",
);

/// 首都圏の地図の背景画像。
pub const MAP_FILE: &str = "assets/images/map/shutoken_map.png";

/// 街の名前 → ファイル名に使うローマ字。
pub fn slug(name: &str) -> &'static str {
    match name {
        "吉祥寺" => "kichijoji",
        "成城学園前" => "seijogakuenmae",
        "調布" => "chofu",
        "府中" => "fuchu",
        "二子玉川" => "futakotamagawa",
        "浦和" => "urawa",
        "大宮" => "omiya",
        "所沢" => "tokorozawa",
        "渋谷" => "shibuya",
        "茗荷谷" => "myogadani",
        "横浜" => "yokohama",
        "たまプラーザ" => "tamaplaza",
        "武蔵小杉" => "musashikosugi",
        "豊洲" => "toyosu",
        "船橋" => "funabashi",
        "柏" => "kashiwa",
        _ => "unknown",
    }
}

/// 街の風景（駅前）の画像。
pub fn town_file(name: &str) -> String {
    format!("assets/images/towns/town_{}.png", slug(name))
}

/// その街に開く自塾の校舎の画像。
pub fn school_file(name: &str) -> String {
    format!("assets/images/schools/school_{}.png", slug(name))
}

/// 画像を `r` いっぱいに描く。読み込み中・ファイルが無いときは、何の画像かを書いた代わりの絵を描く。
pub fn picture(ui: &mut Ui, r: Rect, title: &str, file: &str, tint: Color) {
    match ui.image(file) {
        ImageState::Ready(info) => {
            ui.image_cover(r, info, 10.0);
            ui.dl.rect_border(r, Color::TRANSPARENT, 10.0, 1.0, Color::WHITE.with_alpha(0.12));
        }
        ImageState::Loading => placeholder(ui, r, title, "読み込み中…", tint),
        ImageState::Missing => placeholder(ui, r, title, &format!("画像がありません：{file}"), tint),
    }
}

/// 画像の代わりの絵。斜線の地に、何の画像かと状態を書く。
fn placeholder(ui: &mut Ui, r: Rect, title: &str, note: &str, tint: Color) {
    let t = ui.theme;
    ui.dl.rect_gradient(r, tint.with_alpha(0.30), tint.with_alpha(0.12), 10.0);
    ui.dl.push_clip(r);
    let step = 14.0;
    let mut x = r.x - r.h;
    while x < r.right() {
        ui.dl.line(x, r.bottom(), x + r.h, r.y, 1.0, Color::WHITE.with_alpha(0.05));
        x += step;
    }
    ui.dl.pop_clip();
    ui.dl.rect_border(r, Color::TRANSPARENT, 10.0, 1.0, Color::WHITE.with_alpha(0.18));

    // 山と太陽（「ここに画像が入る」印）
    let (cx, cy) = r.center();
    let s = (r.h * 0.16).clamp(10.0, 26.0);
    let c = Color::WHITE.with_alpha(0.28);
    ui.dl.circle(cx + s * 0.9, cy - s * 1.3, s * 0.35, c);
    ui.dl.line(cx - s * 1.6, cy - s * 0.1, cx - s * 0.5, cy - s * 1.2, 3.0, c);
    ui.dl.line(cx - s * 0.5, cy - s * 1.2, cx + s * 0.4, cy - s * 0.3, 3.0, c);
    ui.dl.line(cx + s * 0.1, cy - s * 0.6, cx + s * 0.8, cy - s * 1.0, 3.0, c);
    ui.dl.line(cx + s * 0.8, cy - s * 1.0, cx + s * 1.6, cy - s * 0.1, 3.0, c);

    ui.text_in(
        Rect::new(r.x + 8.0, cy + 2.0, r.w - 16.0, 20.0),
        title,
        t.font_small,
        FontWeight::Bold,
        Color::WHITE.with_alpha(0.75),
        Align::Center,
    );
    let label = ui.fit_text(note, t.font_small - 2.0, FontWeight::Regular, r.w - 16.0);
    ui.text_in(
        Rect::new(r.x + 8.0, cy + 22.0, r.w - 16.0, 16.0),
        &label,
        t.font_small - 2.0,
        FontWeight::Regular,
        Color::WHITE.with_alpha(0.5),
        Align::Center,
    );
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn every_town_has_town_and_school_images() {
        for a in juku_sim::data::AREAS {
            assert_ne!(slug(a.name), "unknown", "{} のファイル名", a.name);
            assert!(ALL.contains(&town_file(a.name).as_str()), "{} の街の画像", a.name);
            assert!(ALL.contains(&school_file(a.name).as_str()), "{} の校舎の画像", a.name);
        }
        assert!(ALL.contains(&MAP_FILE));
    }

    #[test]
    fn embedded_images_decode() {
        for path in ALL {
            let bytes = embedded(path).expect(path);
            let img = juku_render::decode_png(bytes).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert!(img.width >= 640 && img.height >= 360, "{path}");
        }
    }
}
