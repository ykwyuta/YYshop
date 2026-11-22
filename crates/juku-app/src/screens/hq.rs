//! 本部タブ：経営（教室・開校・買収）／財務（融資・物件）／人材・教材（スター講師・自社教材）／その他。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::materials::{self, AXIS_COUNT, AXIS_DIFFICULTY, AXIS_LABELS, AXIS_NOTES, PROJECT_WEEKS};
use juku_sim::{Calendar, Season, TeacherKind, World};
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
    /// キャッシュを削除して再読み込み（Web）
    ForceReload,
    /// 確認してから実行する操作
    Confirm(crate::Confirm),
    /// スター講師への声かけ（競合, 講師）
    Star(usize, usize),
}

pub const SECTIONS: [&str; 4] = ["経営", "財務", "人材・教材", "その他"];

pub fn draw(ui: &mut Ui, world: &mut World, is_web: bool, section: &mut usize) -> Option<HqAction> {
    let mut action = None;
    if let Some(i) = ui.segmented(&SECTIONS, *section) {
        *section = i;
    }
    match *section {
        0 => {
            classrooms(ui, world, &mut action);
            opening(ui, world, &mut action);
            acquisitions(ui, world, &mut action);
        }
        1 => {
            finance(ui, world, &mut action);
            properties(ui, world, &mut action);
        }
        2 => {
            stars(ui, world, &mut action);
            materials_section(ui, world, &mut action);
        }
        _ => others(ui, world, is_web, &mut action),
    }
    action
}

