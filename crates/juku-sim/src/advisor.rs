//! 秘書の報告。会社と教室の状態を調べて、具体的な問題・競合の動き・チャンスを優先度順に挙げる。
//!
//! 定型のヒントではなく、名前と数字を入れて「いま何が起きていて、どこを見ればよいか」を伝える。

use crate::calendar::{Calendar, Season};
use crate::data;
use crate::format_yen;
use crate::market::{price_score, RivalMoveKind};
use crate::materials::{AXIS_LABELS, AXIS_TREND};
use crate::model::*;
use crate::world::World;

/// 報告がどの画面に関係するか（「見に行く」の行き先）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topic {
    /// 教室タブ（講師・生徒・教室長）
    Classroom,
    /// コースタブ
    Courses,
    /// 集客タブ（シェア・競合・施策）
    Marketing,
    /// 経理タブ
    Accounting,
    /// 本部：経営（開校・買収）
    Expansion,
    /// 本部：財務・上場
    Finance,
    /// 本部：人材・教材
    Materials,
    /// 本部：その他（入試）
    Exams,
}

/// 優先度。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// 好機・提案
    Opportunity = 1,
    /// 気をつけたいこと
    Important = 2,
    /// すぐ手を打つべきこと
    Urgent = 3,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Priority::Urgent => "至急",
            Priority::Important => "注意",
            Priority::Opportunity => "提案",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Advice {
    /// 同じ報告を何度も出さないための識別子（状況が変われば変わる）
    pub key: String,
    pub priority: Priority,
    pub topic: Topic,
    /// 対象の教室（`World::classrooms` の添字）
    pub classroom: Option<usize>,
    pub title: String,
    pub body: String,
}

/// いまの状況についての報告を、優先度の高い順に返す。
pub fn advise(w: &World) -> Vec<Advice> {
    let mut out = Vec::new();
    cash(w, &mut out);
    for ci in 0..w.classrooms.len() {
        staffing(w, ci, &mut out);
        students(w, ci, &mut out);
        profitability(w, ci, &mut out);
        courses(w, ci, &mut out);
        ratings(w, ci, &mut out);
    }
    rivals(w, &mut out);
    seasonal(w, &mut out);
    materials(w, &mut out);
    growth(w, &mut out);
    // 優先度の高い順。同じ優先度なら元の順（重要なものから足している）
    out.sort_by_key(|a| std::cmp::Reverse(a.priority));
    out
}

fn push(
    out: &mut Vec<Advice>,
    key: String,
    priority: Priority,
    topic: Topic,
    classroom: Option<usize>,
    title: String,
    body: String,
) {
    out.push(Advice { key, priority, topic, classroom, title, body });
}

/// `weeks` 週前の日付。
fn weeks_ago(w: &World, weeks: u32) -> crate::calendar::Date {
    let total = w.calendar.school_year as i64 * 52 + w.calendar.week as i64 - weeks as i64;
    Calendar { school_year: total.div_euclid(52) as i32, week: total.rem_euclid(52) as u32 }.date()
}

// ---------------------------------------------------------------------------
// 資金
// ---------------------------------------------------------------------------

