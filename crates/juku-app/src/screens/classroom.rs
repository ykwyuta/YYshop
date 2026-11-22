//! 教室タブ：口コミ・気になる生徒・教室長・講師。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::{Delegation, TeacherKind, World};
use juku_ui::{Align, ButtonStyle, Id, Ui};

use crate::art;
use crate::fmt::format_yen;

pub fn draw(ui: &mut Ui, world: &mut World) {
    reviews(ui, world);
    concerns(ui, world);
    manager(ui, world);
    teachers(ui, world);
}

fn reviews(ui: &mut Ui, world: &World) {
    let t = ui.theme;
    ui.heading("保護者の口コミ");
    ui.card(18.0, |ui| {
        ui.set_spacing(10.0);
        let r = ui.alloc(128.0);
        let overall = world.ratings.overall();
        ui.text_in(
            Rect::new(r.x, r.y, 140.0, 56.0),
            &format!("{overall:.1}"),
            44.0,
            FontWeight::Bold,
            t.text,
            Align::Left,
        );
        ui.stars_at(r.x, r.y + 62.0, 22.0, overall);
        ui.text_in(
            Rect::new(r.x, r.y + 92.0, 160.0, 24.0),
            &format!("{} 件の口コミ", world.review_count),
            t.font_body,
            FontWeight::Regular,
            t.text_dim,
            Align::Left,
        );
        let axes = [
            ("料金", world.ratings.price),
            ("授業", world.ratings.teaching),
            ("面倒見", world.ratings.care),
            ("安心・安全", world.ratings.safety),
        ];
        let right_x = r.x + r.w * 0.42;
        let rw = r.right() - right_x;
        for (i, (label, v)) in axes.iter().enumerate() {
            let y = r.y + 6.0 + i as f32 * 30.0;
            ui.text_in(
                Rect::new(right_x, y, 72.0, 24.0),
                label,
                t.font_small,
                FontWeight::Regular,
                t.text,
                Align::Right,
            );
            ui.stars_at(right_x + 80.0, y + 4.0, 15.0, *v);
            ui.text_in(
                Rect::new(right_x, y, rw, 24.0),
                &format!("{v:.1}"),
                t.font_body,
                FontWeight::Bold,
                t.text,
                Align::Right,
            );
        }
    });
    for rv in world.reviews.iter().take(3) {
        ui.card(16.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(20.0);
            ui.stars_at(r.x, r.y + 1.0, 17.0, rv.stars as f64);
            let c = if rv.positive { t.positive } else { t.negative };
            ui.text_in(r, &rv.author, t.font_small, FontWeight::Regular, c, Align::Right);
            ui.label(&rv.text);
        });
    }
    if world.reviews.is_empty() {
        ui.caption("週を進めると、保護者の口コミが届きます。");
    }
}

