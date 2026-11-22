//! バランス調整用の自動プレイヤー（ボット）と指標の集計。
//!
//! ヘッドレスで何年分も回して、戦略ごとの結果を比べるために使う。
//! テスト（`tests/balance.rs`）とレポート（`examples/balance.rs`）の両方から使う。

use crate::finance::OfferLevel;
use crate::materials::{AXIS_CLARITY, AXIS_COVERAGE, AXIS_PRACTICE, AXIS_TREND};
use crate::model::*;
use crate::report::{Choice, EventKind};
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    /// 何もしない（イベントは B を選ぶ）
    Passive,
    /// 講師が足りなければ採用するだけ
    Standard,
    /// 標準 ＋ 資金があれば開校・買収・自社教材・物件購入、新教室は委任
    Growth,
    /// 成長 ＋ 融資を使って早めに拡大し、スター講師も引き抜く
    Aggressive,
    /// 標準 ＋ 短期の数字を優先（イベントは A、宿題多め、値上げ）
    Greedy,
    /// 標準 ＋ 大幅な値下げ
    Discount,
}

impl Strategy {
    pub const ALL: [Strategy; 6] = [
        Strategy::Passive,
        Strategy::Standard,
        Strategy::Growth,
        Strategy::Aggressive,
        Strategy::Greedy,
        Strategy::Discount,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Strategy::Passive => "放置",
            Strategy::Standard => "標準",
            Strategy::Growth => "成長",
            Strategy::Aggressive => "積極拡大",
            Strategy::Greedy => "短期志向",
            Strategy::Discount => "安売り",
        }
    }
}

/// 年度ごとの記録。
#[derive(Clone, Debug, Default)]
pub struct YearStats {
    pub cash: Yen,
    pub debt: Yen,
    pub valuation: Yen,
    pub net_profit: Yen,
    pub classrooms: usize,
    pub students: u32,
    pub teachers: usize,
    pub brand: f64,
    pub trust: f64,
    pub rating: f64,
    pub first_choice_rate: f64,
    pub passes: u32,
    pub churned: u32,
    pub new_students: u32,
    /// 最初の教室のエリアシェア
    pub share: f64,
    /// 全エリアの総在籍数（自塾＋競合）
    pub market_total: u32,
    /// 資金がマイナスだった週数
    pub negative_weeks: u32,
    pub scandals: u32,
    pub bankrupt: bool,
    pub listed: bool,
    /// 教材外販の契約塾数
    pub sales_contracts: u32,
    /// 直近 52 週の教材外販の売上
    pub sales_revenue: Yen,
    /// 塾長の個人資産（持ち株の評価額）
    pub personal_assets: Yen,
}

/// 1 回分のプレイ結果。
#[derive(Clone, Debug)]
pub struct Run {
    pub years: Vec<YearStats>,
}

impl Run {
    pub fn last(&self) -> &YearStats {
        self.years.last().expect("at least one year")
    }
}

/// `strategy` で `years` 年プレイする。
pub fn play(seed: u64, strategy: Strategy, years: u32) -> Run {
    play_with(seed, strategy, years, |_| {})
}

