//! 本部タブ：教室一覧・新規開校・自社教材・年間カレンダー・入試結果・セーブ。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::materials::{self, AXIS_COUNT, AXIS_DIFFICULTY, AXIS_LABELS, AXIS_NOTES, PROJECT_WEEKS};
use juku_sim::{Calendar, Season, World};
use juku_ui::{Align, ButtonStyle, Ui};

use crate::fmt::{format_yen, pct};

pub enum HqAction {
    ShowExam(usize),
    NewGame,
    Save,
    /// 教室を選んで教室タブへ
    Select(usize),
    /// このエリアに開校する（教室長を選ぶ）
    Open(usize),
    Toast(String),
}

pub fn draw(ui: &mut Ui, world: &mut World) -> Option<HqAction> {
    let t = ui.theme;
    let mut action = None;

    classrooms(ui, world, &mut action);
    opening(ui, world, &mut action);
    materials_section(ui, world, &mut action);

    ui.heading("年間カレンダー");
    ui.card(16.0, |ui| {
        ui.set_spacing(10.0);
        let r = ui.alloc(56.0);
        let segs = [
            (Season::NewTerm, "新学年", "2–3月"),
            (Season::Spring, "春〜初夏", "4–7月"),
            (Season::Summer, "夏期", "7–8月"),
            (Season::Autumn, "特訓", "9–11月"),
            (Season::ExamRun, "直前期", "12–1月"),
        ];
        // 週数で幅を割り振る
        let counts: Vec<u32> = segs
            .iter()
            .map(|(s, _, _)| {
                (0..52)
                    .filter(|w| Calendar { school_year: world.calendar.school_year, week: *w }.season() == *s)
                    .count() as u32
            })
            .collect();
        let mut x = r.x;
        let current = world.calendar.season();
        for (i, (s, label, months)) in segs.iter().enumerate() {
            let w = r.w * counts[i] as f32 / 52.0;
            let cell = Rect::new(x + 1.0, r.y, w - 2.0, 40.0);
            let active = *s == current;
            let c = if active { t.accent } else { t.track };
            ui.dl.rect(cell, c, 6.0);
            let tc = if active { Color::hex(0x1A1208) } else { t.text_dim };
            ui.text_in(cell, label, t.font_small - 1.0, FontWeight::Bold, tc, Align::Center);
            ui.text_in(
                Rect::new(cell.x, r.y + 40.0, cell.w, 16.0),
                months,
                10.0,
                FontWeight::Regular,
                t.text_dim,
                Align::Center,
            );
            x += w;
        }
        // 現在位置
        let px = r.x + r.w * (world.calendar.week as f32 + 0.5) / 52.0;
        ui.dl.line(px, r.y - 4.0, px, r.y + 44.0, 2.5, t.text);
        let weeks_left = 52 - world.calendar.week;
        ui.kv_row("2 月の入試まで", &format!("あと {weeks_left} 週"), Some(t.accent));
        let devs: Vec<f64> = world
            .classrooms
            .iter()
            .flat_map(|c| c.students.iter())
            .filter(|s| s.grade == 6)
            .map(|s| s.deviation)
            .collect();
        let seniors = devs.len();
        ui.kv_row("受験予定の 6 年生（全教室）", &format!("{seniors} 人"), None);
        let avg6 = if seniors > 0 { devs.iter().sum::<f64>() / seniors as f64 } else { 0.0 };
        ui.kv_row("6 年生の平均偏差値", &format!("{avg6:.1}"), None);
    });

    ui.heading("入試結果");
    if world.exam_results.is_empty() {
        ui.caption("最初の入試は 2 月 1 日です。6 年生の合否が、翌年の評判（ブランド力）を決めます。");
    }
    for (i, e) in world.exam_results.iter().enumerate().rev() {
        ui.card(16.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(28.0);
            ui.text_in(
                r,
                &format!("{}年度 入試", e.school_year),
                t.font_heading,
                FontWeight::Bold,
                t.text,
                Align::Left,
            );
            if e.padded {
                let w = ui.measure(&format!("{}年度 入試", e.school_year), t.font_heading, FontWeight::Bold);
                ui.badge_at(r.x + w + 10.0, r.y + 3.0, "実績上積み", t.negative);
            }
            ui.kv_row("受験者 / 合格件数", &format!("{} 人 / {} 件", e.candidates, e.total_passes()), None);
            ui.kv_row("第一志望合格率", &pct(e.first_choice_rate()), Some(t.accent));
            ui.kv_row("ブランド力", &format!("{} → {}", pct(e.brand_before), pct(e.brand_after)), None);
            if ui.button("結果を見る", ButtonStyle::Secondary) {
                action = Some(HqAction::ShowExam(i));
            }
        });
    }

    ui.heading("データ");
    ui.card(16.0, |ui| {
        ui.set_spacing(10.0);
        ui.caption("週を進めるたびに自動で保存されます。");
        if ui.button("今すぐ保存", ButtonStyle::Ghost) {
            action = Some(HqAction::Save);
        }
        if ui.button("新しいゲームを始める", ButtonStyle::Secondary) {
            action = Some(HqAction::NewGame);
        }
    });
    action
}

