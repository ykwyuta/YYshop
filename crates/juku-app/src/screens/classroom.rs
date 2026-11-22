//! 教室タブ：口コミ・気になる生徒・教室長・講師。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::{Delegation, Teacher, TeacherKind, World};
use juku_ui::{Align, ButtonStyle, Id, Ui};

use crate::fmt::format_yen;
use crate::portraits::{manager_expression, Portrait};

pub fn draw(ui: &mut Ui, world: &mut World, ci: usize) {
    building(ui, world, ci);
    reviews(ui, world, ci);
    concerns(ui, world, ci);
    manager(ui, world, ci);
    teachers(ui, world, ci);
}

/// 校舎の画像（未生成）と、街での立ち位置。
fn building(ui: &mut Ui, world: &World, ci: usize) {
    let t = ui.theme;
    let c = &world.classrooms[ci];
    let a = c.area;
    let area = &world.areas[a];
    ui.card(12.0, |ui| {
        ui.set_spacing(6.0);
        let w = ui.width();
        let img = ui.alloc((w * 9.0 / 21.0).clamp(120.0, 220.0));
        crate::images::placeholder(
            ui,
            img,
            &format!("{}の校舎", c.name),
            &crate::images::school_file(&area.name),
            t.accent,
        );
        let r = ui.alloc(22.0);
        let title = format!("{}・{}", area.region, area.name);
        ui.text_in(r, &title, t.font_body, FontWeight::Bold, t.text, Align::Left);
        if let Some((share, rank)) = world.area_standing(a) {
            let (label, color) = if world.area_conquered(a) {
                ("制覇".to_string(), t.star)
            } else {
                (format!("地域 {rank} 位・シェア {:.0}%", share * 100.0), t.text_dim)
            };
            ui.text_in(r, &label, t.font_small, FontWeight::Bold, color, Align::Right);
        }
    });
}

