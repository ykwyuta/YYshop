//! モーダル：判断イベント・合格発表・確認。

use juku_render::{FontWeight, Rect};
use juku_sim::{Choice, GameEvent, OfferLevel, World};
use juku_ui::{Align, ButtonStyle, Id, Ui};

use crate::art;
use crate::fmt::pct;

pub fn event(ui: &mut Ui, ev: &GameEvent, classroom: Option<&str>) -> Option<Choice> {
    let t = ui.theme;
    ui.modal(Id::new("modal-event"), 460.0, |ui| {
        match classroom {
            Some(name) => ui.caption(&format!("判断が必要です・{name}")),
            None => ui.caption("判断が必要です・本部"),
        };
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
    ui.modal(Id::new("modal-exam").with(idx as u64), 480.0, |ui| {
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
        if world.classrooms.len() > 1 {
            ui.separator();
            let mut names: Vec<&str> = e.entries.iter().map(|x| x.classroom.as_str()).collect();
            names.dedup();
            names.sort();
            names.dedup();
            for name in names {
                let passes = e.entries.iter().filter(|x| x.classroom == name && x.passed).count();
                let first = e.entries.iter().filter(|x| x.classroom == name && x.first_choice).count();
                let first_pass = e.entries.iter().filter(|x| x.classroom == name && x.first_choice && x.passed).count();
                ui.kv_row(name, &format!("合格 {passes} 件・第一志望 {first_pass}/{first}"), None);
            }
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
    ui.modal(Id::new("modal-confirm"), 420.0, |ui| {
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

/// 開校：教室長を選ぶ。Some(Some(i)) = 候補 i で開校、Some(None) = やめる。
pub fn open_classroom(ui: &mut Ui, world: &World, area: usize) -> Option<Option<usize>> {
    let t = ui.theme;
    let a = &world.areas[area];
    ui.modal(Id::new("modal-open").with(area as u64), 480.0, |ui| {
        ui.caption("新しい教室を開く");
        ui.label_ex(&format!("{}校", a.name), t.font_title, FontWeight::Bold, t.text, Align::Left);
        ui.kv_row("開校費", &crate::fmt::format_yen(a.open_cost), Some(t.accent));
        ui.kv_row("家賃", &format!("{}/週", crate::fmt::format_yen(a.rent)), None);
        ui.label("この教室を任せる教室長を選んでください。");
        let mut out = None;
        for (i, m) in world.manager_candidates.iter().enumerate() {
            ui.card(12.0, |ui| {
                ui.set_spacing(4.0);
                let r = ui.alloc(24.0);
                ui.text_in(r, &m.name, t.font_body, FontWeight::Bold, t.text, Align::Left);
                ui.text_in(
                    r,
                    &format!("年俸 {}", crate::fmt::format_yen(m.salary)),
                    t.font_small,
                    FontWeight::Regular,
                    t.text_dim,
                    Align::Right,
                );
                for (label, v) in
                    [("講師管理", m.staffing), ("指導", m.teaching), ("営業", m.sales), ("倫理", m.ethics)]
                {
                    let r = ui.alloc(16.0);
                    ui.text_in(
                        Rect::new(r.x, r.y, 64.0, 16.0),
                        label,
                        t.font_small,
                        FontWeight::Regular,
                        t.text_dim,
                        Align::Left,
                    );
                    ui.meter_at(
                        Rect::new(r.x + 64.0, r.y + 3.0, r.w - 64.0 - 30.0, 10.0),
                        v as f32 / 100.0,
                        t.positive,
                    );
                    ui.text_in(r, &v.to_string(), t.font_small, FontWeight::Bold, t.text, Align::Right);
                }
                ui.space(4.0);
                if ui.button("この人に任せて開校する", ButtonStyle::Primary) {
                    out = Some(Some(i));
                }
            });
        }
        if ui.button("やめる", ButtonStyle::Ghost) {
            out = Some(None);
        }
        out
    })
}

pub enum StarChoice {
    Classroom(usize),
    Offer(OfferLevel),
    Cancel,
}

/// スター講師に声をかける：配属先と条件を選ぶ。
pub fn star_offer(ui: &mut Ui, world: &World, comp: usize, star: usize, classroom: usize) -> Option<StarChoice> {
    let t = ui.theme;
    let c = &world.competitors[comp];
    let Some(s) = c.stars.get(star) else { return Some(StarChoice::Cancel) };
    let n = world.classrooms.len();
    let classroom = classroom.min(n - 1);
    ui.modal(Id::new("modal-star"), 480.0, |ui| {
        ui.caption(&format!("スター講師の引き抜き・{}", c.name));
        ui.label_ex(&format!("{} 先生", s.name), t.font_title, FontWeight::Bold, t.text, Align::Left);
        ui.kv_row("授業力 / 面倒見", &format!("{} / {}", s.skill, s.care), None);
        ui.kv_row("いまの年俸", &crate::fmt::format_yen(s.pay), None);
        let mut out = None;
        let r = ui.alloc(40.0);
        ui.text_in(r, "配属先", t.font_body, FontWeight::Regular, t.text_dim, Align::Left);
        let br = Rect::new(r.x + r.w * 0.35, r.y, r.w * 0.65, r.h);
        let label = if n > 1 {
            format!("{}（タップで変更）", world.classrooms[classroom].name)
        } else {
            world.classrooms[classroom].name.clone()
        };
        if ui.button_at(br, &label, ButtonStyle::Ghost) && n > 1 {
            out = Some(StarChoice::Classroom((classroom + 1) % n));
        }
        for level in OfferLevel::ALL {
            let (salary, bonus) = world.star_offer_terms(comp, star, level);
            let chance = world.star_offer_chance(comp, level);
            ui.card(12.0, |ui| {
                ui.set_spacing(4.0);
                let r = ui.alloc(24.0);
                ui.text_in(r, level.label(), t.font_body, FontWeight::Bold, t.text, Align::Left);
                ui.text_in(
                    r,
                    &format!("成功率 {:.0}%", chance * 100.0),
                    t.font_body,
                    FontWeight::Bold,
                    t.accent,
                    Align::Right,
                );
                ui.kv_row("年俸", &crate::fmt::format_yen(salary), None);
                ui.kv_row("契約金（成功時のみ）", &crate::fmt::format_yen(bonus), None);
                let ok = world.cash >= bonus;
                let style = if ok { ButtonStyle::Primary } else { ButtonStyle::Ghost };
                if ui.button("この条件で声をかける", style) {
                    out = Some(StarChoice::Offer(level));
                }
            });
        }
        ui.caption("断られると 12 週間はこの塾と交渉できません。");
        if ui.button("やめる", ButtonStyle::Ghost) {
            out = Some(StarChoice::Cancel);
        }
        out
    })
}