fn classrooms(ui: &mut Ui, world: &World, action: &mut Option<HqAction>) {
    let t = ui.theme;
    ui.heading(&format!("教室一覧（{} 教室）", world.classrooms.len()));
    for (ci, c) in world.classrooms.iter().enumerate() {
        ui.card(16.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(26.0);
            ui.text_in(r, &c.name, t.font_heading, FontWeight::Bold, t.text, Align::Left);
            let net = c.last_report().map(|r| r.pnl.net_profit()).unwrap_or(0);
            let nc = if net >= 0 { t.accent } else { t.negative };
            ui.text_in(r, &format!("純利益 {}/週", format_yen(net)), t.font_body, FontWeight::Bold, nc, Align::Right);
            ui.caption(&format!(
                "{}・教室長 {}・講師 {} 人",
                world.areas[c.area].name,
                c.manager.name,
                c.teachers.len()
            ));
            let r = ui.alloc(22.0);
            ui.text_in(
                r,
                &format!("在籍 {} 人", c.students.len()),
                t.font_body,
                FontWeight::Regular,
                t.text,
                Align::Left,
            );
            ui.stars_at(r.x + 110.0, r.y + 3.0, 15.0, c.ratings.overall());
            let delegated = c.manager.delegations.iter().filter(|d| **d).count();
            ui.text_in(r, &format!("委任 {delegated}/4"), t.font_small, FontWeight::Regular, t.text_dim, Align::Right);
            if ui.button("この教室を見る", ButtonStyle::Ghost) {
                *action = Some(HqAction::Select(ci));
            }
        });
    }
    if world.classrooms.len() > 1 {
        ui.caption("画面上部の ‹ › で教室を切り替えられます。教室が増えたら、教室長への業務委任を活用しましょう。");
    }
}

fn opening(ui: &mut Ui, world: &World, action: &mut Option<HqAction>) {
    let t = ui.theme;
    let areas = world.open_areas();
    if areas.is_empty() {
        return;
    }
    ui.heading("新しい教室を開く");
    ui.caption("開校すると本部の運営費（週 ¥40,000／教室）がかかります。新しい教室は生徒ゼロから始まります。");
    for a in areas {
        let area = &world.areas[a];
        let rivals: Vec<&str> =
            world.competitors.iter().filter(|c| c.branch(a).is_some()).map(|c| c.name.as_str()).collect();
        ui.card(16.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(26.0);
            ui.text_in(r, &format!("{}エリア", area.name), t.font_heading, FontWeight::Bold, t.text, Align::Left);
            ui.text_in(r, &format!("市場規模 ×{:.2}", area.pool), t.font_body, FontWeight::Bold, t.star, Align::Right);
            ui.label_ex(&area.description, t.font_small, FontWeight::Regular, t.text_dim, Align::Left);
            ui.kv_row("家賃", &format!("{}/週", format_yen(area.rent)), None);
            let rv = if rivals.is_empty() { "なし".to_string() } else { rivals.join("・") };
            ui.label_ex(&format!("競合：{rv}"), t.font_small, FontWeight::Regular, t.text, Align::Left);
            let affordable = world.cash >= area.open_cost;
            let label = format!("開校する（{}）", format_yen(area.open_cost));
            let style = if affordable { ButtonStyle::Primary } else { ButtonStyle::Ghost };
            if ui.button(&label, style) {
                *action = Some(if affordable {
                    HqAction::Open(a)
                } else {
                    HqAction::Toast(format!("資金が足りません（あと {}）", format_yen(area.open_cost - world.cash)))
                });
            }
        });
    }
}