fn cash(w: &World, out: &mut Vec<Advice>) {
    if w.cash < 0 {
        let left = crate::world::BANKRUPTCY_WEEKS.saturating_sub(w.negative_weeks);
        let worst = worst_classroom(w);
        let mut body = format!("資金が {} です。このまま {} 週マイナスが続くと倒産します。", format_yen(w.cash), left);
        let limit = w.credit_limit();
        if limit > 0 {
            body.push_str(&format!("銀行の融資枠はあと {} あります。", format_yen(limit)));
        }
        if let Some((name, net)) = worst {
            body.push_str(&format!("いちばん赤字が大きいのは{name}（直近 4 週の平均 {}/週）です。", format_yen(net)));
        }
        push(
            out,
            format!("cash-negative-{}", w.negative_weeks / 4),
            Priority::Urgent,
            Topic::Finance,
            None,
            "資金がマイナスです".into(),
            body,
        );
        return;
    }
    // 直近 4 週の平均で、資金が何週もつか
    // 直近 4 週の資金の増減（物件の売買や借入などの一時的な動きは除く）
    let n = w.history.len();
    if n >= 5 {
        let recent: Vec<i64> =
            (n - 4..n).map(|i| w.history[i].cash - w.history[i - 1].cash - w.history[i].flows.total()).collect();
        let avg = recent.iter().sum::<i64>() / 4;
        if avg < 0 {
            let weeks = w.cash / -avg;
            if weeks < 16 {
                let mut body = format!(
                    "ここ 4 週、資金が平均 {}/週 減っています。このままだと約 {weeks} 週で資金が尽きます。",
                    format_yen(-avg)
                );
                if let Some((name, net)) = worst_classroom(w) {
                    if net < 0 {
                        body.push_str(&format!("赤字がいちばん大きいのは{name}（{}/週）です。", format_yen(net)));
                    }
                }
                push(
                    out,
                    format!("runway-{}", weeks / 4),
                    if weeks < 8 { Priority::Urgent } else { Priority::Important },
                    Topic::Accounting,
                    None,
                    format!("資金はあと約 {weeks} 週分です"),
                    body,
                );
            }
        }
    }
    // 金利の高い借入を返せる
    if let Some(l) = w.loans.iter().filter(|l| l.rate >= 0.045).max_by(|a, b| a.rate.partial_cmp(&b.rate).unwrap()) {
        if w.cash > l.balance * 2 && w.cash - l.balance > 10_000_000 {
            push(
                out,
                format!("repay-{}", l.id),
                Priority::Opportunity,
                Topic::Finance,
                None,
                "金利の高い借入を返せます".into(),
                format!(
                    "{}に借りた {}（年利 {:.1}%）は、残高 {} です。手元資金に余裕があるので、繰り上げ返済すれば利息を節約できます。",
                    l.taken.label(),
                    format_yen(l.principal),
                    l.rate * 100.0,
                    format_yen(l.balance)
                ),
            );
        }
    }
}

/// 直近 4 週の平均純利益がいちばん低い教室。
fn worst_classroom(w: &World) -> Option<(String, Yen)> {
    w.classrooms
        .iter()
        .filter(|c| c.history.len() >= 4)
        .map(|c| (c.name.clone(), c.history.iter().rev().take(4).map(|r| r.pnl.net_profit()).sum::<Yen>() / 4))
        .min_by_key(|(_, net)| *net)
}

// ---------------------------------------------------------------------------
// 教室ごと
// ---------------------------------------------------------------------------

fn staffing(w: &World, ci: usize, out: &mut Vec<Advice>) {
    let c = &w.classrooms[ci];
    if c.students.is_empty() {
        return;
    }
    let stats = w.stats(ci);
    let tired: Vec<&Teacher> = c.teachers.iter().filter(|t| t.fatigue > 0.7).collect();
    if stats.coverage < 0.95 || !tired.is_empty() {
        let short = (stats.needed_hours - stats.available_hours).max(0.0);
        let mut body = if stats.coverage < 0.95 {
            format!(
                "{}の講師が足りません。授業に必要な時間は週 {:.0} 時間ですが、担当できるのは {:.0} 時間（充足率 {:.0}%）で、あと {:.0} 時間分足りません。",
                c.name,
                stats.needed_hours,
                stats.available_hours,
                stats.coverage * 100.0,
                short
            )
        } else {
            format!("{}の講師に疲れがたまっています。", c.name)
        };
        if !tired.is_empty() {
            let names: Vec<String> =
                tired.iter().take(3).map(|t| format!("{}先生（疲労 {:.0}）", t.name, t.fatigue * 100.0)).collect();
            body.push_str(&format!("{} が限界に近く、このままだと辞めてしまいます。", names.join("・")));
        }
        if let Some(best) = c.candidates.iter().max_by_key(|t| t.skill) {
            body.push_str(&format!(
                "採用候補には{}の{}（授業力 {}、週 {:.0} 時間）がいます。",
                best.kind.label(),
                best.name,
                best.skill,
                best.kind.weekly_hours()
            ));
        }
        let urgent = stats.coverage < 0.85 || tired.iter().any(|t| t.fatigue > 0.85);
        push(
            out,
            format!("staff-{}-{}-{}", c.id, (stats.coverage * 10.0) as i32, tired.len()),
            if urgent { Priority::Urgent } else { Priority::Important },
            Topic::Classroom,
            Some(ci),
            format!("{}：講師が足りません", c.name),
            body,
        );
    }
}