fn reviews(ui: &mut Ui, world: &World, ci: usize) {
    let t = ui.theme;
    ui.heading("保護者の口コミ");
    ui.card(18.0, |ui| {
        ui.set_spacing(10.0);
        let r = ui.alloc(128.0);
        let overall = world.classrooms[ci].ratings.overall();
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
            &format!("{} 件の口コミ", world.classrooms[ci].review_count),
            t.font_body,
            FontWeight::Regular,
            t.text_dim,
            Align::Left,
        );
        let axes = [
            ("料金", world.classrooms[ci].ratings.price),
            ("授業", world.classrooms[ci].ratings.teaching),
            ("面倒見", world.classrooms[ci].ratings.care),
            ("安心・安全", world.classrooms[ci].ratings.safety),
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
    for rv in world.classrooms[ci].reviews.iter().take(3) {
        ui.card(16.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(20.0);
            ui.stars_at(r.x, r.y + 1.0, 17.0, rv.stars as f64);
            let c = if rv.positive { t.positive } else { t.negative };
            ui.text_in(r, &rv.author, t.font_small, FontWeight::Regular, c, Align::Right);
            ui.label(&rv.text);
        });
    }
    if world.classrooms[ci].reviews.is_empty() {
        ui.caption("週を進めると、保護者の口コミが届きます。");
    }
}

fn concerns(ui: &mut Ui, world: &World, ci: usize) {
    let t = ui.theme;
    let list = world.classrooms[ci].students_of_concern(4);
    ui.heading("気になる生徒");
    if list.is_empty() {
        ui.caption("いまのところ、心配な生徒はいません。");
        return;
    }
    for s in list {
        let school = &world.schools[s.target_school];
        ui.card(14.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(48.0);
            Portrait::student(s).draw(ui, Rect::new(r.x, r.y, r.h, r.h));
            let text = Rect::new(r.x + 58.0, r.y, r.w - 58.0, r.h);
            let score = format!("偏差値 {:.1}", s.deviation);
            let score_width = ui.measure(&score, t.font_body, FontWeight::Bold);
            let name = ui.fit_text(
                &format!("{}（{}年）", s.name, s.grade),
                t.font_body,
                FontWeight::Bold,
                text.w - score_width - 8.0,
            );
            ui.text_in(text, &name, t.font_body, FontWeight::Bold, t.text, Align::Left);
            ui.text_in(text, &score, t.font_body, FontWeight::Bold, t.text, Align::Right);
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

fn manager(ui: &mut Ui, world: &mut World, ci: usize) {
    let t = ui.theme;
    ui.heading("教室長");
    let m = world.classrooms[ci].manager.clone();
    ui.card(16.0, |ui| {
        ui.set_spacing(12.0);
        let r = ui.alloc(150.0);
        let portrait = Rect::new(r.x, r.y + 8.0, 116.0, 116.0);
        ui.dl.rect_border(portrait, t.track, 12.0, 1.5, t.card_border);
        Portrait::manager(&m, manager_expression(world, ci)).draw(ui, portrait.shrink(4.0));
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
            world.classroom_mut(ci).set_manager_salary(m.salary + d as i64 * 500_000);
        }
        ui.caption("教室長への業務委任");
        for (i, d) in Delegation::ALL.iter().enumerate() {
            let on = world.classrooms[ci].manager.delegated(*d);
            if let Some(v) = ui.toggle_row(Id::new("delegation").with(i as u64), d.label(), on) {
                world.classroom_mut(ci).set_delegation(*d, v);
            }
        }
        ui.caption("委任すると、教室長が自分の能力と倫理観で判断します。");
    });
}

fn teachers(ui: &mut Ui, world: &mut World, ci: usize) {
    let t = ui.theme;
    let stats = world.stats(ci);
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
    for tc in &world.classrooms[ci].teachers {
        ui.card(14.0, |ui| {
            ui.set_spacing(6.0);
            teacher_header(ui, tc);
            teacher_meters(ui, tc.skill, tc.care, Some(tc.fatigue));
            let r = ui.alloc(34.0);
            let b = Rect::new(r.right() - 96.0, r.y, 96.0, 34.0);
            if ui.button_at(b, "退職を打診", ButtonStyle::Ghost) {
                dismiss = Some(tc.id);
            }
        });
    }
    if let Some(id) = dismiss {
        world.classroom_mut(ci).dismiss_teacher(id);
    }

    ui.heading("採用候補");
    ui.caption("4 週ごとに候補が入れ替わります。");
    let mut hire = None;
    for (i, c) in world.classrooms[ci].candidates.iter().enumerate() {
        ui.card(14.0, |ui| {
            ui.set_spacing(6.0);
            teacher_header(ui, c);
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
        world.classroom_mut(ci).hire_candidate(i);
    }
    if world.classrooms[ci].candidates.is_empty() {
        ui.caption("今月の候補はもういません。");
    }
}

fn teacher_header(ui: &mut Ui, teacher: &Teacher) {
    let t = ui.theme;
    let r = ui.alloc(52.0);
    Portrait::teacher(teacher).draw(ui, Rect::new(r.x, r.y, r.h, r.h));
    let name_row = Rect::new(r.x + 62.0, r.y + 2.0, r.w - 62.0, 24.0);
    let kind = teacher.kind.label();
    let kind_color = if teacher.kind == TeacherKind::Pro { t.series[1] } else { t.series[4] };
    let badge_width = ui.measure(kind, t.font_small, FontWeight::Bold) + 14.0;
    let name = ui.fit_text(&teacher.name, t.font_body, FontWeight::Bold, name_row.w - badge_width - 8.0);
    ui.text_in(name_row, &name, t.font_body, FontWeight::Bold, t.text, Align::Left);
    let nw = ui.measure(&name, t.font_body, FontWeight::Bold);
    ui.badge_at(name_row.x + nw + 8.0, name_row.y + 1.0, kind, kind_color);
    let pay = match teacher.kind {
        TeacherKind::Pro => format!("年俸 {}", format_yen(teacher.pay)),
        TeacherKind::Student => format!("時給 {}", format_yen(teacher.pay)),
    };
    ui.text_in(name_row.translate(0.0, 24.0), &pay, t.font_small, FontWeight::Regular, t.text_dim, Align::Left);
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
