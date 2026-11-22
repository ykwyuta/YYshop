//! コースタブ：在籍数の推移・指導方針・コースカード。

use juku_render::{FontWeight, Rect};
use juku_sim::{CourseKind, World};
use juku_ui::{Align, Id, Series, Ui};

use crate::fmt::{format_signed, format_yen};

pub fn draw(ui: &mut Ui, world: &mut World, ci: usize, segment: &mut usize) {
    let t = ui.theme;
    trend(ui, world, ci);

    ui.heading("指導方針");
    ui.card(16.0, |ui| {
        ui.set_spacing(10.0);
        ui.kv_row("宿題の量", "", None);
        let r = ui.alloc(56.0);
        let labels = ["とても少ない", "少なめ", "ふつう", "多め", "とても多い"];
        let label = labels[(world.classrooms[ci].homework - 1) as usize];
        let d =
            ui.stepper_at(Id::new("homework"), r, label, Some(&format!("レベル {}", world.classrooms[ci].homework)));
        if d != 0 {
            let level = (world.classrooms[ci].homework as i32 + d).clamp(1, 5) as u8;
            world.classroom_mut(ci).set_homework(level);
        }
        ui.caption("宿題を増やすと学力は伸びますが、ストレスがたまり退塾が増えます。");
    });

    ui.heading("コース");
    if let Some(i) = ui.segmented(&["学年別本科", "講習・特訓"], *segment) {
        *segment = i;
    }
    let indices: Vec<usize> = world.classrooms[ci]
        .courses
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c.kind, CourseKind::Main(_)) == (*segment == 0))
        .map(|(i, _)| i)
        .collect();
    for i in indices {
        course_card(ui, world, ci, i);
    }
    let _ = t;
}

fn trend(ui: &mut Ui, world: &World, ci: usize) {
    let room = &world.classrooms[ci];
    let t = ui.theme;
    let hist: Vec<_> = room.history.iter().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect();
    ui.card(16.0, |ui| {
        ui.set_spacing(8.0);
        let r = ui.alloc(60.0);
        let total = room.students.len() as i64;
        let prev = hist.iter().rev().nth(1).map(|h| h.total_students() as i64).unwrap_or(total);
        let cols = [("4年", 4u8, t.series[0]), ("5年", 5, t.series[4]), ("6年", 6, t.series[1])];
        let cw = r.w / 4.0;
        ui.text_in(Rect::new(r.x, r.y, cw, 18.0), "在籍", t.font_small, FontWeight::Regular, t.text_dim, Align::Left);
        ui.text_in(
            Rect::new(r.x, r.y + 18.0, cw, 34.0),
            &format!("{total}人"),
            26.0,
            FontWeight::Bold,
            t.text,
            Align::Left,
        );
        let (dtext, dcolor) = crate::delta(ui, total, prev, true);
        ui.text_in(
            Rect::new(r.x, r.y + 46.0, cw, 16.0),
            &dtext,
            t.font_small,
            FontWeight::Regular,
            dcolor,
            Align::Left,
        );
        for (k, (label, g, c)) in cols.iter().enumerate() {
            let x = r.x + cw * (k + 1) as f32;
            ui.text_in(Rect::new(x, r.y, cw - 8.0, 18.0), label, t.font_small, FontWeight::Regular, *c, Align::Right);
            ui.text_in(
                Rect::new(x, r.y + 18.0, cw - 8.0, 34.0),
                &room.students_in_grade(*g).to_string(),
                24.0,
                FontWeight::Bold,
                *c,
                Align::Right,
            );
        }
        if hist.len() >= 2 {
            let labels: Vec<String> = hist.iter().map(|h| h.date.short_label()).collect();
            let s: Vec<Vec<f32>> = (0..3).map(|gi| hist.iter().map(|h| h.students[gi] as f32).collect()).collect();
            let series = [
                Series { values: &s[0], color: t.series[0] },
                Series { values: &s[1], color: t.series[4] },
                Series { values: &s[2], color: t.series[1] },
            ];
            let r = ui.alloc(170.0);
            ui.line_chart(r, &labels, &series, false);
        } else {
            ui.caption("週を進めると在籍数の推移が表示されます。");
        }
    });
}

