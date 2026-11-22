//! 画面上部：教室名・日付・資金・「次の週へ」・教室長のひとこと。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::World;
use juku_ui::{Align, ButtonStyle, Ui};

use crate::art;
use crate::fmt::format_yen;

/// 週を進めたら true。
pub fn draw(ui: &mut Ui, r: Rect, world: &mut World) -> bool {
    let t = ui.theme;
    ui.dl.rect(r, t.bar, 0.0);
    ui.dl.rect(Rect::new(r.x, r.bottom() - 1.0, r.w, 1.0), t.card_border, 0.0);
    let pad = 16.0;

    // 1 行目：ロゴ・教室名・日付
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
        Rect::new(name_x, r.y + 8.0, r.w, 24.0),
        &world.classroom_name,
        t.font_heading,
        FontWeight::Bold,
        t.text,
        Align::Left,
    );
    ui.text_in(
        Rect::new(name_x, r.y + 30.0, r.w, 20.0),
        &world.address,
        t.font_small,
        FontWeight::Regular,
        t.text_dim,
        Align::Left,
    );
    let date = world.calendar.date();
    let date_r = Rect::new(r.x, r.y + 8.0, r.w - pad, 24.0);
    ui.text_in(date_r, &date.label(), t.font_body, FontWeight::Bold, t.text, Align::Right);
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
        "資金",
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
    ui.text_in(
        Rect::new(sx, row.y - 2.0, 100.0, 18.0),
        "生徒",
        t.font_small,
        FontWeight::Regular,
        t.text_dim,
        Align::Left,
    );
    ui.text_in(
        Rect::new(sx, row.y + 14.0, 100.0, 26.0),
        &format!("{}人", world.students.len()),
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

    // 3 行目：教室長のひとこと
    let bubble = Rect::new(r.x + pad, r.y + 106.0, r.w - pad * 2.0, 36.0);
    let look = art::Look::from_name(&world.manager.name);
    art::person(ui, Rect::new(bubble.x, bubble.y - 6.0, 36.0, 42.0), look, true);
    let msg_r = Rect::new(bubble.x + 44.0, bubble.y + 2.0, bubble.w - 44.0, 32.0);
    ui.dl.rect(msg_r, Color::WHITE.with_alpha(0.06), 10.0);
    let msg = manager_message(world);
    let fitted = ui.fit_text(&msg, t.font_small + 1.0, FontWeight::Regular, msg_r.w - 20.0);
    ui.text_in(
        msg_r.shrink(10.0).translate(0.0, 0.0),
        &fitted,
        t.font_small + 1.0,
        FontWeight::Regular,
        t.text,
        Align::Left,
    );
    stepped
}

fn manager_message(world: &World) -> String {
    if world.pending_event.is_some() {
        return "塾長、ご判断をお願いします。".into();
    }
    let Some(r) = world.last_report() else {
        return "こんにちは塾長。今年度もよろしくお願いします！".into();
    };
    if r.coverage < 0.95 {
        return format!("講師が足りません（充足率 {:.0}%）。採用をご検討ください。", r.coverage * 100.0);
    }
    if let Some(n) = r.news.first() {
        return n.clone();
    }
    if r.new_students > 0 {
        return format!("今週は {} 人が入塾しました！", r.new_students);
    }
    "こんにちは塾長。今週も順調です。".into()
}
