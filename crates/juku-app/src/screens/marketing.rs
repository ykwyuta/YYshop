//! 集客タブ：地域シェア・認知度・集客施策。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::{CostModel, World};
use juku_ui::{Align, Id, Ui};

use crate::fmt::{format_yen, pct};

pub fn draw(ui: &mut Ui, world: &mut World, ci: usize) {
    share(ui, world, ci);
    rivals(ui, world, ci);
    brand(ui, world, ci);
    campaigns(ui, world, ci);
}

fn share(ui: &mut Ui, world: &World, ci: usize) {
    let t = ui.theme;
    let room = &world.classrooms[ci];
    let area = &world.areas[room.area];
    ui.heading(&format!("{}エリアの生徒シェア", area.name));
    let shares = world.area_shares(room.area);
    let segs: Vec<(f32, Color)> = shares
        .iter()
        .map(|(_, n, col, ours)| (*n as f32, if *ours { t.accent } else { Color::rgb(col[0], col[1], col[2]) }))
        .collect();
    ui.card(16.0, |ui| {
        let rows = shares.len().max(4) as f32;
        let r = ui.alloc((rows * 46.0).max(200.0));
        let cx = r.x + 92.0;
        let cy = r.y + r.h * 0.5;
        ui.donut(cx, cy, 90.0, 54.0, &segs, true);
        let x = r.x + 200.0;
        let w = r.right() - x;
        for (i, (name, n, col, ours)) in shares.iter().enumerate() {
            let color = if *ours { t.accent } else { Color::rgb(col[0], col[1], col[2]) };
            let sub = if *ours {
                format!("自塾・{n}人")
            } else {
                let style = world.competitors.iter().find(|c| &c.name == name).map(|c| c.style.label()).unwrap_or("");
                format!("{style}・{n}人")
            };
            let y = r.y + 4.0 + i as f32 * 46.0;
            ui.dl.circle(x + 8.0, y + 14.0, 7.0, color);
            let name = ui.fit_text(name, t.font_body, FontWeight::Bold, w - 22.0);
            let sub = ui.fit_text(&sub, t.font_small, FontWeight::Regular, w - 22.0);
            ui.text_in(
                Rect::new(x + 22.0, y, w - 22.0, 22.0),
                &name,
                t.font_body,
                FontWeight::Bold,
                color,
                Align::Left,
            );
            ui.text_in(
                Rect::new(x + 22.0, y + 20.0, w - 22.0, 18.0),
                &sub,
                t.font_small,
                FontWeight::Regular,
                color.lerp(t.text_dim, 0.4),
                Align::Left,
            );
        }
    });
}

