//! 教材（市販教材 → 自社教材）。会社全体で 1 つを使う。
//!
//! 5 つの軸（網羅性・難度・わかりやすさ・演習量・入試対応）で性格が決まり、
//! 学力の伸び方・ストレス・入試の合格率に効く。

use serde::{Deserialize, Serialize};

use crate::calendar::Date;
use crate::model::Yen;

pub const AXIS_COUNT: usize = 5;
pub const AXIS_LABELS: [&str; AXIS_COUNT] = ["網羅性", "難度", "わかりやすさ", "演習量", "入試対応"];
pub const AXIS_NOTES: [&str; AXIS_COUNT] = [
    "上位層の伸びと最難関校の合格率に効く",
    "上位層は伸びるが、下位層はついていけずストレスが増える",
    "中位・下位層の伸びと意欲に効く。授業の評価も上がる",
    "全員の学力が伸びるが、ストレスも増える",
    "入試の合格率に直結する。毎年の入試傾向の変化で古くなる",
];

/// 網羅性・難度・わかりやすさ・演習量・入試対応
pub const AXIS_COVERAGE: usize = 0;
pub const AXIS_DIFFICULTY: usize = 1;
pub const AXIS_CLARITY: usize = 2;
pub const AXIS_PRACTICE: usize = 3;
pub const AXIS_TREND: usize = 4;

/// 自社教材の立ち上げ費用。
pub const LAUNCH_COST: Yen = 2_500_000;
/// 生徒 1 人あたりの月間教材原価。
pub const COMMERCIAL_COST_MONTHLY: Yen = 2_600;
pub const OWN_COST_MONTHLY: Yen = 1_300;
/// 開発プロジェクトの期間（週）と 1 回の上げ幅。
pub const PROJECT_WEEKS: u32 = 4;
pub const PROJECT_STEP: f64 = 0.12;

