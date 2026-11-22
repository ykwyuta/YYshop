//! タウンマップ：街の地図の上でエリアを選び、教室を開く。
//!
//! 地図の座標は画面側の都合なので、エリアの名前からここで決める（シミュレーションには持たせない）。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::World;
use juku_ui::{Align, ButtonStyle, Ui};

use crate::fmt::format_yen;
use crate::screens::hq::HqAction;

/// エリアの地図上の位置（0..1）。北が上、海が南東。
fn position(name: &str, index: usize, count: usize) -> (f32, f32) {
    match name {
        // 北部
        "北野台" => (0.16, 0.14),
        "青葉ヶ丘" => (0.43, 0.11),
        "みなと北" => (0.70, 0.17),
        // 市内
        "けやき台" => (0.30, 0.46),
        "桜ヶ丘" => (0.57, 0.40),
        "緑町" => (0.13, 0.66),
        "港南" => (0.40, 0.73),
        "東雲" => (0.64, 0.80),
        // 都心
        "学園都市" => (0.87, 0.36),
        "中央駅前" => (0.84, 0.60),
        // 知らないエリアは円周上に並べる
        _ => {
            let a = index as f32 / count.max(1) as f32 * std::f32::consts::TAU;
            (0.5 + 0.38 * a.sin(), 0.5 - 0.38 * a.cos())
        }
    }
}

/// 線路（駅どうしを結ぶ）。
const RAILS: &[(&str, &str)] = &[
    ("北野台", "けやき台"),
    ("けやき台", "桜ヶ丘"),
    ("桜ヶ丘", "中央駅前"),
    ("みなと北", "桜ヶ丘"),
    ("青葉ヶ丘", "みなと北"),
    ("緑町", "港南"),
    ("港南", "東雲"),
    ("東雲", "中央駅前"),
    ("学園都市", "中央駅前"),
];

/// エリアの状態。
#[derive(Clone, Copy, PartialEq)]
enum State {
    /// 自塾の教室がある（教室の添字）
    Ours(usize),
    /// 開校できる
    Open,
    /// まだ出店できない
    Locked,
}

fn state(world: &World, a: usize) -> State {
    if let Some(ci) = world.classrooms.iter().position(|c| c.area == a) {
        State::Ours(ci)
    } else if world.area_unlocked(a) {
        State::Open
    } else {
        State::Locked
    }
}