/// 競合の動き（このエリアのものを優先して表示）。
fn rivals(ui: &mut Ui, world: &World, ci: usize) {
    let t = ui.theme;
    let area = world.classrooms[ci].area;
    ui.heading("競合の動き");
    let local: Vec<_> = world.rival_log.iter().filter(|m| m.area == area).take(5).collect();
    let others: Vec<_> = world
        .rival_log
        .iter()
        .filter(|m| m.area != area && world.classrooms.iter().any(|c| c.area == m.area))
        .take(3)
        .collect();
    if local.is_empty() && others.is_empty() {
        ui.caption("競合塾は 4 週ごとに状況を見て、値下げ・広告・出店・講師の引き抜きなどを仕掛けてきます。");
        return;
    }
    ui.card(14.0, |ui| {
        ui.set_spacing(8.0);
        for m in local.iter().chain(others.iter()) {
            let comp = &world.competitors[m.competitor];
            let color = Color::rgb(comp.color[0], comp.color[1], comp.color[2]);
            let r = ui.alloc(18.0);
            ui.dl.circle(r.x + 5.0, r.y + 9.0, 5.0, color);
            ui.text_in(
                Rect::new(r.x + 16.0, r.y, r.w, 18.0),
                &m.date.label(),
                t.font_small,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
            ui.label(&m.text);
        }
    });
}

fn brand(ui: &mut Ui, world: &World, ci: usize) {
    let t = ui.theme;
    let awareness = world.classrooms[ci].awareness;
    ui.card(16.0, |ui| {
        ui.set_spacing(8.0);
        for (label, v, c, note) in [
            (
                "ブランド力（会社全体の合格実績）",
                world.brand,
                t.star,
                "毎年 2 月の入試結果で決まり、全教室の集客に効きます",
            ),
            ("この教室の地域での認知度", awareness, t.series[4], "広告や説明会で上がり、何もしないと下がります"),
        ] {
            let r = ui.alloc(22.0);
            ui.text_in(r, label, t.font_body, FontWeight::Bold, t.text, Align::Left);
            ui.text_in(r, &pct(v), t.font_body, FontWeight::Bold, c, Align::Right);
            let r = ui.alloc(14.0);
            ui.meter_at(r, v as f32, c);
            ui.caption(note);
        }
    });
}

fn campaigns(ui: &mut Ui, world: &mut World, ci: usize) {
    let t = ui.theme;
    ui.heading("集客・サービス施策");
    let n = world.classrooms[ci].campaigns.len();
    let mut i = 0;
    while i < n {
        let pair = [i, i + 1];
        ui.columns(2, 12.0, |ui, col| {
            let idx = pair[col];
            if idx >= n {
                return;
            }
            let c = world.classrooms[ci].campaigns[idx].clone();
            ui.card(14.0, |ui| {
                ui.set_spacing(8.0);
                let r = ui.alloc(64.0);
                campaign_icon(ui, idx, r, if c.active { t.accent } else { t.text_dim });
                ui.label_ex(&c.name, t.font_heading, FontWeight::Bold, t.text, Align::Center);
                ui.label_ex(&c.description, t.font_small, FontWeight::Regular, t.text_dim, Align::Left);
                let (cost, unit) = match c.kind.cost_model() {
                    CostModel::Weekly(y) => (format_yen(y), "/ 週"),
                    CostModel::RevenueRate(r) => (format!("{:.0}%", r * 100.0), "授業料に対して"),
                    CostModel::PerTrial(y) => (format_yen(y), "体験生 1 人あたり"),
                };
                let r = ui.alloc(26.0);
                ui.text_in(r, &cost, t.font_heading + 2.0, FontWeight::Bold, t.text, Align::Right);
                ui.label_ex(unit, t.font_small, FontWeight::Regular, t.text_dim, Align::Right);
                let r = ui.alloc(40.0);
                if let Some(v) = ui.toggle_at(Id::new("campaign").with(idx as u64), r, c.active) {
                    world.classroom_mut(ci).set_campaign(idx, v);
                }
                let status = if c.active { "実施中" } else { "未実施" };
                let sc = if c.active { t.accent } else { t.text_dim };
                ui.text_in(r, status, t.font_body, FontWeight::Bold, sc, Align::Right);
            });
        });
        i += 2;
    }
}

/// 施策ごとの簡単な図柄。
fn campaign_icon(ui: &mut Ui, idx: usize, r: Rect, c: Color) {
    let (cx, cy) = r.center();
    let dl = &mut *ui.dl;
    match idx {
        // 無料体験：黒板と「体験」
        0 => {
            dl.rect_border(
                Rect::new(cx - 34.0, cy - 22.0, 68.0, 44.0),
                Color::hex(0x23402F),
                4.0,
                3.0,
                Color::hex(0x8A6A4A),
            );
            dl.line(cx - 20.0, cy - 6.0, cx + 20.0, cy - 6.0, 2.0, Color::WHITE.with_alpha(0.8));
            dl.line(cx - 20.0, cy + 4.0, cx + 8.0, cy + 4.0, 2.0, Color::WHITE.with_alpha(0.8));
            dl.circle(cx + 22.0, cy + 12.0, 6.0, c);
        }
        // チラシ
        1 => {
            for k in 0..3 {
                let o = k as f32 * 6.0;
                dl.rect_border(
                    Rect::new(cx - 26.0 + o, cy - 24.0 + o, 40.0, 44.0),
                    Color::hex(0xF4F1E8),
                    3.0,
                    1.0,
                    Color::hex(0xC9C2B0),
                );
            }
            dl.rect(Rect::new(cx - 10.0, cy - 6.0, 28.0, 8.0), c, 2.0);
            dl.line(cx - 8.0, cy + 8.0, cx + 16.0, cy + 8.0, 2.0, Color::hex(0x8A8A8A));
        }
        // 説明会：演台と人
        2 => {
            for k in 0..3 {
                let x = cx - 20.0 + k as f32 * 20.0;
                dl.circle(x, cy + 2.0, 6.0, Color::hex(0xF1C9A5));
                dl.rect(Rect::new(x - 8.0, cy + 9.0, 16.0, 14.0), c.with_alpha(0.7), 6.0);
            }
            dl.rect(Rect::new(cx - 30.0, cy - 24.0, 60.0, 14.0), c, 4.0);
        }
        // 自習室：机とランプ
        3 => {
            dl.rect(Rect::new(cx - 32.0, cy + 4.0, 64.0, 6.0), Color::hex(0x8A6A4A), 2.0);
            dl.rect(Rect::new(cx - 28.0, cy + 10.0, 4.0, 14.0), Color::hex(0x8A6A4A), 1.0);
            dl.rect(Rect::new(cx + 24.0, cy + 10.0, 4.0, 14.0), Color::hex(0x8A6A4A), 1.0);
            dl.rect(Rect::new(cx - 16.0, cy - 6.0, 26.0, 10.0), Color::hex(0xF4F1E8), 2.0);
            dl.line(cx + 18.0, cy + 4.0, cx + 24.0, cy - 16.0, 3.0, Color::hex(0x9DB0A6));
            dl.arc(cx + 24.0, cy - 16.0, 10.0, 0.0, 4.7, 7.9, c);
        }
        // 兄弟割：大小 2 人
        4 => {
            dl.circle(cx - 12.0, cy - 8.0, 9.0, Color::hex(0xF1C9A5));
            dl.rect(Rect::new(cx - 24.0, cy + 2.0, 24.0, 22.0), c, 10.0);
            dl.circle(cx + 14.0, cy, 7.0, Color::hex(0xE0AC85));
            dl.rect(Rect::new(cx + 5.0, cy + 8.0, 18.0, 16.0), c.lerp(Color::WHITE, 0.3), 8.0);
        }
        // 特待生：メダル
        5 => {
            dl.line(cx - 10.0, cy - 26.0, cx, cy - 4.0, 6.0, Color::hex(0x4F8CF0));
            dl.line(cx + 10.0, cy - 26.0, cx, cy - 4.0, 6.0, Color::hex(0xF06A5A));
            dl.circle(cx, cy + 6.0, 16.0, c);
            dl.star(cx, cy + 6.0, 10.0, Color::hex(0xFFF4D0));
        }
        // 入退室メール：スマホ
        6 => {
            dl.rect_border(Rect::new(cx - 16.0, cy - 28.0, 32.0, 56.0), Color::hex(0x1C2A3A), 6.0, 2.5, c);
            dl.rect(Rect::new(cx - 10.0, cy - 12.0, 20.0, 14.0), Color::hex(0xF4F1E8), 2.0);
            dl.line(cx - 10.0, cy - 12.0, cx, cy - 4.0, 1.5, Color::hex(0x8A8A8A));
            dl.line(cx + 10.0, cy - 12.0, cx, cy - 4.0, 1.5, Color::hex(0x8A8A8A));
        }
        // Web 広告：ノート PC
        _ => {
            dl.rect_border(
                Rect::new(cx - 28.0, cy - 22.0, 56.0, 34.0),
                Color::hex(0x1C2A3A),
                3.0,
                2.5,
                Color::hex(0x9DB0A6),
            );
            dl.rect(Rect::new(cx - 36.0, cy + 12.0, 72.0, 6.0), Color::hex(0x9DB0A6), 3.0);
            dl.rect(Rect::new(cx - 18.0, cy - 12.0, 36.0, 8.0), c, 2.0);
            dl.line(cx - 18.0, cy + 2.0, cx + 10.0, cy + 2.0, 2.0, Color::WHITE.with_alpha(0.6));
        }
    }
}
