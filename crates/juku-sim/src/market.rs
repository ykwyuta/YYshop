//! エリアと競合塾。競合は 4 週ごとに状況を見て自分から動く。

use serde::{Deserialize, Serialize};

use crate::calendar::Date;
use crate::data;
use crate::model::Yen;
use crate::rng::Rng;

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Area {
    pub name: String,
    pub address: String,
    pub description: String,
    /// 市場規模（入塾希望者数の倍率）
    pub pool: f64,
    pub rent: Yen,
    pub open_cost: Yen,
    /// 物件を買い取る場合の価格（年 2% ずつ上がる）
    pub property_price: Yen,
    /// 市場規模の年間の伸び
    #[serde(default)]
    pub pool_growth: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompetitorStyle {
    /// 大手難関型：資金力があり、強い相手の隣に出店してくる
    Elite,
    /// 面倒見型：負けると面倒見を強化する
    Caring,
    /// 格安オンライン型：値下げで応戦する
    Online,
    /// 個別指導型：入塾キャンペーンと出店
    Individual,
    /// 個人塾：地域密着の小さな塾。後継者不足で売りに出ることがある
    Independent,
    /// 中小塾：数教室の地場チェーン。経営が苦しくなると売りに出る
    SmallChain,
}

impl CompetitorStyle {
    pub fn label(self) -> &'static str {
        match self {
            CompetitorStyle::Elite => "大手難関型",
            CompetitorStyle::Caring => "面倒見型",
            CompetitorStyle::Online => "格安オンライン型",
            CompetitorStyle::Individual => "個別指導型",
            CompetitorStyle::Independent => "個人塾",
            CompetitorStyle::SmallChain => "中小塾",
        }
    }

    /// 市場標準に対する月謝の上限（これ以上は値上げしない）。
    fn price_cap(self) -> f64 {
        match self {
            CompetitorStyle::Elite => 1.3,
            CompetitorStyle::Caring => 1.0,
            CompetitorStyle::Online => 0.6,
            CompetitorStyle::Individual => 1.25,
            CompetitorStyle::Independent | CompetitorStyle::SmallChain => 1.0,
        }
    }

    fn expansion_chance(self) -> f64 {
        match self {
            CompetitorStyle::Elite => 0.3,
            CompetitorStyle::Individual => 0.2,
            CompetitorStyle::Caring => 0.1,
            CompetitorStyle::Online | CompetitorStyle::Independent | CompetitorStyle::SmallChain => 0.0,
        }
    }

    /// 大手かどうか（買収できない）。
    pub fn is_major(self) -> bool {
        matches!(
            self,
            CompetitorStyle::Elite | CompetitorStyle::Caring | CompetitorStyle::Online | CompetitorStyle::Individual
        )
    }

    /// 撤退を考え始める在籍数。
    fn weak_threshold(self) -> u32 {
        if self.is_major() {
            18
        } else {
            6
        }
    }

    /// スター講師の引き抜きに応じやすさ（基本確率）。
    pub fn star_base_chance(self) -> f64 {
        match self {
            CompetitorStyle::Elite => 0.22,
            CompetitorStyle::Caring => 0.30,
            CompetitorStyle::Online => 0.35,
            CompetitorStyle::Individual => 0.40,
            CompetitorStyle::Independent | CompetitorStyle::SmallChain => 0.45,
        }
    }

    fn poach_chance(self) -> f64 {
        match self {
            CompetitorStyle::Elite => 0.12,
            CompetitorStyle::Individual => 0.08,
            _ => 0.0,
        }
    }
}

/// 競合の 1 エリアの教室。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Branch {
    pub area: usize,
    /// 学年別の在籍数（4・5・6 年）
    pub students: [u32; 3],
    pub fees: [Yen; 3],
    /// エリアでの認知度 0..1
    pub awareness: f64,
    /// 前回（4 週前）のエリアシェア
    pub prev_share: f64,
    /// 生徒が少ない状態が続いた月数（撤退の判断に使う）
    pub weak_months: u32,
}