fn students(w: &World, ci: usize, out: &mut Vec<Advice>) {
    let c = &w.classrooms[ci];
    let churned: u32 = c.history.iter().rev().take(4).map(|r| r.churned).sum();
    let stressed: Vec<&Student> = c.students.iter().filter(|s| s.stress > 0.7).collect();
    let unmotivated = c.students.iter().filter(|s| s.motivation < 0.35).count();
    let pricey = c
        .students
        .iter()
        .filter(|s| {
            let gi = (s.grade - 4) as usize;
            c.main_fee(s.grade) as f64 / data::REFERENCE_FEES[gi] as f64 > s.budget.tolerance()
        })
        .count();
    if churned >= 3 || stressed.len() >= 5 {
        let mut causes = Vec::new();
        if !stressed.is_empty() {
            causes.push(format!("ストレスが限界に近い生徒が {} 人（宿題レベル {}）", stressed.len(), c.homework));
        }
        if unmotivated > 0 {
            causes.push(format!("意欲が落ちている生徒が {unmotivated} 人"));
        }
        if pricey > 0 {
            causes.push(format!("月謝が家計に重い家庭が {pricey} 世帯"));
        }
        let mut body = if churned >= 3 {
            format!("{}で、この 4 週間に {churned} 人が退塾しました。", c.name)
        } else {
            format!("{}で、退塾につながりそうな兆しがあります。", c.name)
        };
        if !causes.is_empty() {
            body.push_str(&format!("いま、{}。", causes.join("、")));
        }
        if let Some(s) = stressed.iter().max_by(|a, b| a.stress.partial_cmp(&b.stress).unwrap()) {
            body.push_str(&format!(
                "特に{}さん（{}年・ストレス {:.0}）は要注意です。",
                s.name,
                s.grade,
                s.stress * 100.0
            ));
        }
        if c.homework >= 4 && stressed.len() >= 3 {
            body.push_str("宿題の量を見直すと落ち着くかもしれません。");
        }
        push(
            out,
            format!("churn-{}-{}-{}", c.id, churned, stressed.len() / 3),
            if churned >= 5 { Priority::Urgent } else { Priority::Important },
            if c.homework >= 4 { Topic::Courses } else { Topic::Classroom },
            Some(ci),
            format!("{}：退塾が増えています", c.name),
            body,
        );
    }
}

fn profitability(w: &World, ci: usize, out: &mut Vec<Advice>) {
    let c = &w.classrooms[ci];
    if c.history.len() < 26 {
        return;
    }
    let recent: Vec<&crate::report::WeekReport> = c.history.iter().rev().take(13).collect();
    let losing = recent.iter().filter(|r| r.pnl.net_profit() < 0).count();
    if losing < 10 {
        return;
    }
    let avg_net = recent.iter().map(|r| r.pnl.net_profit()).sum::<Yen>() / 13;
    let rev: Yen = recent.iter().map(|r| r.pnl.revenue()).sum();
    // いちばん重い費用
    let mut costs = [("講師人件費", 0i64), ("教室長給与", 0), ("家賃", 0), ("広告宣伝費", 0)];
    for r in &recent {
        costs[0].1 += r.pnl.teacher_cost;
        costs[1].1 += r.pnl.manager_cost;
        costs[2].1 += r.pnl.rent + r.pnl.property_upkeep;
        costs[3].1 += r.pnl.marketing;
    }
    let (name, heavy) = costs.iter().max_by_key(|(_, v)| *v).copied().unwrap();
    let share = if rev > 0 { heavy as f64 / rev as f64 * 100.0 } else { 0.0 };
    let students = c.students.len();
    let mut body = format!(
        "{}はこの 13 週のうち {losing} 週が赤字で、平均 {}/週 です。在籍は {students} 人。費用でいちばん重いのは{name}（売上の {share:.0}%）です。",
        c.name,
        format_yen(avg_net)
    );
    let stats = w.stats(ci);
    if stats.coverage > 1.5 {
        body.push_str(&format!("講師の時間が必要量の {:.0}% あり、余っています。", stats.coverage * 100.0));
    } else if students < 40 {
        body.push_str(
            "生徒数が損益分岐（40 人前後）に届いていません。集客を強めるか、思い切って閉じることも考えましょう。",
        );
    }
    push(
        out,
        format!("loss-{}-{}", c.id, w.calendar.week / 8),
        Priority::Important,
        Topic::Accounting,
        Some(ci),
        format!("{}：赤字が続いています", c.name),
        body,
    );
}

