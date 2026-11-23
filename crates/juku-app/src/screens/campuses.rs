//! 校舎一覧：経営している校舎を、校舎の絵と主な数字で並べる。タップでその校舎に切り替える。
//!
//! ヘッダーの校舎名から開く画面と、本部タブの「経営」の両方で同じカードを使う。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::World;
use juku_ui::{Align, ButtonStyle, ImageState, Ui};

use crate::fmt::format_yen;
use crate::images;
use crate::portraits::{manager_expression, Portrait};

/// 校舎一覧で起きたこと。
pub enum CampusAction {
    /// この校舎に切り替えて、教室タブを開く
    Select(usize),
    /// 新しい校舎を開く（本部の首都圏攻略マップへ）
    OpenNew,
    /// 一覧を閉じる
    Close,
}

pub const SORTS: [&str; 4] = ["開校順", "生徒数", "利益", "評価"];

fn net_profit(world: &World, ci: usize) -> i64 {
    world.classrooms[ci].last_report().map(|r| r.pnl.net_profit()).unwrap_or(0)
}

/// 並べ替えた校舎の添字。
pub fn sorted(world: &World, sort: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..world.classrooms.len()).collect();
    match sort {
        1 => order.sort_by_key(|&ci| std::cmp::Reverse(world.classrooms[ci].students.len())),
        2 => order.sort_by_key(|&ci| std::cmp::Reverse(net_profit(world, ci))),
        3 => order
            .sort_by(|&a, &b| world.classrooms[b].ratings.overall().total_cmp(&world.classrooms[a].ratings.overall())),
        _ => {}
    }
    order
}

/// 校舎一覧の画面。`current` はいま表示している校舎、`sort` は並べ替え。
pub fn draw(ui: &mut Ui, world: &World, current: usize, sort: &mut usize) -> Option<CampusAction> {
    let t = ui.theme;
    let mut action = None;

    // 見出しと閉じるボタン
    let r = ui.alloc(32.0);
    ui.text_in(
        r,
        &format!("校舎一覧（{} 校舎）", world.classrooms.len()),
        t.font_heading + 2.0,
        FontWeight::Bold,
        t.text,
        Align::Left,
    );
    let close = Rect::new(r.right() - 84.0, r.y, 84.0, 32.0);
    if ui.button_at(close, "閉じる", ButtonStyle::Ghost) {
        action = Some(CampusAction::Close);
    }
    ui.caption(
        "見たい校舎をタップすると、その校舎に切り替わります。教室・コース・集客・経理のタブは、選んだ校舎の画面です。",
    );

    summary(ui, world);

    ui.label_ex("並べ替え", t.font_small, FontWeight::Bold, t.text_dim, Align::Left);
    if let Some(i) = ui.segmented(&SORTS, *sort) {
        *sort = i;
    }
    for ci in sorted(world, *sort) {
        if card(ui, world, ci, ci == current) {
            action = Some(CampusAction::Select(ci));
        }
    }
    if ui.button("＋ 新しい校舎を開く（首都圏攻略マップへ）", ButtonStyle::Secondary) {
        action = Some(CampusAction::OpenNew);
    }
    action
}

/// 会社全体の要約（校舎数・生徒・利益・制覇）。
fn summary(ui: &mut Ui, world: &World) {
    let t = ui.theme;
    let n = world.classrooms.len();
    let net: i64 = (0..n).map(|ci| net_profit(world, ci)).sum();
    let (won, total) = world.conquest_progress();
    ui.card(12.0, |ui| {
        let r = ui.alloc(46.0);
        let items = [
            ("校舎", format!("{n}"), t.text),
            ("生徒", format!("{}人", world.total_students()), t.text),
            ("純利益/週", format_yen(net), if net >= 0 { t.accent } else { t.negative }),
            ("制覇", format!("{won}/{total}街"), t.star),
        ];
        let w = r.w / items.len() as f32;
        for (k, (label, value, color)) in items.iter().enumerate() {
            let x = r.x + w * k as f32;
            ui.text_in(Rect::new(x, r.y, w, 18.0), label, t.font_small, FontWeight::Regular, t.text_dim, Align::Center);
            let v = ui.fit_text(value, t.font_body + 1.0, FontWeight::Bold, w - 4.0);
            ui.text_in(
                Rect::new(x, r.y + 20.0, w, 24.0),
                &v,
                t.font_body + 1.0,
                FontWeight::Bold,
                *color,
                Align::Center,
            );
        }
    });
}