fn course_card(ui: &mut Ui, world: &mut World, ci: usize, index: usize) {
    let t = ui.theme;
    let c = world.classrooms[ci].courses[index].clone();
    let unit_cost = world.course_unit_cost(ci, c.kind);
    let (fee_label, monthly_fee) = match c.kind {
        CourseKind::Summer => ("受講料（総額）", c.fee as f64 / (6.0 * 12.0 / 52.0)),
        _ => ("月謝", c.fee as f64),
    };
    let margin = if monthly_fee > 0.0 { (1.0 - unit_cost as f64 / monthly_fee).clamp(-1.0, 1.0) } else { 0.0 };

    ui.card(18.0, |ui| {
        ui.set_spacing(12.0);
        let r = ui.alloc(30.0);
        ui.text_in(r, &c.name, t.font_title, FontWeight::Bold, t.text, Align::Left);
        let tw = ui.measure(&c.name, t.font_title, FontWeight::Bold);
        ui.stars_at(r.x + tw + 12.0, r.y + 6.0, 16.0, c.rating);

        let r = ui.alloc(46.0);
        if let Some(v) = ui.toggle_at(Id::new("course-enabled").with(index as u64), r, c.enabled) {
            world.classroom_mut(ci).set_course_enabled(index, v);
        }
        ui.text_in(
            Rect::new(r.x + 64.0, r.y, 120.0, r.h),
            if c.enabled { "開講中" } else { "休講" },
            t.font_body,
            FontWeight::Regular,
            t.text,
            Align::Left,
        );
        let label = match c.kind {
            CourseKind::Main(_) => "在籍",
            _ => "先週の受講",
        };
        ui.text_in(
            Rect::new(r.x, r.y - 4.0, r.w, 18.0),
            label,
            t.font_small,
            FontWeight::Regular,
            t.text_dim,
            Align::Right,
        );
        let count = format!("{}人", c.last_enrolled);
        ui.text_in(Rect::new(r.x, r.y + 12.0, r.w, 30.0), &count, 24.0, FontWeight::Bold, t.text, Align::Right);
        let d = c.last_enrolled as i64 - c.prev_enrolled as i64;
        let dc = if d > 0 {
            t.positive
        } else if d < 0 {
            t.negative
        } else {
            t.text_dim
        };
        ui.text_in(
            Rect::new(r.x, r.y + 40.0, r.w, 16.0),
            &format_signed(d),
            t.font_small,
            FontWeight::Regular,
            dc,
            Align::Right,
        );
        if let CourseKind::Main(_) = c.kind {
            ui.caption(&format!("定員 {} 人", c.capacity));
        } else if c.kind == CourseKind::Summer {
            ui.caption("7 月下旬〜8 月に開講。新しい生徒が入る大きなチャンスです。");
        } else {
            ui.caption("9 月〜1 月に 6 年生向けに開講。合格率を押し上げます。");
        }

        let r = ui.alloc(120.0);
        let (cx, cy) = (r.x + 58.0, r.y + 62.0);
        let m = margin.max(0.0) as f32;
        ui.donut(
            cx,
            cy,
            52.0,
            38.0,
            &[(1.0 - m, t.negative.lerp(t.accent_dark, 0.4)), (m, t.star.lerp(t.accent, 0.3))],
            false,
        );
        ui.text_in(
            Rect::new(cx - 40.0, cy - 22.0, 80.0, 20.0),
            "粗利",
            t.font_small,
            FontWeight::Regular,
            t.star,
            Align::Center,
        );
        ui.text_in(
            Rect::new(cx - 40.0, cy - 2.0, 80.0, 24.0),
            &format!("{:.0}%", margin * 100.0),
            t.font_heading,
            FontWeight::Bold,
            t.star,
            Align::Center,
        );
        ui.text_in(
            Rect::new(r.x, r.y - 8.0, 116.0, 18.0),
            &format!("原価 {}/月", format_yen(unit_cost)),
            t.font_small,
            FontWeight::Regular,
            t.negative,
            Align::Center,
        );

        let sx = r.x + 136.0;
        ui.text_in(
            Rect::new(sx, r.y + 10.0, r.right() - sx, 20.0),
            fee_label,
            t.font_small,
            FontWeight::Regular,
            t.text_dim,
            Align::Left,
        );
        let step = Rect::new(sx, r.y + 36.0, r.right() - sx, 54.0);
        let d = ui.stepper_at(Id::new("course-fee").with(index as u64), step, &format_yen(c.fee), None);
        if d != 0 {
            world.classroom_mut(ci).set_course_fee(index, c.fee + d as i64 * c.fee_step());
        }
    });
}
