//! あとで画像生成で差し替える画像の枠。
//!
//! 画像はまだ無いので、置き場所（ファイル名）を書いた仮の絵を描く。
//! 一覧と生成用のプロンプトは `docs/images/README.md` にまとめてある。

use juku_render::{Color, FontWeight, Rect};
use juku_ui::{Align, Ui};

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

/// 未生成の画像の代わりに描く仮の絵。斜線の地に、何の画像かとファイル名を書く。
pub fn placeholder(ui: &mut Ui, r: Rect, title: &str, file: &str, tint: Color) {
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
    let label = ui.fit_text(&format!("画像未生成：{file}"), t.font_small - 2.0, FontWeight::Regular, r.w - 16.0);
    ui.text_in(
        Rect::new(r.x + 8.0, cy + 22.0, r.w - 16.0, 16.0),
        &label,
        t.font_small - 2.0,
        FontWeight::Regular,
        Color::WHITE.with_alpha(0.5),
        Align::Center,
    );
}
