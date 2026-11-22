//! 経理タブ：売上・純利益の推移と損益計算書。

use juku_render::{FontWeight, Rect};
use juku_sim::{Pnl, World};
use juku_ui::{Align, Ui};

use crate::fmt::{format_yen, man, pct_change};

pub fn draw(ui: &mut Ui, world: &World, ci: usize, segment: &mut usize) {
    if world.classrooms.len() > 1 {
        if let Some(i) = ui.segmented(&["この教室", "会社全体"], *segment) {
            *segment = i;
        }
    } else {
        *segment = 0;
    }
    if *segment == 1 {
        company_view(ui, world);
    } else {
        classroom_view(ui, world, ci);
    }
}

fn classroom_view(ui: &mut Ui, world: &World, ci: usize) {
    let t = ui.theme;
    let room = &world.classrooms[ci];
    let Some(last) = room.last_report() else {
        ui.heading("経理");
        ui.card(18.0, |ui| {
            ui.label("まだ決算がありません。");
            ui.caption("画面上の「次の週へ」を押すと、1 週間分の損益計算書ができます。");
        });
        return;
    };
    let empty = Pnl::default();
    let prev = room.history.iter().rev().nth(1).map(|r| &r.pnl).unwrap_or(&empty);
    let cur = &last.pnl;

    ui.card(18.0, |ui| {
        ui.set_spacing(10.0);
        let r = ui.alloc(72.0);
        let half = r.w * 0.5;
        for (k, (label, v, pv, color)) in [
            ("この教室の売上（週）", cur.revenue(), prev.revenue(), t.text),
            (
                "この教室の純利益（週）",
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
        let hist: Vec<_> = room.history.iter().rev().take(5).collect::<Vec<_>>().into_iter().rev().collect();
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

    if world.classrooms.len() == 1 {
        assets(ui, world);
    }
}

/// 会社全体：教室ごとの成績と本部費用。
fn company_view(ui: &mut Ui, world: &World) {
    let t = ui.theme;
    let Some(last) = world.last_week() else {
        ui.caption("週を進めると、会社全体の決算が表示されます。");
        return;
    };
    let prev_net = world.history.iter().rev().nth(1).map(|w| w.net()).unwrap_or(0);
    ui.card(18.0, |ui| {
        ui.set_spacing(10.0);
        let r = ui.alloc(72.0);
        let half = r.w * 0.5;
        for (k, (label, v, color)) in [
            ("会社の売上（週）", last.revenue(), t.text),
            ("会社の純利益（週）", last.net(), if last.net() >= 0 { t.accent } else { t.negative }),
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
        }
        let (d, dc) = crate::delta(ui, last.net(), prev_net, true);
        let r2 = Rect::new(r.x, r.y + 50.0, r.w - 4.0, 18.0);
        ui.text_in(r2, &format!("純利益 {d}"), t.font_small, FontWeight::Regular, dc, Align::Right);
        let hist: Vec<_> = world.history.iter().rev().take(5).collect::<Vec<_>>().into_iter().rev().collect();
        let labels: Vec<String> = hist.iter().map(|h| h.date.short_label()).collect();
        let rev: Vec<f32> = hist.iter().map(|h| h.revenue() as f32).collect();
        let net: Vec<f32> = hist.iter().map(|h| h.net() as f32).collect();
        let r = ui.alloc(190.0);
        ui.bar_chart(r, &labels, &rev, &net, t.text, t.accent);
    });

    ui.heading("教室別の成績（週）");
    ui.card(18.0, |ui| {
        ui.set_spacing(4.0);
        let r = ui.alloc(22.0);
        ui.text_in(r, "教室", t.font_small, FontWeight::Regular, t.text_dim, Align::Left);
        ui.text_in(
            Rect::new(r.x, r.y, r.w * 0.46, r.h),
            "在籍",
            t.font_small,
            FontWeight::Regular,
            t.text_dim,
            Align::Right,
        );
        ui.text_in(
            Rect::new(r.x, r.y, r.w * 0.74, r.h),
            "売上",
            t.font_small,
            FontWeight::Regular,
            t.text_dim,
            Align::Right,
        );
        ui.text_in(r, "純利益", t.font_small, FontWeight::Regular, t.text_dim, Align::Right);
        for (name, rev, net, students) in &last.classrooms {
            let r = ui.alloc(26.0);
            ui.text_in(r, name, t.font_body, FontWeight::Regular, t.text, Align::Left);
            ui.text_in(
                Rect::new(r.x, r.y, r.w * 0.46, r.h),
                &format!("{students}人"),
                t.font_body,
                FontWeight::Regular,
                t.text,
                Align::Right,
            );
            ui.text_in(
                Rect::new(r.x, r.y, r.w * 0.74, r.h),
                &format_yen(*rev).replace('¥', ""),
                t.font_body,
                FontWeight::Regular,
                t.text,
                Align::Right,
            );
            let c = if *net >= 0 { t.text } else { t.negative };
            ui.text_in(r, &format_yen(*net).replace('¥', ""), t.font_body, FontWeight::Bold, c, Align::Right);
        }
        total_plain(ui, "教室の純利益 計", last.classroom_net());
        ui.space(12.0);
        header_row(ui, "本部の費用", "");
        for (label, v) in last.hq.lines() {
            let r = ui.alloc(24.0);
            ui.text_in(r, label, t.font_body, FontWeight::Regular, t.text_dim, Align::Left);
            ui.text_in(r, &format_yen(v).replace('¥', ""), t.font_body, FontWeight::Regular, t.text, Align::Right);
        }
        total_plain(ui, "本部費用 計", last.hq.total());
        ui.space(8.0);
        total_big(ui, "会社の純利益", last.net(), prev_net);
    });

    if !last.flows.is_empty() {
        ui.heading("投資・財務の動き（今週）");
        ui.card(18.0, |ui| {
            ui.set_spacing(4.0);
            for (label, v) in last.flows.lines() {
                if v == 0 {
                    continue;
                }
                let r = ui.alloc(24.0);
                ui.text_in(r, label, t.font_body, FontWeight::Regular, t.text_dim, Align::Left);
                let c = if v >= 0 { t.positive } else { t.negative };
                ui.text_in(r, &crate::fmt::format_signed(v), t.font_body, FontWeight::Regular, c, Align::Right);
            }
            ui.caption("物件の売買・買収・借入と返済は損益には入らず、資金だけが動きます。");
        });
    }
    assets(ui, world);
}

fn assets(ui: &mut Ui, world: &World) {
    let t = ui.theme;
    ui.heading("資産");
    ui.card(18.0, |ui| {
        ui.set_spacing(8.0);
        ui.kv_row("現金", &format_yen(world.cash), None);
        if world.property_assets() > 0 {
            ui.kv_row("所有物件", &format_yen(world.property_assets()), None);
        }
        if world.debt() > 0 {
            ui.kv_row("借入残高", &format!("-{}", format_yen(world.debt())), Some(t.negative));
        }
        ui.kv_row("企業価値（目安）", &format_yen(world.valuation()), Some(t.accent));
        let annual: i64 = world.history.iter().rev().take(52).map(|r| r.net()).sum();
        ui.kv_row("直近 52 週の純利益", &man(annual), None);
        ui.caption("企業価値は、現金・物件・直近の利益・ブランド力から借入を引いて見積もっています。");
    });
}

fn total_plain(ui: &mut Ui, label: &str, v: i64) {
    let t = ui.theme;
    let r = ui.alloc(30.0);
    ui.dl.rect(Rect::new(r.x + r.w * 0.6, r.y, r.w * 0.4, 1.5), t.text, 0.0);
    ui.text_in(r, label, t.font_body, FontWeight::Regular, t.text_dim, Align::Left);
    ui.text_in(r, &format_yen(v), t.font_body, FontWeight::Bold, t.text, Align::Right);
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