const COMMERCIAL_AXES: [f64; AXIS_COUNT] = [0.55, 0.5, 0.5, 0.5, 0.45];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialSource {
    /// 市販教材
    Commercial,
    /// 自社教材
    Own,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DevProject {
    pub axis: usize,
    /// +1 = 上げる、-1 = 下げる（難度のみ）
    pub direction: i8,
    pub weeks_left: u32,
    pub weekly_cost: Yen,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Materials {
    pub source: MaterialSource,
    pub axes: [f64; AXIS_COUNT],
    pub project: Option<DevProject>,
    pub launched: Option<Date>,
    /// 改訂した回数
    pub revisions: u32,
    /// 他塾への外販
    #[serde(default)]
    pub sales: ExternalSales,
}

// ---------------------------------------------------------------------------
// 教材の外販
// ---------------------------------------------------------------------------

/// 外販を始めるための営業体制づくりの費用。
pub const SALES_LAUNCH_COST: Yen = 3_000_000;
/// 外販の営業・サポートの固定費（週）。
pub const SALES_TEAM_WEEKLY: Yen = 80_000;
/// 外販の売上に対する原価（印刷・配送・解説動画など）。
pub const SALES_COGS_RATE: f64 = 0.2;
/// 外販の利益にかかる税率。
pub const SALES_TAX_RATE: f64 = 0.3;
/// 教材を買ってくれそうな塾の数（市場）。
pub const SALES_MARKET: f64 = 200.0;
/// 地域の競合塾にも売ると、市場がこれだけ広がる。
pub const SALES_RIVAL_BONUS: f64 = 0.4;
/// 1 塾あたりの月額の相場。
pub const SALES_REFERENCE_FEE: Yen = 30_000;

/// 他塾への教材の外販。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalSales {
    pub active: bool,
    pub started: Option<Date>,
    /// 契約している塾の数
    pub contracts: u32,
    /// 1 塾あたりの月額
    pub fee: Yen,
    /// 自塾と競合する地域の塾にも売る
    pub sell_to_rivals: bool,
    /// 先週の新規契約・解約
    pub last_new: u32,
    pub last_cancel: u32,
}

impl Default for ExternalSales {
    fn default() -> Self {
        ExternalSales {
            active: false,
            started: None,
            contracts: 0,
            fee: SALES_REFERENCE_FEE,
            sell_to_rivals: false,
            last_new: 0,
            last_cancel: 0,
        }
    }
}

/// 1 週分の外販の結果。
#[derive(Clone, Copy, Debug, Default)]
pub struct SalesWeek {
    pub revenue: Yen,
    /// 原価・営業費・税
    pub cost: Yen,
}

impl Materials {
    /// 外販での評価（入試対応・網羅性・わかりやすさ・演習量の平均。難度は好みが分かれるので入れない）。
    pub fn quality_score(&self) -> f64 {
        let a = &self.axes;
        (a[AXIS_COVERAGE] + a[AXIS_CLARITY] + a[AXIS_PRACTICE] + a[AXIS_TREND]) / 4.0
    }

    /// 外販の 1 週分。契約は品質・ブランド・価格で増え、品質が落ちると解約される。
    pub fn advance_sales(&mut self, brand: f64, rng: &mut crate::rng::Rng) -> SalesWeek {
        if !self.sales.active {
            return SalesWeek::default();
        }
        let q = self.quality_score();
        let s = &mut self.sales;
        let price = crate::market::price_score(s.fee, SALES_REFERENCE_FEE);
        let appeal = (0.6 * q + 0.25 * brand + 0.15 * price).clamp(0.0, 1.0);
        let market = SALES_MARKET * if s.sell_to_rivals { 1.0 + SALES_RIVAL_BONUS } else { 1.0 };
        let potential = (market - s.contracts as f64).max(0.0);
        let new = rng.round_stochastic(potential * appeal * appeal * appeal * 0.012);
        let fee_ratio = s.fee as f64 / SALES_REFERENCE_FEE as f64;
        let churn_rate = 0.002 + (0.55 - q).max(0.0) * 0.03 + (fee_ratio - 1.2).max(0.0) * 0.01;
        let cancel = rng.round_stochastic(s.contracts as f64 * churn_rate).min(s.contracts);
        s.contracts = s.contracts + new - cancel;
        s.last_new = new;
        s.last_cancel = cancel;
        let revenue = (s.contracts as f64 * s.fee as f64 * 12.0 / 52.0) as Yen;
        let mut cost = SALES_TEAM_WEEKLY + (revenue as f64 * SALES_COGS_RATE) as Yen;
        let profit = revenue - cost;
        if profit > 0 {
            cost += (profit as f64 * SALES_TAX_RATE) as Yen;
        }
        SalesWeek { revenue, cost }
    }
}

impl Default for Materials {
    fn default() -> Self {
        Materials {
            source: MaterialSource::Commercial,
            axes: COMMERCIAL_AXES,
            project: None,
            launched: None,
            sales: ExternalSales::default(),
            revisions: 0,
        }
    }
}

impl Materials {
    pub fn is_own(&self) -> bool {
        self.source == MaterialSource::Own
    }

    pub fn name(&self) -> &'static str {
        match self.source {
            MaterialSource::Commercial => "市販教材",
            MaterialSource::Own => "自社オリジナル教材",
        }
    }

    pub fn cost_per_student_monthly(&self) -> Yen {
        match self.source {
            MaterialSource::Commercial => COMMERCIAL_COST_MONTHLY,
            MaterialSource::Own => OWN_COST_MONTHLY,
        }
    }

    /// 開発プロジェクトの週あたり費用（高い軸ほど上げにくい）。
    pub fn project_weekly_cost(&self, axis: usize) -> Yen {
        let level = self.axes[axis];
        let cost = 120_000.0 * (1.0 + level * 1.5);
        (cost / 10_000.0) as Yen * 10_000
    }

    pub fn can_start_project(&self, axis: usize, direction: i8) -> bool {
        if !self.is_own() || self.project.is_some() || axis >= AXIS_COUNT {
            return false;
        }
        if direction < 0 {
            axis == AXIS_DIFFICULTY && self.axes[axis] > 0.15
        } else {
            self.axes[axis] < 0.99
        }
    }

    pub fn start_project(&mut self, axis: usize, direction: i8) -> bool {
        if !self.can_start_project(axis, direction) {
            return false;
        }
        self.project = Some(DevProject {
            axis,
            direction: direction.signum(),
            weeks_left: PROJECT_WEEKS,
            weekly_cost: self.project_weekly_cost(axis),
        });
        true
    }

    /// 1 週分の開発を進める。費用を返し、完成したら Some(軸) を返す。
    pub fn advance(&mut self) -> (Yen, Option<usize>) {
        let Some(p) = &mut self.project else { return (0, None) };
        let cost = p.weekly_cost;
        p.weeks_left = p.weeks_left.saturating_sub(1);
        if p.weeks_left == 0 {
            let axis = p.axis;
            let delta = PROJECT_STEP * p.direction as f64;
            self.axes[axis] = (self.axes[axis] + delta).clamp(0.05, 1.0);
            self.project = None;
            self.revisions += 1;
            (cost, Some(axis))
        } else {
            (cost, None)
        }
    }

    /// 年度替わり：入試傾向が変わり、入試対応が古くなる。
    pub fn yearly_decay(&mut self) {
        match self.source {
            MaterialSource::Own => self.axes[AXIS_TREND] = (self.axes[AXIS_TREND] - 0.2).max(0.1),
            // 市販教材は出版社が毎年改訂する
            MaterialSource::Commercial => self.axes = COMMERCIAL_AXES,
        }
    }

    /// 週あたりの偏差値の伸びへの寄与。
    pub fn learning_bonus(&self, deviation: f64) -> f64 {
        let a = &self.axes;
        let upper = deviation >= 55.0;
        let lower = deviation < 45.0;
        let coverage = 0.03 * (a[AXIS_COVERAGE] - 0.5) * if upper { 1.5 } else { 0.5 };
        let difficulty = 0.05
            * (a[AXIS_DIFFICULTY] - 0.5)
            * if upper {
                1.0
            } else if lower {
                -1.2
            } else {
                0.0
            };
        let clarity = 0.05 * (a[AXIS_CLARITY] - 0.5) * if upper { 0.3 } else { 1.0 };
        let practice = 0.035 * (a[AXIS_PRACTICE] - 0.5);
        coverage + difficulty + clarity + practice
    }

    /// 週あたりのストレス増加への寄与。
    pub fn stress_delta(&self, deviation: f64) -> f64 {
        let a = &self.axes;
        let lower = deviation < 45.0;
        0.008 * (a[AXIS_DIFFICULTY] - 0.5) * if lower { 2.0 } else { 0.7 } + 0.01 * (a[AXIS_PRACTICE] - 0.5)
    }

    pub fn motivation_delta(&self) -> f64 {
        0.006 * (self.axes[AXIS_CLARITY] - 0.5)
    }

    /// 授業品質への寄与（0..1 のスケール）。
    pub fn quality_bonus(&self) -> f64 {
        0.12 * (self.axes[AXIS_CLARITY] - 0.5) + 0.06 * (self.axes[AXIS_COVERAGE] - 0.5)
    }

    /// 入試で偏差値に上乗せする点数。`top` は最難関校か。
    pub fn exam_bonus(&self, top: bool) -> f64 {
        let trend = (self.axes[AXIS_TREND] - 0.5) * 2.4;
        let coverage = if top { (self.axes[AXIS_COVERAGE] - 0.5) * 1.6 } else { 0.0 };
        trend + coverage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_raises_axis_after_weeks() {
        let mut m = Materials { source: MaterialSource::Own, ..Default::default() };
        let before = m.axes[AXIS_CLARITY];
        assert!(m.start_project(AXIS_CLARITY, 1));
        assert!(!m.start_project(AXIS_PRACTICE, 1), "同時に 2 つは進められない");
        let mut total = 0;
        for _ in 0..PROJECT_WEEKS {
            total += m.advance().0;
        }
        assert!(m.project.is_none());
        assert!((m.axes[AXIS_CLARITY] - before - PROJECT_STEP).abs() < 1e-9);
        assert!(total > 0);
    }

    #[test]
    fn commercial_cannot_develop() {
        let mut m = Materials::default();
        assert!(!m.start_project(AXIS_CLARITY, 1));
    }

    #[test]
    fn difficulty_hurts_lower_students() {
        let mut m = Materials { source: MaterialSource::Own, ..Default::default() };
        m.axes[AXIS_DIFFICULTY] = 0.9;
        assert!(m.learning_bonus(40.0) < m.learning_bonus(60.0));
        assert!(m.stress_delta(40.0) > m.stress_delta(60.0));
    }

    #[test]
    fn trend_decays_for_own() {
        let mut m = Materials { source: MaterialSource::Own, ..Default::default() };
        m.axes[AXIS_TREND] = 0.8;
        m.yearly_decay();
        assert!(m.axes[AXIS_TREND] < 0.8);
    }
}