fn courses(w: &World, ci: usize, out: &mut Vec<Advice>) {
    let c = &w.classrooms[ci];
    for (k, course) in c.courses.iter().enumerate() {
        let CourseKind::Main(g) = course.kind else { continue };
        if !course.enabled {
            continue;
        }
        let gi = (g - 4) as usize;
        // 定員間近
        if course.last_enrolled + 3 >= course.capacity {
            push(
                out,
                format!("full-{}-{}", c.id, k),
                Priority::Opportunity,
                Topic::Courses,
                Some(ci),
                format!("{}：{}が定員間近です", c.name, course.name),
                format!(
                    "{}は {}/{} 人です。満員になると入塾希望を断ることになります。月謝 {} は少し上げても生徒は集まりそうです。",
                    course.name,
                    course.last_enrolled,
                    course.capacity,
                    format_yen(course.fee)
                ),
            );
        }
        // 競合との価格差
        let rivals: Vec<(String, Yen)> = w
            .competitors
            .iter()
            .filter(|r| !r.acquired)
            .filter_map(|r| r.branch(c.area).map(|b| (r.name.clone(), b.fees[gi])))
            .collect();
        if rivals.is_empty() {
            continue;
        }
        let avg = rivals.iter().map(|(_, f)| *f).sum::<Yen>() / rivals.len() as Yen;
        let ratio = course.fee as f64 / avg as f64;
        if ratio > 1.3 && price_score(course.fee, data::REFERENCE_FEES[gi]) < 0.3 {
            let cheapest = rivals.iter().min_by_key(|(_, f)| *f).unwrap();
            push(
                out,
                format!("price-high-{}-{}-{}", c.id, k, course.fee / 1000),
                Priority::Important,
                Topic::Courses,
                Some(ci),
                format!("{}：{}の月謝が高めです", c.name, course.name),
                format!(
                    "{}の月謝 {} は、{}の競合の平均 {} より {:.0}% 高くなっています（いちばん安い{}は {}）。月謝が重いと感じる家庭が増えると退塾につながります。",
                    course.name,
                    format_yen(course.fee),
                    w.areas[c.area].name,
                    format_yen(avg),
                    (ratio - 1.0) * 100.0,
                    cheapest.0,
                    format_yen(cheapest.1)
                ),
            );
        }
    }
}

fn ratings(w: &World, ci: usize, out: &mut Vec<Advice>) {
    let c = &w.classrooms[ci];
    if c.history.len() < 8 {
        return;
    }
    let r = c.ratings;
    let axes = [("料金", r.price), ("授業", r.teaching), ("面倒見", r.care), ("安心・安全", r.safety)];
    let (name, v) = axes.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).copied().unwrap();
    if v >= 2.6 {
        return;
    }
    let hint = match name {
        "料金" => "月謝が地域の相場より高いと感じられています。",
        "授業" => "講師の授業力や人数が足りていない可能性があります。",
        "面倒見" => "保護者説明会や自習室、面倒見のよい講師が効きます。",
        _ => "入退室メールや自習室で安心感を高められます。",
    };
    let bad =
        c.reviews.iter().find(|rv| !rv.positive).map(|rv| format!("最近の口コミ：「{}」", rv.text)).unwrap_or_default();
    push(
        out,
        format!("rating-{}-{}", c.id, name),
        Priority::Important,
        Topic::Classroom,
        Some(ci),
        format!("{}：{}の評価が低くなっています", c.name, name),
        format!("{}の口コミで「{name}」が {v:.1} と低くなっています。{hint}{bad}", c.name),
    );
}