fn concerns(ui: &mut Ui, world: &World) {
    let t = ui.theme;
    let list = world.students_of_concern(4);
    ui.heading("気になる生徒");
    if list.is_empty() {
        ui.caption("いまのところ、心配な生徒はいません。");
        return;
    }
    for s in list {
        let school = &world.schools[s.target_school];
        ui.card(14.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(24.0);
            ui.text_in(r, &format!("{}（{}年）", s.name, s.grade), t.font_body, FontWeight::Bold, t.text, Align::Left);
            ui.text_in(r, &format!("偏差値 {:.1}", s.deviation), t.font_body, FontWeight::Bold, t.text, Align::Right);
            ui.caption(&format!("第一志望：{}（{:.0}）・保護者：{}", school.name, school.deviation, s.parent.label()));
            let r = ui.alloc(20.0);
            let half = r.w * 0.5 - 6.0;
            ui.text_in(
                Rect::new(r.x, r.y, 60.0, 20.0),
                "ストレス",
                t.font_small,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
            ui.meter_at(Rect::new(r.x + 62.0, r.y + 4.0, half - 62.0, 12.0), s.stress as f32, t.negative);
            let x2 = r.x + half + 12.0;
            ui.text_in(
                Rect::new(x2, r.y, 40.0, 20.0),
                "意欲",
                t.font_small,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
            ui.meter_at(Rect::new(x2 + 42.0, r.y + 4.0, half - 42.0, 12.0), s.motivation as f32, t.positive);
        });
    }
}

fn manager(ui: &mut Ui, world: &mut World) {
    let t = ui.theme;
    ui.heading("教室長");
    let m = world.manager.clone();
    ui.card(16.0, |ui| {
        ui.set_spacing(12.0);
        let r = ui.alloc(150.0);
        let portrait = Rect::new(r.x, r.y, 116.0, 150.0);
        ui.dl.rect_border(portrait, t.track, 12.0, 1.5, t.card_border);
        art::person(ui, portrait.shrink(8.0), art::Look::from_name(&m.name), true);
        let x = r.x + 132.0;
        let w = r.right() - x;
        ui.text_in(Rect::new(x, r.y, w, 28.0), &m.name, t.font_title, FontWeight::Bold, t.text, Align::Left);
        ui.text_in(
            Rect::new(x, r.y + 26.0, w, 20.0),
            &format!("就任 {}", m.hired.label()),
            t.font_small,
            FontWeight::Regular,
            t.text_dim,
            Align::Left,
        );
        let stats = [("講師管理", m.staffing), ("指導", m.teaching), ("営業", m.sales), ("倫理", m.ethics)];
        for (i, (label, v)) in stats.iter().enumerate() {
            let y = r.y + 54.0 + i as f32 * 24.0;
            ui.text_in(Rect::new(x, y, 64.0, 20.0), label, t.font_small, FontWeight::Regular, t.text, Align::Left);
            ui.meter_at(Rect::new(x + 64.0, y + 3.0, w - 64.0 - 32.0, 15.0), *v as f32 / 100.0, t.positive);
            ui.text_in(Rect::new(x, y, w, 20.0), &v.to_string(), t.font_small, FontWeight::Bold, t.text, Align::Right);
        }

        ui.caption("年俸");
        let r = ui.alloc(56.0);
        let d = ui.stepper_at(Id::new("manager-salary"), r, &format_yen(m.salary), None);
        if d != 0 {
            world.set_manager_salary(m.salary + d as i64 * 500_000);
        }
        ui.caption("教室長への業務委任");
        for (i, d) in Delegation::ALL.iter().enumerate() {
            let on = world.manager.delegated(*d);
            if let Some(v) = ui.toggle_row(Id::new("delegation").with(i as u64), d.label(), on) {
                world.set_delegation(*d, v);
            }
        }
        ui.caption("委任すると、教室長が自分の能力と倫理観で判断します。");
    });
}

fn teachers(ui: &mut Ui, world: &mut World) {
    let t = ui.theme;
    let stats = world.classroom_stats();
    ui.heading("講師");
    ui.card(16.0, |ui| {
        ui.set_spacing(8.0);
        let color = if stats.coverage < 0.95 {
            t.negative
        } else if stats.coverage > 1.4 {
            t.star
        } else {
            t.positive
        };
        ui.kv_row(
            "授業の充足率",
            &format!("{:.0}%（{:.0} / {:.0} 時間）", stats.coverage * 100.0, stats.available_hours, stats.needed_hours),
            Some(color),
        );
        ui.kv_row("授業品質", &format!("{:.0}", stats.quality * 100.0), None);
        ui.kv_row("面倒見", &format!("{:.0}", stats.care * 100.0), None);
        ui.caption("充足率が 100% を下回ると授業の質が落ち、講師が疲れて辞めてしまいます。");
    });

    let mut dismiss = None;
    for tc in &world.teachers {
        ui.card(14.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(24.0);
            ui.text_in(r, &tc.name, t.font_body, FontWeight::Bold, t.text, Align::Left);
            let kind_color = if tc.kind == TeacherKind::Pro { t.series[1] } else { t.series[4] };
            let nw = ui.measure(&tc.name, t.font_body, FontWeight::Bold);
            ui.badge_at(r.x + nw + 10.0, r.y + 1.0, tc.kind.label(), kind_color);
            let pay = match tc.kind {
                TeacherKind::Pro => format!("年俸 {}", format_yen(tc.pay)),
                TeacherKind::Student => format!("時給 {}", format_yen(tc.pay)),
            };
            ui.text_in(r, &pay, t.font_small, FontWeight::Regular, t.text_dim, Align::Right);
            teacher_meters(ui, tc.skill, tc.care, Some(tc.fatigue));
            let r = ui.alloc(34.0);
            let b = Rect::new(r.right() - 96.0, r.y, 96.0, 34.0);
            if ui.button_at(b, "退職を打診", ButtonStyle::Ghost) {
                dismiss = Some(tc.id);
            }
        });
    }
    if let Some(id) = dismiss {
        world.dismiss_teacher(id);
    }

    ui.heading("採用候補");
    ui.caption("4 週ごとに候補が入れ替わります。");
    let mut hire = None;
    for (i, c) in world.candidates.iter().enumerate() {
        ui.card(14.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(24.0);
            ui.text_in(r, &c.name, t.font_body, FontWeight::Bold, t.text, Align::Left);
            let kind_color = if c.kind == TeacherKind::Pro { t.series[1] } else { t.series[4] };
            let nw = ui.measure(&c.name, t.font_body, FontWeight::Bold);
            ui.badge_at(r.x + nw + 10.0, r.y + 1.0, c.kind.label(), kind_color);
            let pay = match c.kind {
                TeacherKind::Pro => format!("年俸 {}", format_yen(c.pay)),
                TeacherKind::Student => format!("時給 {}", format_yen(c.pay)),
            };
            ui.text_in(r, &pay, t.font_small, FontWeight::Regular, t.text_dim, Align::Right);
            teacher_meters(ui, c.skill, c.care, None);
            let r = ui.alloc(38.0);
            let b = Rect::new(r.right() - 110.0, r.y, 110.0, 38.0);
            if ui.button_at(b, "採用する", ButtonStyle::Primary) {
                hire = Some(i);
            }
            let hours = c.kind.weekly_hours();
            ui.text_in(
                Rect::new(r.x, r.y, r.w - 120.0, 38.0),
                &format!("週 {hours:.0} 時間担当"),
                t.font_small,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
        });
    }
    if let Some(i) = hire {
        world.hire_candidate(i);
    }
    if world.candidates.is_empty() {
        ui.caption("今月の候補はもういません。");
    }
}

fn teacher_meters(ui: &mut Ui, skill: u32, care: u32, fatigue: Option<f64>) {
    let t = ui.theme;
    let mut items: Vec<(&str, f32, Color)> =
        vec![("授業力", skill as f32 / 100.0, t.positive), ("面倒見", care as f32 / 100.0, t.series[4])];
    if let Some(f) = fatigue {
        items.push(("疲労", f as f32, t.negative));
    }
    for (label, v, c) in items {
        let r = ui.alloc(18.0);
        ui.text_in(Rect::new(r.x, r.y, 56.0, 18.0), label, t.font_small, FontWeight::Regular, t.text_dim, Align::Left);
        ui.meter_at(Rect::new(r.x + 58.0, r.y + 3.0, r.w - 58.0 - 36.0, 12.0), v, c);
        ui.text_in(r, &format!("{:.0}", v * 100.0), t.font_small, FontWeight::Bold, t.text, Align::Right);
    }
}