/// 最初に `setup` で設定を変えてからプレイする（価格や宿題量のスイープ用）。
pub fn play_with(seed: u64, strategy: Strategy, years: u32, setup: impl FnOnce(&mut World)) -> Run {
    let mut w = World::new(seed);
    setup(&mut w);
    let mut stats = Vec::new();
    let mut churned = 0;
    let mut new_students = 0;
    let mut negative = 0;
    let mut scandals = 0;
    if strategy == Strategy::Greedy {
        let c = w.classroom_mut(0);
        c.set_homework(4);
        for i in 0..3 {
            let fee = c.courses[i].fee;
            c.set_course_fee(i, fee * 115 / 100);
        }
    }
    if strategy == Strategy::Discount {
        let c = w.classroom_mut(0);
        for i in 0..3 {
            let fee = c.courses[i].fee;
            c.set_course_fee(i, fee * 70 / 100);
        }
    }
    for week in 0..years * 52 {
        if w.bankrupt {
            // 倒産後は同じ状態のまま年度末の記録だけ続ける
            if week % 52 == 51 {
                let mut last: YearStats = stats.last().cloned().unwrap_or_default();
                last.bankrupt = true;
                last.cash = w.cash;
                last.valuation = w.valuation().min(0);
                last.students = 0;
                last.classrooms = 0;
                last.net_profit = 0;
                stats.push(last);
            }
            continue;
        }
        act_week(&mut w, strategy, week);
        w.step_week();
        if let Some(last) = w.last_week() {
            if last.news.iter().any(|n| n.contains("炎上")) {
                scandals += 1;
            }
        }
        for c in &w.classrooms {
            if let Some(r) = c.last_report() {
                churned += r.churned;
                new_students += r.new_students;
            }
        }
        if w.cash < 0 {
            negative += 1;
        }
        if w.calendar.week == 0 {
            let exam = w.exam_results.last();
            let c0 = &w.classrooms[0];
            let market_total: u32 = w.total_students()
                + w.competitors.iter().filter(|c| !c.acquired).map(|c| c.total_students()).sum::<u32>();
            stats.push(YearStats {
                cash: w.cash,
                debt: w.debt(),
                valuation: w.valuation(),
                net_profit: w.annual_profit(),
                classrooms: w.classrooms.len(),
                students: w.total_students(),
                teachers: w.classrooms.iter().map(|c| c.teachers.len()).sum(),
                brand: w.brand,
                trust: w.trust,
                rating: w.classrooms.iter().map(|c| c.ratings.overall()).sum::<f64>() / w.classrooms.len() as f64,
                first_choice_rate: exam.map(|e| e.first_choice_rate()).unwrap_or(0.0),
                passes: exam.map(|e| e.total_passes()).unwrap_or(0),
                churned,
                new_students,
                share: c0.last_report().map(|r| r.market_share).unwrap_or(0.0),
                market_total,
                negative_weeks: negative,
                scandals,
                bankrupt: w.bankrupt,
                listed: w.listing.is_listed(),
                sales_contracts: w.materials.sales.contracts,
                sales_revenue: w.history.iter().rev().take(52).map(|h| h.materials_sales).sum(),
                personal_assets: w.personal_assets(),
            });
            churned = 0;
            new_students = 0;
            negative = 0;
            scandals = 0;
        }
    }
    Run { years: stats }
}

/// 戦略に従って 1 週分の操作をし、判断待ちのイベントに答える（`step_week` の直前に呼ぶ）。
pub fn act_week(w: &mut World, strategy: Strategy, week: u32) {
    act(w, strategy, week);
    if let Some(ev) = &w.pending_event {
        let choice = match (strategy, ev.kind) {
            (Strategy::Greedy, _) => Choice::A,
            // 引き抜きは引き止める
            (_, EventKind::Poaching { .. }) if strategy != Strategy::Passive => Choice::A,
            _ => Choice::B,
        };
        w.resolve_event(choice);
    }
}