fn others(ui: &mut Ui, world: &mut World, is_web: bool, out: &mut Option<HqAction>) {
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
    help(ui, is_web, &mut action);
    if action.is_some() {
        *out = action;
    }
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
        ui.caption("画面上部の教室名の左右にある矢印で、教室を切り替えられます。教室が増えたら、教室長への業務委任を活用しましょう。");
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

fn help(ui: &mut Ui, is_web: bool, action: &mut Option<HqAction>) {
    let t = ui.theme;
    ui.heading("ヘルプ");
    ui.card(16.0, |ui| {
        ui.set_spacing(8.0);
        ui.label_ex("遊び方の基本", t.font_body, FontWeight::Bold, t.text, Align::Left);
        for tip in [
            "・画面右上の「次の週へ」で 1 週間進みます。進めるたびに自動で保存されます。",
            "・講師の充足率が 100% を下回ると授業の質が落ち、講師が辞めていきます。教室タブで採用しましょう。",
            "・毎年 2 月 1 日の入試結果が会社のブランド力になり、翌年の集客を左右します。",
            "・資金が貯まったら、この本部タブから別のエリアに教室を開けます。教室が増えたら教室長に業務を任せましょう。",
            "・自社教材を作ると 5 つの軸を鍛えられます。入試対応は毎年古くなるので改訂を続けましょう。",
        ] {
            ui.label_ex(tip, t.font_small + 1.0, FontWeight::Regular, t.text, Align::Left);
        }
    });
    ui.card(16.0, |ui| {
        ui.set_spacing(8.0);
        ui.label_ex("表示がおかしいとき", t.font_body, FontWeight::Bold, t.text, Align::Left);
        if is_web {
            ui.label_ex(
                "アップデート後に古い画面のままになる・文字や画像が崩れるときは、ブラウザのキャッシュを削除して再読み込みしてください。セーブデータは消えません。",
                t.font_small + 1.0,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
            if ui.button("キャッシュを削除して再読み込み", ButtonStyle::Secondary) {
                *action = Some(HqAction::ForceReload);
            }
        } else {
            ui.label_ex(
                "Windows 版ではキャッシュの削除は不要です。表示がおかしいときは、最新版の exe に置き換えてください。",
                t.font_small + 1.0,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
        }
        ui.caption(&format!("バージョン {}", env!("CARGO_PKG_VERSION")));
    });
}

fn acquisitions(ui: &mut Ui, world: &World, action: &mut Option<HqAction>) {
    let t = ui.theme;
    ui.heading("塾の買収");
    let targets = world.acquisition_targets();
    if targets.is_empty() {
        ui.caption(
            "いま売りに出ている塾はありません。個人塾や中小塾は、後継者不足や経営難で売りに出ることがあります。",
        );
        return;
    }
    ui.caption("買収すると生徒と講師を引き継げます。経営が変わると 1〜2 割の生徒は辞めてしまいます。");
    for comp in targets {
        let c = &world.competitors[comp];
        let price = world.acquisition_price(comp);
        ui.card(16.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(26.0);
            ui.text_in(r, &c.name, t.font_heading, FontWeight::Bold, t.text, Align::Left);
            let w = ui.measure(&c.name, t.font_heading, FontWeight::Bold);
            ui.badge_at(r.x + w + 10.0, r.y + 2.0, c.style.label(), t.series[2]);
            for b in &c.branches {
                let area = &world.areas[b.area];
                let ours = world.classrooms.iter().find(|x| x.area == b.area).map(|x| x.name.clone());
                let dest = match ours {
                    Some(n) => format!("→ {n}に統合"),
                    None => format!("→ {}校として開校", area.name),
                };
                ui.kv_row(&format!("{}・在籍 {} 人", area.name, b.total()), &dest, None);
            }
            ui.kv_row("授業の質", &format!("{:.0}", c.quality * 100.0), None);
            let affordable = world.cash >= price;
            let style = if affordable { ButtonStyle::Primary } else { ButtonStyle::Ghost };
            if ui.button(&format!("買収する（{}）", format_yen(price)), style) {
                *action = Some(if affordable {
                    HqAction::Confirm(crate::Confirm::Acquire(comp))
                } else {
                    HqAction::Toast(format!(
                        "資金が足りません（あと {}）。財務から融資を受けられます",
                        format_yen(price - world.cash)
                    ))
                });
            }
        });
    }
}

fn finance(ui: &mut Ui, world: &mut World, action: &mut Option<HqAction>) {
    let t = ui.theme;
    ui.heading("銀行融資");
    let limit = world.credit_limit();
    let rate = world.loan_rate();
    ui.card(16.0, |ui| {
        ui.set_spacing(8.0);
        ui.kv_row("会社の資金", &format_yen(world.cash), None);
        ui.kv_row("借入残高", &format_yen(world.debt()), Some(if world.debt() > 0 { t.negative } else { t.text }));
        ui.kv_row("融資枠（あと借りられる額）", &format_yen(limit), Some(t.accent));
        ui.kv_row("いま借りた場合の年利", &format!("{:.1}%", rate * 100.0), None);
        ui.caption("融資枠は、直近の利益・所有物件（担保）・ブランド力・信頼で決まります。元金均等で毎週返済します。");
        for years in [5u32, 10] {
            ui.label_ex(&format!("{years} 年返済で借りる"), t.font_small + 1.0, FontWeight::Bold, t.text, Align::Left);
            let amounts = [5_000_000, 10_000_000, 30_000_000];
            let mut clicked = None;
            ui.columns(3, 8.0, |ui, i| {
                let amount = amounts[i];
                let ok = amount <= limit;
                let style = if ok { ButtonStyle::Secondary } else { ButtonStyle::Ghost };
                let r = ui.alloc(40.0);
                if ui.button_at(r, &crate::fmt::man(amount), style) {
                    clicked = Some((amount, ok));
                }
            });
            if let Some((amount, ok)) = clicked {
                *action = Some(if !ok {
                    HqAction::Toast("融資枠を超えています".into())
                } else {
                    match world.borrow(amount, years) {
                        Ok(()) => {
                            HqAction::Toast(format!("{} を借りました（年利 {:.1}%）", format_yen(amount), rate * 100.0))
                        }
                        Err(e) => HqAction::Toast(e),
                    }
                });
            }
        }
    });
    if world.loans.is_empty() {
        return;
    }
    ui.heading("借入の一覧");
    for l in world.loans.clone() {
        ui.card(14.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(24.0);
            ui.text_in(
                r,
                &format!("{}（{}）", format_yen(l.principal), l.taken.label()),
                t.font_body,
                FontWeight::Bold,
                t.text,
                Align::Left,
            );
            ui.text_in(
                r,
                &format!("年利 {:.1}%", l.rate * 100.0),
                t.font_small,
                FontWeight::Regular,
                t.text_dim,
                Align::Right,
            );
            ui.kv_row("残高", &format_yen(l.balance), Some(t.negative));
            ui.kv_row("毎週の返済", &format!("{}（あと {} 週）", format_yen(l.weekly_payment()), l.weeks_left), None);
            let r = ui.alloc(14.0);
            ui.meter_at(r, 1.0 - l.balance as f32 / l.principal as f32, t.positive);
            if ui.button(&format!("繰り上げ返済する（{}）", format_yen(l.balance)), ButtonStyle::Ghost) {
                *action = Some(HqAction::Confirm(crate::Confirm::Repay(l.id)));
            }
        });
    }
}

fn properties(ui: &mut Ui, world: &World, action: &mut Option<HqAction>) {
    let t = ui.theme;
    ui.heading("物件");
    ui.caption("教室の物件を買うと家賃がなくなり、代わりに維持費（評価額の年 1.2%）がかかります。地価は毎年 2% 上がり、融資の担保にもなります。");
    for (ci, c) in world.classrooms.iter().enumerate() {
        let area = &world.areas[c.area];
        ui.card(16.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(24.0);
            ui.text_in(r, &c.name, t.font_body, FontWeight::Bold, t.text, Align::Left);
            let (badge, color) = if c.owned { ("自社物件", t.positive) } else { ("賃貸", t.text_dim) };
            ui.text_in(r, badge, t.font_small, FontWeight::Bold, color, Align::Right);
            if c.owned {
                ui.kv_row("評価額", &format_yen(c.property_value), Some(t.accent));
                ui.kv_row("維持費", &format!("{}/週", format_yen(c.weekly_property_cost(area))), None);
                if ui.button("売却する（手数料 5%）", ButtonStyle::Ghost) {
                    *action = Some(HqAction::Confirm(crate::Confirm::SellProperty(ci)));
                }
            } else {
                let price = area.property_price;
                let upkeep = (price as f64 * juku_sim::classroom::PROPERTY_UPKEEP_RATE / 52.0) as i64;
                ui.kv_row("家賃", &format!("{}/週", format_yen(area.rent)), None);
                ui.kv_row("購入後の維持費", &format!("{}/週", format_yen(upkeep)), None);
                let years = price as f64 / ((area.rent - upkeep).max(1) as f64 * 52.0);
                ui.caption(&format!("家賃との差額で約 {years:.0} 年で元が取れます。"));
                let affordable = world.cash >= price;
                let style = if affordable { ButtonStyle::Secondary } else { ButtonStyle::Ghost };
                if ui.button(&format!("購入する（{}）", format_yen(price)), style) {
                    *action = Some(if affordable {
                        HqAction::Confirm(crate::Confirm::BuyProperty(ci))
                    } else {
                        HqAction::Toast(format!("資金が足りません（あと {}）", format_yen(price - world.cash)))
                    });
                }
            }
        });
    }
}

