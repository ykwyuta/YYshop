//! 年度タブ：年間カレンダー・過去の入試結果・セーブ。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::{Calendar, Season, World};
use juku_ui::{Align, ButtonStyle, Ui};

use crate::fmt::pct;

pub enum YearAction {
    ShowExam(usize),
    NewGame,
    Save,
}

pub fn draw(ui: &mut Ui, world: &World) -> Option<YearAction> {
    let t = ui.theme;
    let mut action = None;

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
        let seniors = world.students_in_grade(6);
        ui.kv_row("受験予定の 6 年生", &format!("{seniors} 人"), None);
        let avg6 = if seniors > 0 {
            world.students.iter().filter(|s| s.grade == 6).map(|s| s.deviation).sum::<f64>() / seniors as f64
        } else {
            0.0
        };
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
                action = Some(YearAction::ShowExam(i));
            }
        });
    }

    ui.heading("データ");
    ui.card(16.0, |ui| {
        ui.set_spacing(10.0);
        ui.caption("週を進めるたびに自動で保存されます。");
        if ui.button("今すぐ保存", ButtonStyle::Ghost) {
            action = Some(YearAction::Save);
        }
        if ui.button("新しいゲームを始める", ButtonStyle::Secondary) {
            action = Some(YearAction::NewGame);
        }
    });
    action
}