/// 戦略に従って 1 週分の操作をする。
fn act(w: &mut World, strategy: Strategy, week: u32) {
    if strategy == Strategy::Passive {
        return;
    }
    // 講師の補充と整理（全戦略共通）
    let summer = w.calendar.is_summer_course();
    for ci in 0..w.classrooms.len() {
        if w.classrooms[ci].manager.delegated(Delegation::Staffing) {
            continue;
        }
        let coverage = w.stats(ci).coverage;
        if coverage < 1.05 && !w.classrooms[ci].candidates.is_empty() {
            // 社員講師より学生講師を優先（安い）
            let idx = w.classrooms[ci].candidates.iter().position(|t| t.kind == TeacherKind::Student).unwrap_or(0);
            w.classroom_mut(ci).hire_candidate(idx);
        } else if coverage > 1.5 && !summer && w.classrooms[ci].teachers.len() > 2 {
            let weakest = w.classrooms[ci]
                .teachers
                .iter()
                .filter(|t| t.kind == TeacherKind::Student && !t.star)
                .min_by_key(|t| t.skill)
                .map(|t| t.id);
            if let Some(id) = weakest {
                w.classroom_mut(ci).dismiss_teacher(id);
            }
        }
    }
    if !matches!(strategy, Strategy::Growth | Strategy::Aggressive) {
        return;
    }
    let aggressive = strategy == Strategy::Aggressive;
    let reserve: Yen = if aggressive { 5_000_000 } else { 8_000_000 };
    // 資金が尽きそうなら借りてしのぐ
    if aggressive && w.cash < 1_000_000 {
        let need = 5_000_000.min(w.credit_limit());
        if need >= crate::corporate::LOAN_MIN {
            let _ = w.borrow(need, 5);
        }
    }

    // 月に 1 回、投資の判断をする（開校は四半期に 1 回まで）
    if week % 4 != 1 {
        return;
    }
    let can_open = week % 13 == 1 || !aggressive;
    // 買収
    for comp in w.acquisition_targets() {
        let price = w.acquisition_price(comp);
        if w.cash > price + reserve {
            let _ = w.acquire(comp);
        }
    }
    // 開校（安いエリアから）
    let mut areas = w.open_areas();
    areas.sort_by_key(|a| w.areas[*a].open_cost);
    if let Some(&a) = areas.first().filter(|_| can_open) {
        let cost = w.areas[a].open_cost;
        if aggressive && w.cash < cost + reserve {
            let need = (cost + reserve - w.cash).max(crate::corporate::LOAN_MIN);
            let need = (need / 1_000_000 + 1) * 1_000_000;
            if need <= w.credit_limit() {
                let _ = w.borrow(need, 5);
            }
        }
        if w.cash > cost + reserve && !w.manager_candidates.is_empty() {
            // 倫理の高い教室長を選ぶ
            let best = (0..w.manager_candidates.len()).max_by_key(|&i| w.manager_candidates[i].ethics).unwrap_or(0);
            if let Ok(ci) = w.open_classroom(a, best) {
                for d in Delegation::ALL {
                    w.classroom_mut(ci).set_delegation(d, true);
                }
            }
        }
    }
    // 2 教室以上になったら自社教材
    if w.classrooms.len() >= 2 && !w.materials.is_own() && w.cash > crate::materials::LAUNCH_COST + reserve {
        let _ = w.launch_own_materials();
    }
    if w.materials.is_own() && w.materials.project.is_none() && w.cash > reserve * 2 {
        // 入試対応を最優先、次にわかりやすさ・網羅性・演習量
        let order = [AXIS_TREND, AXIS_CLARITY, AXIS_COVERAGE, AXIS_PRACTICE];
        if let Some(&axis) = order.iter().min_by(|a, b| {
            w.materials.axes[**a].partial_cmp(&w.materials.axes[**b]).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            if w.materials.axes[axis] < 0.8 {
                w.start_material_project(axis, 1);
            }
        }
    }
    // 教材が育ったら外販を始める
    if w.materials.is_own()
        && !w.materials.sales.active
        && w.materials.quality_score() >= 0.6
        && w.cash > crate::materials::SALES_LAUNCH_COST + reserve
    {
        let _ = w.start_material_sales();
    }
    // 条件を満たしたら上場準備（準備費用 2,600 万円の余裕を持って）
    if w.listing.status == crate::listing::ListingStatus::Private && w.cash > 30_000_000 && w.ipo_ready() {
        let _ = w.start_ipo();
    }
    // 物件：十分に余裕があれば買う
    for ci in 0..w.classrooms.len() {
        let price = w.property_price(ci);
        if !w.classrooms[ci].owned && w.cash > price * 2 {
            let _ = w.buy_property(ci);
        }
    }
    // スター講師（積極拡大のみ）
    // 集客効果は 1 教室 2 人までなので、それ以上は雇わない
    let star_room = (0..w.classrooms.len()).find(|&ci| w.classrooms[ci].star_teachers() < 2);
    if let (true, Some(room)) = (aggressive, star_room) {
        if let Some((comp, star)) = w.star_targets().into_iter().find(|(c, _)| w.competitors[*c].offer_cooldown == 0) {
            let (_, bonus) = w.star_offer_terms(comp, star, OfferLevel::Good);
            if w.cash > bonus + reserve * 2 {
                let _ = w.offer_star(comp, star, room, OfferLevel::Good);
            }
        }
    }
}

/// 複数シードの平均。
pub fn average(runs: &[Run], year: usize, f: impl Fn(&YearStats) -> f64) -> f64 {
    let v: Vec<f64> = runs.iter().filter_map(|r| r.years.get(year)).map(f).collect();
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}