// ---------------------------------------------------------------------------
// 競合
// ---------------------------------------------------------------------------

fn rivals(w: &World, out: &mut Vec<Advice>) {
    let cutoff = weeks_ago(w, 6).ordinal();
    for m in w.rival_log.iter().filter(|m| m.date.ordinal() >= cutoff) {
        let Some(ci) = w.classrooms.iter().position(|c| c.area == m.area) else {
            // 自塾のないエリアは、売り出しだけ伝える
            if m.kind == RivalMoveKind::ForSale {
                sale(w, m.competitor, out);
            }
            continue;
        };
        let c = &w.classrooms[ci];
        let rival = &w.competitors[m.competitor];
        let area = &w.areas[m.area].name;
        let share_now = c.last_report().map(|r| r.market_share).unwrap_or(0.0);
        let share_before = c.history.iter().rev().nth(8).map(|r| r.market_share).unwrap_or(share_now);
        let share =
            format!("{}校のシェアは 8 週前の {:.0}% から {:.0}% です。", area, share_before * 100.0, share_now * 100.0);
        let key = format!("rival-{}-{}-{:?}-{}", m.competitor, m.area, m.kind, m.date.ordinal());
        match m.kind {
            RivalMoveKind::PriceCut => {
                let b = rival.branch(m.area);
                let fees = b.map(|b| b.fees).unwrap_or(rival.base_fees);
                body_push(
                    out,
                    key,
                    Priority::Important,
                    Topic::Marketing,
                    ci,
                    format!("{}が{}で値下げしました", rival.name, area),
                    format!(
                        "{}（{}）が{}で月謝を下げました。5 年の月謝は相手が {}、{}は {} です。{}価格だけで張り合うより、授業や面倒見の評価で差をつけるのも手です。",
                        rival.name,
                        rival.style.label(),
                        area,
                        format_yen(fees[1]),
                        c.name,
                        format_yen(c.main_fee(5)),
                        share
                    ),
                );
            }
            RivalMoveKind::Open => body_push(
                out,
                key,
                Priority::Important,
                Topic::Marketing,
                ci,
                format!("{}が{}に進出してきました", rival.name, area),
                format!(
                    "{}（{}・ブランド力 {:.0}%）が{}の目の前に教室を開きました。開校直後は広告が強力です。{}の認知度は {:.0}% です。合格実績チラシや保護者説明会で迎え撃ちましょう。",
                    rival.name,
                    rival.style.label(),
                    rival.brand * 100.0,
                    c.name,
                    c.name,
                    c.awareness * 100.0
                ),
            ),
            RivalMoveKind::Campaign => body_push(
                out,
                key,
                Priority::Opportunity,
                Topic::Marketing,
                ci,
                format!("{}が{}で大規模な広告を始めました", rival.name, area),
                format!("{}が{}で広告を強めています。{}", rival.name, area, share),
            ),
            RivalMoveKind::Poach => {
                let detail = match c.teachers.iter().find(|t| m.text.contains(t.name.as_str())) {
                    Some(t) => {
                        let pay = match t.kind {
                            TeacherKind::Pro => format!("年俸 {}", format_yen(t.pay)),
                            TeacherKind::Student => format!("時給 {}", format_yen(t.pay)),
                        };
                        format!(
                            "{}先生は授業力 {}・{pay}・疲労 {:.0} です。引き抜かれると{}の授業の質が下がり、相手の授業力が上がります。",
                            t.name,
                            t.skill,
                            t.fatigue * 100.0,
                            c.name
                        )
                    }
                    None => format!("その講師はもう{}にいません。{}の講師は今 {} 人です。", c.name, c.name, c.teachers.len()),
                };
                body_push(
                    out,
                    key,
                    Priority::Important,
                    Topic::Classroom,
                    ci,
                    format!("{}が講師の引き抜きを狙っています", rival.name),
                    format!("{}。{detail}", m.text),
                )
            }
            RivalMoveKind::Withdraw => body_push(
                out,
                key,
                Priority::Opportunity,
                Topic::Marketing,
                ci,
                format!("{}が{}から撤退しました", rival.name, area),
                format!("{}。その塾に通っていた家庭が新しい塾を探しています。今が集客のチャンスです。", m.text),
            ),
            RivalMoveKind::ForSale => sale(w, m.competitor, out),
            RivalMoveKind::PriceUp | RivalMoveKind::CareUp => {}
        }
    }
    // 売りに出ている塾（ログが古くても、まだ買えるなら伝える）。
    // 自塾のあるエリアのもの、手が届きそうなものから 2 件まで
    let mut targets = w.acquisition_targets();
    targets.sort_by_key(|&t| {
        let local = w.competitors[t].branches.iter().any(|b| w.classrooms.iter().any(|c| c.area == b.area));
        (!local, w.acquisition_price(t))
    });
    for t in targets.into_iter().take(2) {
        sale(w, t, out);
    }
}