fn materials_section(ui: &mut Ui, world: &mut World, action: &mut Option<HqAction>) {
    let t = ui.theme;
    ui.heading("教材");
    let m = world.materials.clone();
    ui.card(16.0, |ui| {
        ui.set_spacing(8.0);
        let r = ui.alloc(26.0);
        ui.text_in(r, m.name(), t.font_heading, FontWeight::Bold, t.text, Align::Left);
        if m.is_own() {
            ui.text_in(r, &format!("改訂 {} 回", m.revisions), t.font_small, FontWeight::Regular, t.text_dim, Align::Right);
        }
        let r = ui.alloc(250.0);
        let values: Vec<f32> = m.axes.iter().map(|v| *v as f32).collect();
        let color = if m.is_own() { t.accent } else { t.text_dim };
        ui.radar(r.x + r.w * 0.5, r.y + 128.0, 86.0, &values, &AXIS_LABELS, color);
        ui.kv_row("教材原価（生徒 1 人・月）", &format_yen(m.cost_per_student_monthly()), None);

        if !m.is_own() {
            ui.label_ex(
                "市販教材は安定していますが、他塾と同じで差別化できません。自社教材を作ると原価が下がり、5 つの軸を自由に鍛えられます。",
                t.font_small,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
            let label = format!("自社教材を立ち上げる（{}）", format_yen(materials::LAUNCH_COST));
            if ui.button(&label, ButtonStyle::Primary) {
                *action = Some(match world.launch_own_materials() {
                    Ok(()) => HqAction::Toast("自社教材の開発チームを立ち上げました".into()),
                    Err(e) => HqAction::Toast(e),
                });
            }
            return;
        }

        if let Some(p) = m.project {
            let done = PROJECT_WEEKS - p.weeks_left;
            let dir = if p.direction < 0 { "を下げる" } else { "を強化" };
            ui.kv_row("開発中", &format!("{}{dir}", AXIS_LABELS[p.axis]), Some(t.accent));
            let r = ui.alloc(14.0);
            ui.meter_at(r, done as f32 / PROJECT_WEEKS as f32, t.accent);
            ui.caption(&format!("あと {} 週・{}/週", p.weeks_left, format_yen(p.weekly_cost)));
            if ui.button("開発を中止する", ButtonStyle::Ghost) {
                world.cancel_material_project();
            }
        } else {
            ui.caption(&format!("軸を選んで改訂します（{PROJECT_WEEKS} 週）。高い軸ほど費用がかかります。"));
        }
        ui.separator();
        for axis in 0..AXIS_COUNT {
            let r = ui.alloc(22.0);
            ui.text_in(r, AXIS_LABELS[axis], t.font_body, FontWeight::Bold, t.text, Align::Left);
            ui.text_in(r, &format!("{:.0}", m.axes[axis] * 100.0), t.font_body, FontWeight::Bold, t.accent, Align::Right);
            let r = ui.alloc(12.0);
            ui.meter_at(r, m.axes[axis] as f32, t.accent);
            ui.label_ex(AXIS_NOTES[axis], t.font_small, FontWeight::Regular, t.text_dim, Align::Left);
            if m.project.is_none() {
                let cost = format_yen(m.project_weekly_cost(axis));
                let r = ui.alloc(36.0);
                let up_enabled = m.can_start_project(axis, 1);
                let down = axis == AXIS_DIFFICULTY;
                let bw = if down { r.w * 0.5 - 4.0 } else { r.w };
                let up_r = Rect::new(r.x, r.y, bw, r.h);
                let style = if up_enabled { ButtonStyle::Secondary } else { ButtonStyle::Ghost };
                if ui.button_at(up_r, &format!("強化する（{cost}/週）"), style) && up_enabled {
                    world.start_material_project(axis, 1);
                }
                if down {
                    let dr = Rect::new(r.x + bw + 8.0, r.y, bw, r.h);
                    if ui.button_at(dr, "やさしくする", ButtonStyle::Ghost) && m.can_start_project(axis, -1) {
                        world.start_material_project(axis, -1);
                    }
                }
            }
        }
    });
}
