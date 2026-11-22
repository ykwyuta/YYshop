//! ゲームバランスの回帰テスト。
//!
//! 自動プレイヤー（`juku_sim::bot`）で複数シードを回し、設計上の狙い
//! （docs/proposal/03_technical_architecture.md の 11 章）が崩れていないかを確かめる。
//! 係数を調整したら `cargo run -p juku-sim --example balance --release` で詳細を見ること。

use juku_sim::bot::{average, play, play_with, Run, Strategy};

const SEEDS: u64 = 5;
const YEARS: u32 = 5;

fn runs(strategy: Strategy) -> Vec<Run> {
    (0..SEEDS).map(|s| play(1000 + s, strategy, YEARS)).collect()
}

fn last_year() -> usize {
    YEARS as usize - 1
}

fn cumulative_profit(runs: &[Run]) -> f64 {
    runs.iter().map(|r| r.years.iter().map(|y| y.net_profit as f64).sum::<f64>()).sum::<f64>() / runs.len() as f64
}

#[test]
fn standard_play_is_stable_and_never_bankrupt() {
    let rs = runs(Strategy::Standard);
    for r in &rs {
        assert!(!r.last().bankrupt, "標準プレイで倒産しない");
    }
    // 1 教室で年 500 万〜3,000 万円の利益
    for y in 0..YEARS as usize {
        let p = average(&rs, y, |s| s.net_profit as f64);
        assert!((5_000_000.0..30_000_000.0).contains(&p), "{} 年目の利益 {p}", y + 1);
    }
    // 第一志望合格率は 30〜65%
    let fc = (0..YEARS as usize).map(|y| average(&rs, y, |s| s.first_choice_rate)).sum::<f64>() / YEARS as f64;
    assert!((0.30..0.65).contains(&fc), "第一志望合格率 {fc}");
}

#[test]
fn doing_nothing_is_worse_than_staffing() {
    let passive = runs(Strategy::Passive);
    let standard = runs(Strategy::Standard);
    let y = last_year();
    assert!(average(&standard, y, |s| s.students as f64) > average(&passive, y, |s| s.students as f64) * 1.2);
    assert!(average(&standard, y, |s| s.valuation as f64) > average(&passive, y, |s| s.valuation as f64));
}

#[test]
fn growth_pays_off_but_costs_early() {
    let standard = runs(Strategy::Standard);
    let growth = runs(Strategy::Growth);
    // 1 年目は開校の持ち出しで資金が少ない（リスク）
    assert!(average(&growth, 0, |s| s.cash as f64) < average(&standard, 0, |s| s.cash as f64));
    // 5 年後には企業価値で大きく上回る（報酬）
    let y = last_year();
    assert!(
        average(&growth, y, |s| s.valuation as f64) > average(&standard, y, |s| s.valuation as f64) * 2.0,
        "成長戦略は 5 年後に標準の 2 倍以上の企業価値"
    );
    // ただし市場を独占はしない
    assert!(average(&growth, y, |s| s.share) < 0.5);
}

#[test]
fn short_term_greed_backfires() {
    let standard = runs(Strategy::Standard);
    let greedy = runs(Strategy::Greedy);
    // 1 年目は標準と同じくらい稼げる
    assert!(average(&greedy, 0, |s| s.net_profit as f64) > average(&standard, 0, |s| s.net_profit as f64) * 0.9);
    // 5 年後には生徒・信頼・企業価値を失っている
    let y = last_year();
    assert!(average(&greedy, y, |s| s.valuation as f64) < average(&standard, y, |s| s.valuation as f64) * 0.5);
    assert!(average(&greedy, y, |s| s.trust) < average(&standard, y, |s| s.trust) - 0.2);
}

#[test]
fn reference_price_is_near_optimal() {
    let profit_at = |pct: i64| {
        let rs: Vec<Run> = (0..SEEDS)
            .map(|s| {
                play_with(1000 + s, Strategy::Standard, YEARS, |w| {
                    let c = w.classroom_mut(0);
                    for i in 0..3 {
                        let fee = c.courses[i].fee;
                        c.set_course_fee(i, fee * pct / 100);
                    }
                })
            })
            .collect();
        cumulative_profit(&rs)
    };
    let (cheap, normal, pricey) = (profit_at(70), profit_at(100), profit_at(150));
    assert!(normal > cheap, "大幅な値下げは割に合わない: {cheap} vs {normal}");
    assert!(normal > pricey, "大幅な値上げは割に合わない: {pricey} vs {normal}");
}

#[test]
fn homework_is_a_tradeoff() {
    let at = |level: u8| -> Vec<Run> {
        (0..SEEDS)
            .map(|s| play_with(1000 + s, Strategy::Standard, YEARS, |w| w.classroom_mut(0).set_homework(level)))
            .collect()
    };
    let (l3, l4, l5) = (at(3), at(4), at(5));
    let fc = |rs: &[Run]| (0..YEARS as usize).map(|y| average(rs, y, |s| s.first_choice_rate)).sum::<f64>();
    let churn = |rs: &[Run]| (0..YEARS as usize).map(|y| average(rs, y, |s| s.churned as f64)).sum::<f64>();
    assert!(fc(&l4) > fc(&l3), "宿題を増やすと合格率は上がる");
    assert!(churn(&l4) > churn(&l3), "ただし退塾も増える");
    assert!(cumulative_profit(&l5) < cumulative_profit(&l3), "やりすぎると経営が傾く");
}

#[test]
fn market_size_is_stable() {
    let rs = runs(Strategy::Standard);
    let first = average(&rs, 0, |s| s.market_total as f64);
    let last = average(&rs, last_year(), |s| s.market_total as f64);
    assert!((last / first - 1.0).abs() < 0.25, "地域の総生徒数が大きく増減しない: {first} → {last}");
}