fn body_push(out: &mut Vec<Advice>, key: String, p: Priority, topic: Topic, ci: usize, title: String, body: String) {
    push(out, key, p, topic, Some(ci), title, body);
}

fn sale(w: &World, comp: usize, out: &mut Vec<Advice>) {
    let key = format!("sale-{comp}");
    let shown = out.iter().filter(|a| a.key.starts_with("sale-")).count();
    if shown >= 2 || out.iter().any(|a| a.key == key) || !w.acquisition_targets().contains(&comp) {
        return;
    }
    let c = &w.competitors[comp];
    let price = w.acquisition_price(comp);
    let places: Vec<String> = c
        .branches
        .iter()
        .map(|b| {
            let ours = w.classrooms.iter().find(|x| x.area == b.area);
            match ours {
                Some(o) => format!("{}（在籍 {} 人 → {}に統合）", w.areas[b.area].name, b.total(), o.name),
                None => format!("{}（在籍 {} 人 → 新しい教室に）", w.areas[b.area].name, b.total()),
            }
        })
        .collect();
    let money = if w.cash >= price {
        "手元の資金で買えます。".to_string()
    } else {
        format!("あと {} 足りません（融資枠は {}）。", format_yen(price - w.cash), format_yen(w.credit_limit()))
    };
    push(
        out,
        key,
        Priority::Opportunity,
        Topic::Expansion,
        None,
        format!("{}が売りに出ています", c.name),
        format!(
            "{}（{}）が {} で買えます。{}。開校するより早く生徒と講師が手に入ります。{money}",
            c.name,
            c.style.label(),
            format_yen(price),
            places.join("、")
        ),
    );
}

// ---------------------------------------------------------------------------
// 季節
// ---------------------------------------------------------------------------