/// 校舎 1 つ分のカード。タップされたら true。
pub fn card(ui: &mut Ui, world: &World, ci: usize, is_current: bool) -> bool {
    let t = ui.theme;
    let c = &world.classrooms[ci];
    let area = &world.areas[c.area];
    let width = ui.width();
    // 左に校舎の絵、右に数字。スマホの幅でも 1 画面に 4 校舎ほど並ぶ大きさにする
    let img_w = (width * 0.36).clamp(120.0, 240.0);
    let h = 132.0f32.max(img_w * 9.0 / 16.0 + 20.0);
    let r = ui.alloc(h);
    // 背景（押すと沈む。いま表示中の校舎はオレンジの縁）
    let pressed = ui.pressed(r);
    let fill = if pressed { t.card.lerp(Color::WHITE, 0.06) } else { t.card };
    let (bw, bc) = if is_current { (2.0, t.accent) } else { (1.0, t.card_border) };
    ui.dl.rect_border(r, fill, 14.0, bw, bc);

    let pad = 10.0;
    let img = Rect::new(r.x + pad, r.y + pad, img_w - pad, h - pad * 2.0);
    picture(ui, img, &area.name);
    if is_current {
        let label = "表示中";
        let w = ui.measure(label, t.font_small - 1.0, FontWeight::Bold) + 16.0;
        let b = Rect::new(img.x + 8.0, img.y + 8.0, w, 20.0);
        ui.dl.rect(b, t.accent, 10.0);
        ui.text_in(b, label, t.font_small - 1.0, FontWeight::Bold, Color::hex(0x1A1208), Align::Center);
    }

    // 情報
    let info = Rect::new(img.right() + 12.0, r.y + pad, r.right() - img.right() - 12.0 - pad - 2.0, h - pad * 2.0);
    let mut y = info.y;
    // 1 行目：校舎名と純利益
    ui.text_in(Rect::new(info.x, y, info.w, 24.0), &c.name, t.font_body + 2.0, FontWeight::Bold, t.text, Align::Left);
    let net = net_profit(world, ci);
    let nc = if net >= 0 { t.accent } else { t.negative };
    ui.text_in(
        Rect::new(info.x, y, info.w, 24.0),
        &format!("{}/週", format_yen(net)),
        t.font_body,
        FontWeight::Bold,
        nc,
        Align::Right,
    );
    y += 27.0;
    // 2 行目：地方・街・順位のバッジ
    let mut x = info.x;
    x += ui.badge_at(x, y, &area.region, t.series[4]) + 6.0;
    match world.area_standing(c.area) {
        _ if world.area_conquered(c.area) => {
            ui.badge_at(x, y, "制覇", t.star);
        }
        Some((share, rank)) => {
            let color = if rank <= 2 { t.positive } else { t.text_dim };
            let label = format!("地域 {rank} 位・{:.0}%", share * 100.0);
            if x + ui.measure(&label, t.font_small - 1.0, FontWeight::Bold) + 16.0 <= info.right() {
                ui.badge_at(x, y, &label, color);
            }
        }
        None => {}
    }
    y += 27.0;
    // 3 行目：在籍・評価
    ui.text_in(
        Rect::new(info.x, y, info.w, 22.0),
        &format!("在籍 {} 人", c.students.len()),
        t.font_body,
        FontWeight::Regular,
        t.text,
        Align::Left,
    );
    ui.stars_at(info.x + 92.0, y + 4.0, 13.0, c.ratings.overall());
    ui.text_in(
        Rect::new(info.x, y, info.w, 22.0),
        &format!("{:.1}", c.ratings.overall()),
        t.font_small,
        FontWeight::Regular,
        t.text_dim,
        Align::Right,
    );
    y += 25.0;
    // 4 行目：教室長・講師・委任
    let face = Rect::new(info.x, y, 28.0, 28.0);
    Portrait::manager(&c.manager, manager_expression(world, ci)).draw(ui, face);
    let delegated = c.manager.delegations.iter().filter(|d| **d).count();
    let owned = if c.owned { "・自社物件" } else { "" };
    let line = format!("{}・講師 {} 人・委任 {delegated}/4{owned}", c.manager.name, c.teachers.len());
    let tr = Rect::new(face.right() + 8.0, y + 3.0, info.right() - face.right() - 8.0, 22.0);
    let line = ui.fit_text(&line, t.font_small, FontWeight::Regular, tr.w);
    ui.text_in(tr, &line, t.font_small, FontWeight::Regular, t.text_dim, Align::Left);

    ui.clicked(r)
}

/// 校舎の絵。画面に映っているときだけ読み込む（一覧が長くても GPU のメモリを使いすぎない）。
fn picture(ui: &mut Ui, r: Rect, town: &str) {
    let t = ui.theme;
    let visible = ui.dl.current_clip().is_none_or(|clip| clip.intersect(&r).h > 0.0);
    if visible {
        if let ImageState::Ready(info) = ui.image(&images::school_file(town)) {
            ui.image_cover(r, info, 10.0);
            return;
        }
    }
    ui.dl.rect_gradient(r, t.accent.with_alpha(0.25), t.accent.with_alpha(0.08), 10.0);
}
