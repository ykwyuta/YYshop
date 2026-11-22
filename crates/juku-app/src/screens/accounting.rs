//! 経理タブ：売上・純利益の推移と損益計算書。

use juku_render::{FontWeight, Rect};
use juku_sim::{Pnl, World};
use juku_ui::{Align, Ui};

use crate::fmt::{format_yen, man, pct_change};

pub fn draw(ui: &mut Ui, world: &World) {
    let t = ui.theme;
    let Some(last) = world.last_report() else {
        ui.heading("経理");
        ui.card(18.0, |ui| {
            ui.label("まだ決算がありません。");
            ui.caption("画面上の「次の週へ」を押すと、1 週間分の損益計算書ができます。");
        });
        return;
    };
    let empty = Pnl::default();
    let prev = world.history.iter().rev().nth(1).map(|r| &r.pnl).unwrap_or(&empty);
    let cur = &last.pnl;

    ui.card(18.0, |ui| {
        ui.set_spacing(10.0);
        let r = ui.alloc(72.0);
        let half = r.w * 0.5;
        for (k, (label, v, pv, color)) in [
            ("教室の売上（週）", cur.revenue(), prev.revenue(), t.text),
            (
                "教室の純利益（週）",
                cur.net_profit(),
                prev.net_profit(),
                if cur.net_profit() >= 0 { t.accent } else { t.negative },
            ),
        ]
        .iter()
        .enumerate()
        {
            let cell = Rect::new(r.x + half * k as f32, r.y, half - 4.0, r.h);
            ui.text_in(
                Rect::new(cell.x, cell.y, cell.w, 18.0),
                label,
                t.font_small,
                FontWeight::Regular,
                t.text_dim,
                Align::Right,
            );
            ui.text_in(
                Rect::new(cell.x, cell.y + 18.0, cell.w, 34.0),
                &format_yen(*v),
                24.0,
                FontWeight::Bold,
                *color,
                Align::Right,
            );
            let (d, dc) = crate::delta(ui, *v, *pv, true);
            ui.text_in(
                Rect::new(cell.x, cell.y + 50.0, cell.w, 18.0),
                &format!("{d}（{}）", pct_change(*v, *pv)),
                t.font_small,
                FontWeight::Regular,
                dc,
                Align::Right,
            );
        }
        let hist: Vec<_> = world.history.iter().rev().take(5).collect::<Vec<_>>().into_iter().rev().collect();
        let labels: Vec<String> = hist.iter().map(|h| h.date.short_label()).collect();
        let rev: Vec<f32> = hist.iter().map(|h| h.pnl.revenue() as f32).collect();
        let net: Vec<f32> = hist.iter().map(|h| h.pnl.net_profit() as f32).collect();
        let r = ui.alloc(190.0);
        ui.bar_chart(r, &labels, &rev, &net, t.text, t.accent);
    });

    ui.heading("損益計算書");
    ui.card(18.0, |ui| {
        ui.set_spacing(4.0);
        header_row(ui, "売上高", &last.date.label());
        for (i, (label, v)) in cur.revenue_lines().iter().enumerate() {
            line(ui, label, *v, prev.revenue_lines()[i].1, true);
        }
        total(ui, "総売上高", cur.revenue(), prev.revenue(), true);
        ui.space(14.0);
        header_row(ui, "費用", "");
        for (i, (label, v)) in cur.cost_lines().iter().enumerate() {
            line(ui, label, *v, prev.cost_lines()[i].1, false);
        }
        total(ui, "総費用", cur.costs(), prev.costs(), false);
        ui.space(14.0);
        total_big(ui, "営業利益", cur.operating_profit(), prev.operating_profit());
        line(ui, "法人税等", cur.tax, prev.tax, false);
        ui.space(8.0);
        total_big(ui, "純利益", cur.net_profit(), prev.net_profit());
    });

    ui.heading("資産");
    ui.card(18.0, |ui| {
        ui.set_spacing(8.0);
        ui.kv_row("現金", &format_yen(world.cash), None);
        ui.kv_row("企業価値（目安）", &format_yen(world.valuation()), Some(t.accent));
        let annual: i64 = world.history.iter().rev().take(52).map(|r| r.pnl.net_profit()).sum();
        ui.kv_row("直近 52 週の純利益", &man(annual), None);
        ui.caption("企業価値は、現金・直近の利益・ブランド力から見積もっています。");
    });
}

fn header_row(ui: &mut Ui, title: &str, right: &str) {
    let t = ui.theme;
    let r = ui.alloc(30.0);
    ui.text_in(r, title, t.font_heading, FontWeight::Bold, t.text, Align::Left);
    ui.text_in(
        Rect::new(r.x, r.y, r.w * 0.68, r.h),
        right,
        t.font_small,
        FontWeight::Regular,
        t.text_dim,
        Align::Right,
    );
}

fn line(ui: &mut Ui, label: &str, v: i64, prev: i64, good_when_up: bool) {
    let t = ui.theme;
    let r = ui.alloc(24.0);
    ui.text_in(r, label, t.font_body, FontWeight::Regular, t.text_dim, Align::Left);
    ui.text_in(
        Rect::new(r.x, r.y, r.w * 0.68, r.h),
        &format_yen(v).replace('¥', ""),
        t.font_body,
        FontWeight::Regular,
        t.text,
        Align::Right,
    );
    let (d, c) = crate::delta(ui, v, prev, good_when_up);
    ui.text_in(r, &d, t.font_body, FontWeight::Regular, c, Align::Right);
}

fn total(ui: &mut Ui, label: &str, v: i64, prev: i64, good_when_up: bool) {
    let t = ui.theme;
    let r = ui.alloc(30.0);
    ui.dl.rect(Rect::new(r.x + r.w * 0.42, r.y, r.w * 0.26, 1.5), t.text, 0.0);
    ui.text_in(r, label, t.font_body, FontWeight::Regular, t.text_dim, Align::Left);
    ui.text_in(
        Rect::new(r.x, r.y, r.w * 0.68, r.h),
        &format_yen(v),
        t.font_body,
        FontWeight::Bold,
        t.text,
        Align::Right,
    );
    let (d, c) = crate::delta(ui, v, prev, good_when_up);
    ui.text_in(r, &d, t.font_body, FontWeight::Bold, c, Align::Right);
}

fn total_big(ui: &mut Ui, label: &str, v: i64, prev: i64) {
    let t = ui.theme;
    let r = ui.alloc(32.0);
    ui.text_in(r, label, t.font_heading, FontWeight::Bold, t.text, Align::Left);
    let color = if v >= 0 { t.text } else { t.negative };
    ui.text_in(
        Rect::new(r.x, r.y, r.w * 0.68, r.h),
        &format_yen(v),
        t.font_body + 1.0,
        FontWeight::Bold,
        color,
        Align::Right,
    );
    let (d, c) = crate::delta(ui, v, prev, true);
    ui.text_in(r, &d, t.font_body, FontWeight::Bold, c, Align::Right);
}