fn seasonal(w: &World, out: &mut Vec<Advice>) {
    let next = Calendar { school_year: w.calendar.school_year, week: (w.calendar.week + 4).min(51) };
    let year = w.calendar.school_year;
    // 夏期講習の 4 週前
    if next.season() == Season::Summer && w.calendar.season() != Season::Summer {
        let fee = w.classrooms.first().and_then(|c| c.course(CourseKind::Summer)).map(|c| c.fee).unwrap_or(0);
        let off: Vec<&str> = w
            .classrooms
            .iter()
            .filter(|c| !c.course(CourseKind::Summer).map(|k| k.enabled).unwrap_or(false))
            .map(|c| c.name.as_str())
            .collect();
        let mut body = format!(
            "あと約 4 週で夏期講習です。講習期間は講師の担当時間が増え、新しい生徒が入る大きなチャンスです。受講料は {} です。",
            format_yen(fee)
        );
        if !off.is_empty() {
            body.push_str(&format!("{}は夏期講習が休講になっています。", off.join("・")));
        }
        body.push_str("講師の数と、広告の準備を確認しておきましょう。");
        push(
            out,
            format!("summer-{year}"),
            Priority::Important,
            Topic::Courses,
            Some(0),
            "夏期講習が近づいています".into(),
            body,
        );
    }
    // 入試の 6 週前
    if w.calendar.week >= 46 {
        let weeks = 52 - w.calendar.week;
        let seniors: Vec<&Student> =
            w.classrooms.iter().flat_map(|c| c.students.iter()).filter(|s| s.grade == 6).collect();
        if !seniors.is_empty() {
            let in_range = seniors.iter().filter(|s| w.schools[s.target_school].deviation - s.deviation <= 2.0).count();
            let far: Vec<String> = seniors
                .iter()
                .filter(|s| w.schools[s.target_school].deviation - s.deviation > 6.0)
                .take(3)
                .map(|s| {
                    format!(
                        "{}さん（{:.0} → {} {:.0}）",
                        s.name, s.deviation, w.schools[s.target_school].name, w.schools[s.target_school].deviation
                    )
                })
                .collect();
            let mut body = format!(
                "2 月 1 日の入試まであと {weeks} 週です。6 年生 {} 人のうち、第一志望が合格圏内（偏差値の差 2 以内）なのは {in_range} 人です。",
                seniors.len()
            );
            if !far.is_empty() {
                body.push_str(&format!("まだ距離があるのは {} などです。", far.join("・")));
            }
            push(
                out,
                format!("exam-{year}"),
                Priority::Important,
                Topic::Exams,
                None,
                "入試が近づいています".into(),
                body,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 教材
// ---------------------------------------------------------------------------

fn materials(w: &World, out: &mut Vec<Advice>) {
    let m = &w.materials;
    let students = w.total_students() as i64;
    if !m.is_own() {
        if w.classrooms.len() >= 2 && w.cash > crate::materials::LAUNCH_COST * 3 {
            let saving =
                students * (crate::materials::COMMERCIAL_COST_MONTHLY - crate::materials::OWN_COST_MONTHLY) * 12;
            push(
                out,
                "own-materials".into(),
                Priority::Opportunity,
                Topic::Materials,
                None,
                "自社教材を作りませんか".into(),
                format!(
                    "いまの生徒 {students} 人なら、自社教材に切り替えるだけで教材費が年 {} ほど浮きます（立ち上げ費 {}）。5 つの軸を鍛えれば合格率も上げられます。",
                    format_yen(saving),
                    format_yen(crate::materials::LAUNCH_COST)
                ),
            );
        }
        return;
    }
    if m.axes[AXIS_TREND] < 0.45 && m.project.is_none() {
        push(
            out,
            format!("trend-{}", w.calendar.school_year),
            Priority::Important,
            Topic::Materials,
            None,
            "教材の「入試対応」が古くなっています".into(),
            format!(
                "自社教材の{}が {:.0} まで下がっています。入試の合格率に直結するので、次の入試までに改訂しておきましょう。",
                AXIS_LABELS[AXIS_TREND],
                m.axes[AXIS_TREND] * 100.0
            ),
        );
    }
    let s = &m.sales;
    if !s.active && m.quality_score() >= 0.6 {
        push(
            out,
            "start-sales".into(),
            Priority::Opportunity,
            Topic::Materials,
            None,
            "教材を外販できる水準です".into(),
            format!(
                "自社教材の評価が {:.0} まで上がりました。他の塾に月額で提供すれば、新しい収入源になります。",
                m.quality_score() * 100.0
            ),
        );
    }
    if s.active && s.contracts > 10 {
        let recent: Vec<i64> = w.history.iter().rev().take(8).map(|h| h.materials_sales).collect();
        if recent.len() == 8 && recent[0] < recent[7] * 9 / 10 {
            push(
                out,
                format!("sales-down-{}", s.contracts / 10),
                Priority::Important,
                Topic::Materials,
                None,
                "教材の外販で解約が増えています".into(),
                format!(
                    "外販の契約は {} 塾で、8 週前より売上が {} 減っています。教材の評価は {:.0} です。改訂で質を保ちましょう。",
                    s.contracts,
                    format_yen(recent[7] - recent[0]),
                    m.quality_score() * 100.0
                ),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 成長・上場
// ---------------------------------------------------------------------------

fn growth(w: &World, out: &mut Vec<Advice>) {
    // 開校できるエリア
    let mut candidates: Vec<usize> =
        w.open_areas().into_iter().filter(|a| w.cash > w.areas[*a].open_cost * 2).collect();
    candidates.sort_by(|a, b| {
        let score = |x: usize| {
            let rivals: u32 = w.competitors.iter().filter(|c| !c.acquired).map(|c| c.students_in_area(x)).sum();
            w.areas[x].pool * 1000.0 / (rivals as f64 + 100.0)
        };
        score(*b).partial_cmp(&score(*a)).unwrap()
    });
    if let Some(&a) = candidates.first() {
        let area = &w.areas[a];
        let rivals: Vec<String> = w
            .competitors
            .iter()
            .filter(|c| !c.acquired)
            .filter_map(|c| c.branch(a).map(|b| format!("{}（{} 人）", c.name, b.total())))
            .collect();
        push(
            out,
            format!("open-{a}"),
            Priority::Opportunity,
            Topic::Expansion,
            None,
            format!("{}エリアへの出店を検討できます", area.name),
            format!(
                "資金が {} あり、{}エリア（開校費 {}・家賃 {}/週・市場規模 ×{:.2}）に出店できます。競合は {} です。新しい教室は黒字化まで 2〜3 年かかるので、資金に余裕のあるうちに。",
                format_yen(w.cash),
                area.name,
                format_yen(area.open_cost),
                format_yen(area.rent),
                area.pool,
                if rivals.is_empty() { "いません".into() } else { rivals.join("・") }
            ),
        );
    }
    // 委任
    if w.classrooms.len() >= 3 {
        let none: Vec<&str> =
            w.classrooms.iter().filter(|c| c.manager.delegations.iter().all(|d| !d)).map(|c| c.name.as_str()).collect();
        if none.len() >= 2 {
            push(
                out,
                format!("delegate-{}", none.len()),
                Priority::Opportunity,
                Topic::Classroom,
                None,
                "教室長に業務を任せませんか".into(),
                format!(
                    "{}では、教室長に業務を何も任せていません。教室が {} つになったので、講師の採用や保護者対応を任せると手が回ります。",
                    none.join("・"),
                    w.classrooms.len()
                ),
            );
        }
    }
    // 上場
    if w.listing.status == crate::listing::ListingStatus::Private {
        let reqs = w.ipo_requirements();
        let missing: Vec<String> =
            reqs.iter().filter(|r| !r.ok).map(|r| format!("{}（{}）", r.label, r.detail)).collect();
        if missing.is_empty() {
            push(
                out,
                "ipo-ready".into(),
                Priority::Opportunity,
                Topic::Finance,
                None,
                "上場の条件をすべて満たしました".into(),
                format!(
                    "上場準備（1 年・総額 {}）を始められます。上場すれば大きな資金を調達でき、都心エリアにも出店できます。",
                    format_yen(crate::listing::IPO_PREP_WEEKLY * crate::listing::IPO_PREP_WEEKS as i64)
                ),
            );
        } else if missing.len() <= 2 && w.classrooms.len() >= 3 {
            push(
                out,
                format!("ipo-near-{}", missing.len()),
                Priority::Opportunity,
                Topic::Finance,
                None,
                "上場が見えてきました".into(),
                format!("上場の条件で残っているのは、{} です。", missing.join("、")),
            );
        }
    }
    // スター講師
    if w.cash > 30_000_000 {
        let best = w
            .star_targets()
            .into_iter()
            .filter(|(c, _)| w.competitors[*c].offer_cooldown == 0)
            .max_by_key(|(c, s)| w.competitors[*c].stars[*s].skill);
        if let Some((comp, si)) = best {
            let c = &w.competitors[comp];
            let star = &c.stars[si];
            let chance = w.star_offer_chance(comp, crate::finance::OfferLevel::Good);
            push(
                out,
                format!("star-{}", star.id),
                Priority::Opportunity,
                Topic::Materials,
                None,
                format!("{}の{}先生を狙えるかもしれません", c.name, star.name),
                format!(
                    "{}の看板講師・{}先生（授業力 {}）は年俸 {} です。好条件で声をかければ成功率は約 {:.0}% です。来てくれれば集客力が上がり、相手は弱くなります。",
                    c.name,
                    star.name,
                    star.skill,
                    format_yen(star.pay),
                    chance * 100.0
                ),
            );
        }
    }
}