fn stars(ui: &mut Ui, world: &World, action: &mut Option<HqAction>) {
    let t = ui.theme;
    ui.heading("スター講師の引き抜き");
    ui.caption("競合塾の看板講師に声をかけます。成功すると集客力が上がり、同じエリアの生徒も一部ついてきます。相手の塾は弱くなりますが、恨まれて仕返しの引き抜きが増えます。");
    let targets = world.star_targets();
    if targets.is_empty() {
        ui.caption("いま声をかけられるスター講師はいません。");
    }
    for (comp, si) in targets {
        let c = &world.competitors[comp];
        let star = &c.stars[si];
        let color = Color::rgb(c.color[0], c.color[1], c.color[2]);
        ui.card(14.0, |ui| {
            ui.set_spacing(6.0);
            let r = ui.alloc(24.0);
            ui.text_in(r, &format!("{} 先生", star.name), t.font_body, FontWeight::Bold, t.text, Align::Left);
            ui.text_in(r, &c.name, t.font_small, FontWeight::Bold, color, Align::Right);
            ui.kv_row("授業力 / 面倒見", &format!("{} / {}", star.skill, star.care), None);
            ui.kv_row("いまの年俸", &format_yen(star.pay), None);
            if c.offer_cooldown > 0 {
                ui.caption(&format!("{}とは交渉したばかりです（あと {} 週）", c.name, c.offer_cooldown));
            } else if ui.button("声をかける", ButtonStyle::Secondary) {
                *action = Some(HqAction::Star(comp, si));
            }
        });
    }
    let ours: Vec<_> = world
        .classrooms
        .iter()
        .flat_map(|c| c.teachers.iter().filter(|t| t.star).map(move |t| (c.name.clone(), t)))
        .collect();
    if !ours.is_empty() {
        ui.heading("自塾のスター講師");
        ui.card(14.0, |ui| {
            ui.set_spacing(4.0);
            for (room, s) in ours {
                let kind = if s.kind == TeacherKind::Pro { "" } else { "（学生）" };
                ui.kv_row(&format!("{}{kind}", s.name), &format!("{room}・授業力 {}", s.skill), Some(t.star));
            }
        });
    }
}