impl Branch {
    pub fn total(&self) -> u32 {
        self.students.iter().sum()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Competitor {
    pub name: String,
    pub style: CompetitorStyle,
    pub brand: f64,
    pub quality: f64,
    pub care: f64,
    pub color: [u8; 3],
    pub branches: Vec<Branch>,
    /// 新規出店時の月謝
    pub base_fees: [Yen; 3],
    /// 看板のスター講師（引き抜きの対象）
    #[serde(default)]
    pub stars: Vec<crate::model::Teacher>,
    /// 売りに出ている（個人塾・中小塾のみ）
    #[serde(default)]
    pub for_sale: bool,
    /// 自塾が買収した
    #[serde(default)]
    pub acquired: bool,
    /// スター講師に声をかけられるまでの週数
    #[serde(default)]
    pub offer_cooldown: u32,
    /// 講師を引き抜かれた恨み（週数）。この間は仕返しの引き抜きが増える
    #[serde(default)]
    pub grudge: u32,
}

impl Competitor {
    pub fn from_def(d: &data::CompetitorDef) -> Self {
        Competitor {
            name: d.name.into(),
            style: d.style,
            brand: d.brand,
            quality: d.quality,
            care: d.care,
            color: d.color,
            base_fees: d.fees,
            stars: Vec::new(),
            for_sale: false,
            acquired: false,
            offer_cooldown: 0,
            grudge: 0,
            branches: d
                .branches
                .iter()
                .map(|(area, students)| Branch {
                    area: *area,
                    students: *students,
                    fees: d.fees,
                    awareness: 0.6,
                    prev_share: 0.0,
                    weak_months: 0,
                })
                .collect(),
        }
    }

    pub fn branch(&self, area: usize) -> Option<&Branch> {
        self.branches.iter().find(|b| b.area == area)
    }

    pub fn branch_mut(&mut self, area: usize) -> Option<&mut Branch> {
        self.branches.iter_mut().find(|b| b.area == area)
    }

    pub fn total_students(&self) -> u32 {
        self.branches.iter().map(|b| b.total()).sum()
    }

    pub fn students_in_area(&self, area: usize) -> u32 {
        self.branch(area).map(|b| b.total()).unwrap_or(0)
    }

    /// 学年 `gi`（0..3）での魅力度。
    pub fn attractiveness(&self, branch: &Branch, gi: usize) -> f64 {
        0.30 * self.brand
            + 0.28 * self.quality
            + 0.16 * self.care
            + 0.14 * price_score(branch.fees[gi], data::REFERENCE_FEES[gi])
            + 0.12 * branch.awareness
    }
}

/// 市場標準に対する価格の魅力（0..1）。
pub fn price_score(fee: Yen, reference: Yen) -> f64 {
    let ratio = fee as f64 / reference as f64;
    clamp01(1.3 - 0.8 * ratio)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RivalMoveKind {
    PriceCut,
    PriceUp,
    Campaign,
    CareUp,
    Open,
    Withdraw,
    Poach,
    ForSale,
}

/// 競合の動き（ログ）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RivalMove {
    pub date: Date,
    pub competitor: usize,
    pub area: usize,
    pub kind: RivalMoveKind,
    pub text: String,
}

/// 自塾の各エリアでの状況（競合の判断材料）。
pub struct PlayerPresence {
    pub area: usize,
    pub classroom: usize,
    pub students: u32,
    /// 開校してからの週数
    pub weeks_open: u32,
    /// 最も授業力の高い講師（引き抜きの標的）
    pub best_teacher: Option<(u32, u32)>,
}

/// 引き抜きの申し出。
pub struct PoachAttempt {
    pub competitor: usize,
    pub classroom: usize,
    pub teacher_id: u32,
}

/// 4 週ごとの競合の行動。
pub fn competitor_turn(
    competitors: &mut [Competitor],
    areas: &[Area],
    player: &[PlayerPresence],
    rng: &mut Rng,
    date: Date,
) -> (Vec<RivalMove>, Option<PoachAttempt>) {
    let mut moves = Vec::new();
    let mut poach = None;
    let mut expanded_this_turn = false;

    // エリアごとの総在籍数
    let mut area_total = vec![0u32; areas.len()];
    for c in competitors.iter() {
        for b in &c.branches {
            area_total[b.area] += b.total();
        }
    }
    for p in player {
        area_total[p.area] += p.students;
    }

    for ci in 0..competitors.len() {
        if competitors[ci].acquired {
            continue;
        }
        let style = competitors[ci].style;
        let name = competitors[ci].name.clone();
        let mut withdraw = Vec::new();
        competitors[ci].offer_cooldown = competitors[ci].offer_cooldown.saturating_sub(4);
        competitors[ci].grudge = competitors[ci].grudge.saturating_sub(4);

        for bi in 0..competitors[ci].branches.len() {
            let area = competitors[ci].branches[bi].area;
            let total = area_total[area].max(1);
            let b = &mut competitors[ci].branches[bi];
            let share = b.total() as f64 / total as f64;
            let delta = share - b.prev_share;
            let first = b.prev_share == 0.0;
            b.prev_share = share;
            b.awareness += (0.55 - b.awareness) * 0.1;
            let area_name = &areas[area].name;
            let mut log = |kind, text: String| {
                moves.push(RivalMove { date, competitor: ci, area, kind, text });
            };

            if first {
                continue;
            }
            if delta < -0.015 {
                match style {
                    CompetitorStyle::Elite => {
                        b.awareness = clamp01(b.awareness + 0.25);
                        log(
                            RivalMoveKind::Campaign,
                            format!("{name}が{area_name}で大規模な広告キャンペーンを始めました"),
                        );
                    }
                    CompetitorStyle::Caring => {
                        competitors[ci].care = clamp01(competitors[ci].care + 0.03);
                        log(RivalMoveKind::CareUp, format!("{name}が面談と補習を強化しました（{area_name}）"));
                    }
                    CompetitorStyle::Online => {
                        for f in b.fees.iter_mut() {
                            *f = (*f * 90 / 100 / 1000).max(5) * 1000;
                        }
                        log(RivalMoveKind::PriceCut, format!("{name}が{area_name}で月謝を 1 割値下げしました"));
                    }
                    CompetitorStyle::Individual => {
                        for f in b.fees.iter_mut() {
                            *f = (*f * 95 / 100 / 1000) * 1000;
                        }
                        b.awareness = clamp01(b.awareness + 0.12);
                        log(
                            RivalMoveKind::PriceCut,
                            format!("{name}が{area_name}で入塾金無料キャンペーンを始めました"),
                        );
                    }
                    CompetitorStyle::Independent | CompetitorStyle::SmallChain => {
                        if rng.chance(0.5) {
                            for f in b.fees.iter_mut() {
                                *f = (*f * 95 / 100 / 1000) * 1000;
                            }
                            log(RivalMoveKind::PriceCut, format!("{name}が{area_name}で月謝を少し下げました"));
                        }
                    }
                }
            } else if delta > 0.015 && rng.chance(0.5) {
                let ratio = b.fees[1] as f64 / data::REFERENCE_FEES[1] as f64;
                if ratio < style.price_cap() {
                    for f in b.fees.iter_mut() {
                        *f = (*f * 105 / 100 / 1000) * 1000;
                    }
                    log(RivalMoveKind::PriceUp, format!("{name}が{area_name}で月謝を値上げしました"));
                }
            }

            if total < 40 || b.total() < style.weak_threshold() {
                b.weak_months += 1;
            } else {
                b.weak_months = 0;
            }
            if b.weak_months >= 4 && style != CompetitorStyle::Online {
                withdraw.push(bi);
                let text = if style.is_major() {
                    format!("{name}が{area_name}から撤退しました")
                } else {
                    format!("{name}（{area_name}）が閉塾しました")
                };
                moves.push(RivalMove { date, competitor: ci, area, kind: RivalMoveKind::Withdraw, text });
            }
        }
        for bi in withdraw.into_iter().rev() {
            competitors[ci].branches.remove(bi);
        }

        // 個人塾・中小塾は、後継者不足や経営難で売りに出ることがある
        if !style.is_major() && !competitors[ci].for_sale && !competitors[ci].branches.is_empty() {
            let weak = competitors[ci].branches.iter().any(|b| b.weak_months > 0);
            if rng.chance(if weak { 0.15 } else { 0.03 }) {
                competitors[ci].for_sale = true;
                let area = competitors[ci].branches[0].area;
                let reason = if style == CompetitorStyle::Independent {
                    "後継者がいないため"
                } else {
                    "経営の立て直しのため"
                };
                moves.push(RivalMove {
                    date,
                    competitor: ci,
                    area,
                    kind: RivalMoveKind::ForSale,
                    text: format!("{name}が{reason}、売却先を探しています"),
                });
            }
        }

        // 品質のゆらぎ
        let q = competitors[ci].quality + rng.normal(0.0, 0.006);
        competitors[ci].quality = q.clamp(0.3, 0.9);

        // 出店：自塾が半年以上かけて強くなったエリアに乗り込んでくる（1 回の判断で 1 社まで）
        if !expanded_this_turn && rng.chance(style.expansion_chance()) {
            let target = player
                .iter()
                .filter(|p| p.weeks_open >= 26 && competitors[ci].branch(p.area).is_none())
                .map(|p| (p.area, p.students as f64 / area_total[p.area].max(1) as f64))
                .filter(|(_, share)| *share >= 0.3)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            if let Some((area, _)) = target {
                expanded_this_turn = true;
                let fees = competitors[ci].base_fees;
                competitors[ci].branches.push(Branch {
                    area,
                    students: [6, 5, 4],
                    fees,
                    awareness: 0.75,
                    prev_share: 0.0,
                    weak_months: 0,
                });
                moves.push(RivalMove {
                    date,
                    competitor: ci,
                    area,
                    kind: RivalMoveKind::Open,
                    text: format!("{name}が{}駅前に新しい教室を開きました", areas[area].name),
                });
            }
        }

        // 引き抜き：同じエリアにいる自塾の人気講師を狙う
        let poach_chance = if competitors[ci].grudge > 0 { style.poach_chance() * 2.5 } else { style.poach_chance() };
        if poach.is_none() && rng.chance(poach_chance) {
            let target = player
                .iter()
                .filter(|p| competitors[ci].branch(p.area).is_some())
                .filter_map(|p| p.best_teacher.map(|(id, skill)| (p.classroom, id, skill)))
                .filter(|(_, _, skill)| *skill >= 70)
                .max_by_key(|(_, _, skill)| *skill);
            if let Some((classroom, teacher_id, _)) = target {
                poach = Some(PoachAttempt { competitor: ci, classroom, teacher_id });
            }
        }
    }
    (moves, poach)
}

/// 年度替わり：入試結果でブランドが動き、学年が上がる。
pub fn competitor_new_year(competitors: &mut [Competitor], rng: &mut Rng) {
    for c in competitors.iter_mut().filter(|c| !c.acquired) {
        let result = clamp01(c.quality + rng.normal(0.0, 0.08));
        c.brand = clamp01(c.brand * 0.6 + result * 0.4);
        for b in c.branches.iter_mut() {
            b.students = [0, b.students[0], b.students[1]];
        }
    }
}

/// 毎週の自然な退塾。
pub fn competitor_churn(competitors: &mut [Competitor], rng: &mut Rng) {
    for c in competitors.iter_mut().filter(|c| !c.acquired) {
        for b in c.branches.iter_mut() {
            for gi in 0..3 {
                let leave = rng.round_stochastic(b.students[gi] as f64 * 0.004);
                b.students[gi] = b.students[gi].saturating_sub(leave);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Vec<Competitor>, Vec<Area>) {
        let comps = data::COMPETITORS.iter().map(Competitor::from_def).collect();
        let areas = data::AREAS
            .iter()
            .map(|a| Area {
                name: a.name.into(),
                address: a.address.into(),
                description: a.description.into(),
                pool: a.pool,
                rent: a.rent,
                open_cost: a.open_cost,
                property_price: a.property_price,
                pool_growth: a.pool_growth,
            })
            .collect();
        (comps, areas)
    }

    fn date() -> Date {
        Date { year: 2027, month: 4, day: 1 }
    }

    #[test]
    fn online_cuts_price_when_losing_share() {
        let (mut comps, areas) = setup();
        let mut rng = Rng::new(1);
        let player = vec![PlayerPresence { area: 0, classroom: 0, students: 60, weeks_open: 100, best_teacher: None }];
        competitor_turn(&mut comps, &areas, &player, &mut rng, date());
        let before = comps[2].branch(0).unwrap().fees[1];
        // 自塾が急成長してシェアを奪う
        let player = vec![PlayerPresence { area: 0, classroom: 0, students: 400, weeks_open: 100, best_teacher: None }];
        let (moves, _) = competitor_turn(&mut comps, &areas, &player, &mut rng, date());
        assert!(comps[2].branch(0).unwrap().fees[1] < before);
        assert!(moves.iter().any(|m| m.competitor == 2 && m.kind == RivalMoveKind::PriceCut));
    }

    #[test]
    fn rivals_eventually_open_next_to_a_strong_player() {
        let (mut comps, areas) = setup();
        let mut rng = Rng::new(3);
        // 緑町（3）で自塾が独占している
        let player = vec![PlayerPresence { area: 3, classroom: 0, students: 120, weeks_open: 100, best_teacher: None }];
        let mut opened = false;
        for _ in 0..30 {
            let (moves, _) = competitor_turn(&mut comps, &areas, &player, &mut rng, date());
            if moves.iter().any(|m| m.kind == RivalMoveKind::Open && m.area == 3) {
                opened = true;
                break;
            }
        }
        assert!(opened);
    }

    #[test]
    fn elite_tries_to_poach_star_teacher() {
        let (mut comps, areas) = setup();
        let mut rng = Rng::new(5);
        let player =
            vec![PlayerPresence { area: 0, classroom: 0, students: 60, weeks_open: 100, best_teacher: Some((42, 90)) }];
        let mut found = false;
        for _ in 0..60 {
            if let (_, Some(p)) = competitor_turn(&mut comps, &areas, &player, &mut rng, date()) {
                assert_eq!(p.teacher_id, 42);
                found = true;
                break;
            }
        }
        assert!(found);
    }
}