/// 地図と、選んだエリアの詳細を描く。
pub fn draw(ui: &mut Ui, world: &World, selected: &mut Option<usize>, action: &mut Option<HqAction>) {
    let t = ui.theme;
    let n = world.areas.len();
    if n == 0 {
        return;
    }
    // 最初は開校できるエリアのうち市場の大きいところ、なければ創業の地を選んでおく
    let sel = selected
        .filter(|a| *a < n)
        .or_else(|| world.open_areas().into_iter().max_by(|a, b| world.areas[*a].pool.total_cmp(&world.areas[*b].pool)))
        .unwrap_or(world.classrooms[0].area);
    *selected = Some(sel);

    ui.heading("タウンマップ");
    ui.caption("エリアをタップすると、市場の大きさ・家賃・競合が見られます。開校できるエリアは白い輪、自塾の教室はオレンジです。");

    let mut clicked = None;
    ui.card(12.0, |ui| {
        let width = ui.alloc(0.0).w;
        let m = ui.alloc(width.clamp(280.0, 460.0));
        let pt = |(x, y): (f32, f32)| (m.x + x * m.w, m.y + y * m.h);

        // ---- 地面・海・川 ----
        ui.dl.rect_gradient(m, Color::hex(0x2E4A3B), Color::hex(0x243B30), 10.0);
        // 南東の海
        let sea = Color::hex(0x1D3C55);
        ui.dl.rect(Rect::new(m.x, m.y + m.h * 0.90, m.w, m.h * 0.10), sea, 10.0);
        ui.dl.rect(Rect::new(m.x + m.w * 0.52, m.y + m.h * 0.86, m.w * 0.48, m.h * 0.14), sea, 10.0);
        ui.dl.rect(Rect::new(m.x + m.w * 0.76, m.y + m.h * 0.70, m.w * 0.24, m.h * 0.30), sea, 10.0);
        // 北部と市内を分ける川
        let river = Color::hex(0x3B6E8C);
        let mut prev = pt((0.0, 0.30));
        for k in 1..=16 {
            let x = k as f32 / 16.0;
            let y = 0.28 + 0.035 * (x * 9.0).sin();
            let p = pt((x, y));
            ui.dl.line(prev.0, prev.1, p.0, p.1, 6.0, river);
            prev = p;
        }
        // 都心の街区（格子）
        let grid = Color::WHITE.with_alpha(0.05);
        for k in 0..6 {
            let x = m.x + m.w * (0.74 + k as f32 * 0.045);
            ui.dl.line(x, m.y + m.h * 0.24, x, m.y + m.h * 0.70, 1.0, grid);
            let y = m.y + m.h * (0.26 + k as f32 * 0.08);
            ui.dl.line(m.x + m.w * 0.72, y, m.right() - 6.0, y, 1.0, grid);
        }
        // 地方の名前
        let region_label = |ui: &mut Ui, x: f32, y: f32, s: &str| {
            ui.text_in(
                Rect::new(m.x + m.w * x - 40.0, m.y + m.h * y, 80.0, 18.0),
                s,
                t.font_small,
                FontWeight::Bold,
                Color::WHITE.with_alpha(0.28),
                Align::Center,
            );
        };
        region_label(ui, 0.10, 0.015, "北部");
        region_label(ui, 0.10, 0.33, "市内");
        region_label(ui, 0.86, 0.17, "都心");
        region_label(ui, 0.90, 0.92, "海");

        // ---- 線路 ----
        let pos: Vec<(f32, f32)> = (0..n).map(|a| pt(position(&world.areas[a].name, a, n))).collect();
        let find = |name: &str| world.areas.iter().position(|a| a.name == name);
        for (a, b) in RAILS {
            if let (Some(a), Some(b)) = (find(a), find(b)) {
                let (p, q) = (pos[a], pos[b]);
                ui.dl.line(p.0, p.1, q.0, q.1, 4.0, Color::hex(0x101A15).with_alpha(0.7));
                ui.dl.line(p.0, p.1, q.0, q.1, 1.5, Color::hex(0xC9C2A8).with_alpha(0.55));
            }
        }

        // ---- エリア ----
        let time = ui.input.time as f32;
        for a in 0..n {
            let area = &world.areas[a];
            let (cx, cy) = pos[a];
            let st = state(world, a);
            let r = 11.0 + (area.pool as f32 - 0.6).clamp(0.0, 1.4) * 8.0;
            let is_sel = a == sel;

            if is_sel {
                let pulse = 0.5 + 0.5 * (time * 3.0).sin();
                ui.dl.circle(cx, cy, r + 9.0 + pulse * 2.0, t.star.with_alpha(0.25));
                ui.mem.request_repaint();
            }
            match st {
                State::Ours(_) => {
                    ui.dl.circle(cx, cy, r + 2.0, Color::hex(0x101A15));
                    ui.dl.circle(cx, cy, r, t.accent);
                    ui.text_in(
                        Rect::new(cx - r, cy - r, r * 2.0, r * 2.0),
                        "塾",
                        (r * 1.1).min(18.0),
                        FontWeight::Bold,
                        Color::hex(0x1A1208),
                        Align::Center,
                    );
                }
                State::Open => {
                    ui.dl.circle(cx, cy, r + 2.0, Color::WHITE.with_alpha(0.9));
                    ui.dl.circle(cx, cy, r - 1.0, Color::hex(0x1F3329));
                    ui.text_in(
                        Rect::new(cx - r, cy - r, r * 2.0, r * 2.0),
                        "＋",
                        (r * 1.0).min(16.0),
                        FontWeight::Bold,
                        Color::WHITE.with_alpha(0.9),
                        Align::Center,
                    );
                }
                State::Locked => {
                    ui.dl.circle(cx, cy, r, Color::hex(0x3A4540).with_alpha(0.85));
                    padlock(ui, cx, cy, r * 0.5, Color::WHITE.with_alpha(0.5));
                }
            }

            // 競合の教室（小さな点。売りに出ている塾は赤い縁）
            let rivals = world.competitors.iter().filter(|c| !c.acquired && c.branch(a).is_some());
            for (k, comp) in rivals.enumerate() {
                let ang = -0.9 + k as f32 * 0.55;
                let (dx, dy) = ((r + 7.0) * ang.cos(), (r + 7.0) * ang.sin());
                let c = Color::rgb(comp.color[0], comp.color[1], comp.color[2]);
                if comp.for_sale {
                    ui.dl.circle(cx + dx, cy + dy, 6.0, t.negative);
                }
                ui.dl.circle(cx + dx, cy + dy, 4.5, c);
            }

            // 名前
            let name_c = if st == State::Locked { Color::WHITE.with_alpha(0.45) } else { Color::WHITE };
            let label_r = Rect::new(cx - 50.0, cy + r + 3.0, 100.0, 16.0);
            let tw = ui.measure(&area.name, 11.0, FontWeight::Bold) + 10.0;
            ui.dl.rect(Rect::new(cx - tw * 0.5, label_r.y, tw, 16.0), Color::BLACK.with_alpha(0.45), 8.0);
            ui.text_in(label_r, &area.name, 11.0, FontWeight::Bold, name_c, Align::Center);

            // タップの当たり判定（名前まで含める）
            let hit = Rect::new(cx - r - 10.0, cy - r - 10.0, (r + 10.0) * 2.0, (r + 10.0) * 2.0 + 12.0);
            if ui.clicked(hit) {
                clicked = Some(a);
            }
        }

        // 凡例
        let r = ui.alloc(20.0);
        let items: [(Color, &str); 4] = [
            (t.accent, "自塾"),
            (Color::WHITE, "開校できる"),
            (Color::hex(0x5A6560), "条件あり"),
            (t.negative, "売却中の塾"),
        ];
        let mut x = r.x;
        for (c, label) in items {
            ui.dl.circle(x + 6.0, r.y + 10.0, 5.0, c);
            ui.text_in(
                Rect::new(x + 15.0, r.y, 100.0, 20.0),
                label,
                t.font_small - 1.0,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
            x += 22.0 + ui.measure(label, t.font_small - 1.0, FontWeight::Regular) + 8.0;
        }
    });
    if let Some(a) = clicked {
        *selected = Some(a);
    }
    let sel = selected.unwrap_or(sel);
    detail(ui, world, sel, action);
}

/// 錠前の形（鍵のかかったエリア）。
fn padlock(ui: &mut Ui, cx: f32, cy: f32, s: f32, c: Color) {
    ui.dl.arc(cx, cy - s * 0.25, s * 0.62, s * 0.38, -std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2, c);
    ui.dl.rect(Rect::new(cx - s * 0.75, cy - s * 0.25, s * 1.5, s * 1.15), c, s * 0.2);
}

/// 選んだエリアの詳細カード。
fn detail(ui: &mut Ui, world: &World, a: usize, action: &mut Option<HqAction>) {
    let t = ui.theme;
    let area = &world.areas[a];
    let st = state(world, a);
    let shares = world.area_shares(a);
    let total: u32 = shares.iter().map(|s| s.1).sum();
    ui.card(16.0, |ui| {
        ui.set_spacing(6.0);
        let r = ui.alloc(26.0);
        let title = format!("{}エリア", area.name);
        ui.text_in(r, &title, t.font_heading, FontWeight::Bold, t.text, Align::Left);
        let w = ui.measure(&title, t.font_heading, FontWeight::Bold);
        let (badge, color) = match st {
            State::Ours(_) => ("自塾あり", t.accent),
            State::Open => ("開校できる", t.positive),
            State::Locked => ("条件あり", t.text_dim),
        };
        let bw = ui.badge_at(r.x + w + 8.0, r.y + 2.0, badge, color);
        ui.badge_at(r.x + w + 8.0 + bw + 6.0, r.y + 2.0, &area.region, t.series[4]);
        ui.label_ex(&area.description, t.font_small, FontWeight::Regular, t.text_dim, Align::Left);

        let growth = if area.pool_growth > 0.0 {
            format!("（毎年 +{:.0}%）", area.pool_growth * 100.0)
        } else if area.pool_growth < 0.0 {
            format!("（毎年 {:.0}%）", area.pool_growth * 100.0)
        } else {
            String::new()
        };
        ui.kv_row("市場規模", &format!("×{:.2}{growth}", area.pool), Some(t.star));
        ui.kv_row("家賃", &format!("{}/週", format_yen(area.rent)), None);
        ui.kv_row("開校費用", &format_yen(area.open_cost), None);
        ui.kv_row("エリアの通塾生", &format!("{total}人"), None);

        // 塾ごとのシェア（横棒）
        if total > 0 {
            let mut sorted = shares.clone();
            sorted.sort_by_key(|s| std::cmp::Reverse(s.1));
            for (name, students, rgb, ours) in sorted.iter().take(6) {
                let r = ui.alloc(20.0);
                let c = Color::rgb(rgb[0], rgb[1], rgb[2]);
                let label_w = (r.w * 0.42).min(150.0);
                let weight = if *ours { FontWeight::Bold } else { FontWeight::Regular };
                let label = ui.fit_text(name, t.font_small, weight, label_w - 4.0);
                ui.text_in(Rect::new(r.x, r.y, label_w, r.h), &label, t.font_small, weight, t.text, Align::Left);
                let bar_w = r.w - label_w - 52.0;
                let frac = *students as f32 / total as f32;
                ui.dl.rect(Rect::new(r.x + label_w, r.y + 5.0, bar_w, 10.0), Color::WHITE.with_alpha(0.06), 5.0);
                ui.dl.rect(Rect::new(r.x + label_w, r.y + 5.0, (bar_w * frac).max(4.0), 10.0), c, 5.0);
                ui.text_in(r, &format!("{students}人"), t.font_small, FontWeight::Regular, t.text_dim, Align::Right);
            }
        } else {
            ui.label_ex(
                "このエリアにはまだ塾がありません。",
                t.font_small,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
        }
        for comp in world.competitors.iter().filter(|c| c.for_sale && !c.acquired && c.branch(a).is_some()) {
            ui.label_ex(
                &format!("{}が売りに出ています（下の「塾の買収」から買えます）", comp.name),
                t.font_small,
                FontWeight::Bold,
                t.negative,
                Align::Left,
            );
        }

        match st {
            State::Ours(ci) => {
                let c = &world.classrooms[ci];
                ui.label_ex(
                    &format!("{}：在籍 {} 人・講師 {} 人", c.name, c.students.len(), c.teachers.len()),
                    t.font_small,
                    FontWeight::Bold,
                    t.accent,
                    Align::Left,
                );
                if ui.button("この教室を見る", ButtonStyle::Secondary) {
                    *action = Some(HqAction::Select(ci));
                }
            }
            State::Open => {
                ui.caption(
                    "2 教室目からは本部の運営費（週 ¥60,000／教室）がかかります。新しい教室は生徒ゼロから始まります。",
                );
                let affordable = world.cash >= area.open_cost;
                let label = format!("ここに開校する（{}）", format_yen(area.open_cost));
                let style = if affordable { ButtonStyle::Primary } else { ButtonStyle::Ghost };
                if ui.button(&label, style) {
                    *action = Some(if affordable {
                        HqAction::Open(a)
                    } else {
                        HqAction::Toast(format!("資金が足りません（あと {}）", format_yen(area.open_cost - world.cash)))
                    });
                }
            }
            State::Locked => {
                ui.label_ex(
                    &format!("出店の条件：{}", area.unlock.label()),
                    t.font_body,
                    FontWeight::Bold,
                    t.accent,
                    Align::Left,
                );
            }
        }
    });
}
