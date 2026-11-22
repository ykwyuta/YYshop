//! 画面上部：会社名・教室の切り替え・日付・資金・「次の週へ」・教室長のひとこと。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::World;
use juku_ui::{Align, ButtonStyle, Ui};

use crate::art;
use crate::fmt::format_yen;

/// (週を進めたか, 切り替え先の教室)
/// ヘッダーで起きたこと。
pub struct HeaderResult {
    pub stepped: bool,
    pub switch: Option<usize>,
    /// 秘書を呼んだ
    pub secretary: bool,
}

/// `reports` は秘書が抱えている報告の数（バッジに出す）。
pub fn draw(ui: &mut Ui, r: Rect, world: &mut World, ci: usize, reports: usize) -> HeaderResult {
    let t = ui.theme;
    ui.dl.rect(r, t.bar, 0.0);
    ui.dl.rect(Rect::new(r.x, r.bottom() - 1.0, r.w, 1.0), t.card_border, 0.0);
    let pad = 16.0;
    let n = world.classrooms.len();
    let mut switch = None;

    // 1 行目：ロゴ・会社名・教室名（切り替え）・日付
    let logo_c = (r.x + pad + 20.0, r.y + 30.0);
    ui.dl.circle(logo_c.0, logo_c.1, 20.0, t.accent);
    ui.dl.circle(logo_c.0, logo_c.1, 16.5, t.accent_dark);
    ui.text_in(
        Rect::new(logo_c.0 - 20.0, logo_c.1 - 12.0, 40.0, 24.0),
        "塾",
        18.0,
        FontWeight::Bold,
        t.text,
        Align::Center,
    );
    let name_x = r.x + pad + 50.0;
    ui.text_in(
        Rect::new(name_x, r.y + 6.0, r.w, 24.0),
        &world.company_name,
        t.font_heading,
        FontWeight::Bold,
        t.text,
        Align::Left,
    );
    if world.listing.is_listed() {
        let w = ui.measure(&world.company_name, t.font_heading, FontWeight::Bold);
        ui.badge_at(name_x + w + 8.0, r.y + 7.0, "上場", t.star);
    }

    let c = &world.classrooms[ci];
    let label = if n > 1 { format!("{}（{}/{}）", c.name, ci + 1, n) } else { format!("{}・{}", c.name, c.address) };
    if n > 1 {
        // ‹ 教室名 › で切り替える
        let lw = ui.measure(&label, t.font_small + 1.0, FontWeight::Bold);
        let y = r.y + 30.0;
        let prev = Rect::new(name_x - 6.0, y - 2.0, 26.0, 24.0);
        let text_r = Rect::new(prev.right(), y, lw + 4.0, 20.0);
        let next = Rect::new(text_r.right(), y - 2.0, 26.0, 24.0);
        for (rr, left) in [(prev, true), (next, false)] {
            let (cx, cy) = rr.center();
            let d = if left { 4.0 } else { -4.0 };
            let col = if ui.pressed(rr) { t.text } else { t.accent };
            ui.dl.line(cx + d, cy - 6.0, cx - d, cy, 2.5, col);
            ui.dl.line(cx - d, cy, cx + d, cy + 6.0, 2.5, col);
        }
        ui.text_in(text_r, &label, t.font_small + 1.0, FontWeight::Bold, t.accent, Align::Center);
        if ui.clicked(prev) {
            switch = Some((ci + n - 1) % n);
        }
        if ui.clicked(next) || ui.clicked(text_r) {
            switch = Some((ci + 1) % n);
        }
    } else {
        ui.text_in(
            Rect::new(name_x, r.y + 30.0, r.w, 20.0),
            &label,
            t.font_small,
            FontWeight::Regular,
            t.text_dim,
            Align::Left,
        );
    }

    let date = world.calendar.date();
    ui.text_in(
        Rect::new(r.x, r.y + 8.0, r.w - pad, 24.0),
        &date.label(),
        t.font_body,
        FontWeight::Bold,
        t.text,
        Align::Right,
    );
    let season = world.calendar.season().label();
    let wk = format!("{}年度 第{}週・{}", world.calendar.school_year, world.calendar.week + 1, season);
    ui.text_in(
        Rect::new(r.x, r.y + 30.0, r.w - pad, 20.0),
        &wk,
        t.font_small,
        FontWeight::Regular,
        t.text_dim,
        Align::Right,
    );

    // 2 行目：資金・生徒数・次の週へ
    let row = Rect::new(r.x + pad, r.y + 58.0, r.w - pad * 2.0, 40.0);
    let cash_color = if world.cash < 0 { t.negative } else { t.text };
    ui.text_in(
        Rect::new(row.x, row.y - 2.0, 200.0, 18.0),
        "会社の資金",
        t.font_small,
        FontWeight::Regular,
        t.text_dim,
        Align::Left,
    );
    ui.text_in(
        Rect::new(row.x, row.y + 14.0, 200.0, 26.0),
        &format_yen(world.cash),
        t.font_heading + 1.0,
        FontWeight::Bold,
        cash_color,
        Align::Left,
    );
    let sx = row.x + 150.0;
    let students = world.classrooms[ci].students.len();
    let (slabel, svalue) = if n > 1 {
        ("生徒（教室/全体）", format!("{}/{}", students, world.total_students()))
    } else {
        ("生徒", format!("{students}人"))
    };
    ui.text_in(
        Rect::new(sx, row.y - 2.0, 120.0, 18.0),
        slabel,
        t.font_small,
        FontWeight::Regular,
        t.text_dim,
        Align::Left,
    );
    ui.text_in(
        Rect::new(sx, row.y + 14.0, 120.0, 26.0),
        &svalue,
        t.font_heading + 1.0,
        FontWeight::Bold,
        t.text,
        Align::Left,
    );

    let btn = Rect::new(row.right() - 128.0, row.y, 128.0, 40.0);
    let mut stepped = false;
    if ui.button_at(btn, "次の週へ →", ButtonStyle::Primary) {
        stepped = world.step_week();
    }

    // 3 行目：教室長のひとこと ＋ 秘書を呼ぶボタン
    let sec = Rect::new(r.right() - pad - 76.0, r.y + 104.0, 76.0, 40.0);
    let bubble = Rect::new(r.x + pad, r.y + 106.0, r.w - pad * 2.0 - 84.0, 36.0);
    let look = art::Look::from_name(&world.classrooms[ci].manager.name);
    art::person(ui, Rect::new(bubble.x, bubble.y - 6.0, 36.0, 42.0), look, true);
    let msg_r = Rect::new(bubble.x + 44.0, bubble.y + 2.0, bubble.w - 44.0, 32.0);
    ui.dl.rect(msg_r, Color::WHITE.with_alpha(0.06), 10.0);
    let msg = manager_message(world, ci);
    let fitted = ui.fit_text(&msg, t.font_small + 1.0, FontWeight::Regular, msg_r.w - 20.0);
    ui.text_in(msg_r.shrink(10.0), &fitted, t.font_small + 1.0, FontWeight::Regular, t.text, Align::Left);

    let pressed = ui.pressed(sec);
    let fill = if pressed { t.accent_dark } else { t.card };
    ui.dl.rect_border(sec, fill, 12.0, 1.5, t.accent);
    art::person(ui, Rect::new(sec.x + 2.0, sec.y + 1.0, 30.0, 38.0), crate::screens::secretary::look(), true);
    ui.text_in(
        Rect::new(sec.x + 32.0, sec.y, sec.w - 34.0, sec.h),
        "秘書",
        t.font_small + 1.0,
        FontWeight::Bold,
        t.text,
        Align::Center,
    );
    if reports > 0 {
        let label = if reports > 9 { "9+".to_string() } else { reports.to_string() };
        ui.dl.circle(sec.right() - 4.0, sec.y + 4.0, 10.0, t.negative);
        ui.text_in(
            Rect::new(sec.right() - 14.0, sec.y - 6.0, 20.0, 20.0),
            &label,
            t.font_small - 1.0,
            FontWeight::Bold,
            Color::WHITE,
            Align::Center,
        );
    }
    let secretary = ui.clicked(sec);
    HeaderResult { stepped, switch, secretary }
}

fn manager_message(world: &World, ci: usize) -> String {
    if world.pending_event.is_some() {
        return "塾長、ご判断をお願いします。".into();
    }
    if world.cash < 0 {
        return format!(
            "資金がマイナスです！あと {} 週で倒産します。融資や経費削減を急いでください。",
            juku_sim::world::BANKRUPTCY_WEEKS.saturating_sub(world.negative_weeks)
        );
    }
    let c = &world.classrooms[ci];
    let Some(r) = c.last_report() else {
        if c.students.is_empty() {
            return "新しい教室です。まずは講師をそろえて、生徒を集めましょう！".into();
        }
        return "こんにちは塾長。今年度もよろしくお願いします！".into();
    };
    if r.coverage < 0.95 && !c.students.is_empty() {
        return format!("講師が足りません（充足率 {:.0}%）。採用をご検討ください。", r.coverage * 100.0);
    }
    if let Some(n) = r.news.first() {
        return n.clone();
    }
    if let Some(n) = world.last_week().and_then(|w| w.news.first()) {
        return n.clone();
    }
    "こんにちは塾長。今週も順調です。".into()
}
