//! モーダル：判断イベント・合格発表・確認。

use juku_render::{FontWeight, Rect};
use juku_sim::{Choice, GameEvent, World};
use juku_ui::{Align, ButtonStyle, Ui};

use crate::art;
use crate::fmt::pct;

pub fn event(ui: &mut Ui, ev: &GameEvent) -> Option<Choice> {
    let t = ui.theme;
    ui.modal(460.0, |ui| {
        ui.caption("判断が必要です");
        ui.label_ex(&ev.title, t.font_title, FontWeight::Bold, t.text, Align::Left);
        ui.space(4.0);
        ui.label(&ev.body);
        ui.space(8.0);
        let mut out = None;
        if ui.button(&format!("A：{}", ev.choice_a), ButtonStyle::Ghost) {
            out = Some(Choice::A);
        }
        if ui.button(&format!("B：{}", ev.choice_b), ButtonStyle::Ghost) {
            out = Some(Choice::B);
        }
        ui.caption("あなたの判断は、短期の数字と長期の信頼の両方に影響します。");
        out
    })
}

/// 合格発表。閉じたら true。
pub fn exam(ui: &mut Ui, world: &World, idx: usize) -> bool {
    let t = ui.theme;
    let Some(e) = world.exam_results.get(idx) else { return true };
    ui.modal(480.0, |ui| {
        let r = ui.alloc(40.0);
        art::sakura(ui, r.x + 16.0, r.y + 20.0, 14.0);
        art::sakura(ui, r.right() - 16.0, r.y + 20.0, 14.0);
        ui.text_in(
            r,
            &format!("{}年度 合格発表", e.school_year),
            t.font_title + 2.0,
            FontWeight::Bold,
            t.text,
            Align::Center,
        );

        let r = ui.alloc(64.0);
        let third = r.w / 3.0;
        for (k, (label, value)) in [
            ("受験者", format!("{}人", e.candidates)),
            ("合格件数", format!("{}件", e.total_passes())),
            ("第一志望", pct(e.first_choice_rate())),
        ]
        .iter()
        .enumerate()
        {
            let cell = Rect::new(r.x + third * k as f32, r.y, third, r.h);
            ui.text_in(
                Rect::new(cell.x, cell.y, cell.w, 18.0),
                label,
                t.font_small,
                FontWeight::Regular,
                t.text_dim,
                Align::Center,
            );
            ui.text_in(
                Rect::new(cell.x, cell.y + 20.0, cell.w, 40.0),
                value,
                26.0,
                FontWeight::Bold,
                t.star,
                Align::Center,
            );
        }

        ui.separator();
        for tally in e.tallies.iter().take(8) {
            let s = &world.schools[tally.school];
            let r = ui.alloc(24.0);
            ui.text_in(
                r,
                &format!("{}（{}）", s.name, s.tier.label()),
                t.font_body,
                FontWeight::Regular,
                t.text,
                Align::Left,
            );
            let c = if tally.passes > 0 { t.positive } else { t.text_dim };
            ui.text_in(
                r,
                &format!("{} / {} 名", tally.passes, tally.takers),
                t.font_body,
                FontWeight::Bold,
                c,
                Align::Right,
            );
        }
        if !e.highlights.is_empty() {
            ui.separator();
            for h in &e.highlights {
                ui.label_ex(&format!("★ {h}"), t.font_small + 1.0, FontWeight::Regular, t.text, Align::Left);
            }
        }
        ui.separator();
        ui.kv_row("ブランド力", &format!("{} → {}", pct(e.brand_before), pct(e.brand_after)), Some(t.accent));
        if e.padded {
            ui.caption("※ 特待生の多数受験で合格件数を上積みしました。信頼が下がっています。");
        }
        ui.caption("6 年生は卒業し、4・5 年生が進級しました。新学年の生徒募集が始まります。");
        ui.button("新年度へ", ButtonStyle::Primary)
    })
}

/// 確認ダイアログ。Some(true) = 実行、Some(false) = やめる。
pub fn confirm(ui: &mut Ui, title: &str, body: &str, ok: &str) -> Option<bool> {
    let t = ui.theme;
    ui.modal(420.0, |ui| {
        ui.label_ex(title, t.font_title, FontWeight::Bold, t.text, Align::Left);
        ui.label(body);
        let mut out = None;
        if ui.button(ok, ButtonStyle::Primary) {
            out = Some(true);
        }
        if ui.button("やめる", ButtonStyle::Ghost) {
            out = Some(false);
        }
        out
    })
}
