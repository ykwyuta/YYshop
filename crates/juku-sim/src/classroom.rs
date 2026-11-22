//! 1 つの教室。生徒・講師・教室長・コース・施策・口コミを持ち、週ごとに経営が進む。

use serde::{Deserialize, Serialize};

use crate::calendar::{Calendar, Date, Season};
use crate::data;
use crate::gen;
use crate::market::{price_score, Area, Competitor};
use crate::materials::Materials;
use crate::model::*;
use crate::report::*;
use crate::rng::Rng;

/// 1 クラスの定員。
pub const CLASS_SIZE: u32 = 20;
/// 学年別・1 クラスあたりの週授業時間。
const HOURS_PER_CLASS: [f64; 3] = [8.0, 12.0, 16.0];
/// 生徒 1 人あたりの質問対応・採点時間（週）。
const SUPPORT_HOURS_PER_STUDENT: f64 = 0.5;
/// 週あたりの市場の入塾希望者（4・5・6 年）。
pub const BASE_POOL: [f64; 3] = [3.6, 1.9, 0.95];
/// 事務員・光熱費など（固定分）。
const OVERHEAD_BASE_PER_WEEK: Yen = 40_000;
/// 事務員・光熱費など（生徒 1 人あたり）。
const OVERHEAD_PER_STUDENT_WEEK: Yen = 700;
/// 塾選びの「勝者総取り」の強さ（魅力度の何乗でシェアが決まるか）。
pub const SHARE_EXPONENT: i32 = 3;
const DEPRECIATION_PER_WEEK: Yen = 25_000;
const ADMISSION_FEE: Yen = 20_000;
const MATERIALS_FEE_MONTHLY: Yen = 4_500;
const TAX_RATE: f64 = 0.30;
pub const SUMMER_WEEKS: f64 = 6.0;
const MAX_REVIEWS: usize = 8;

pub(crate) fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// 月額を週額に換算。
pub(crate) fn monthly_to_weekly(m: Yen) -> f64 {
    m as f64 * 12.0 / 52.0
}

/// 魅力度をシェアの重みに変換する（整数乗なので環境差が出ない）。
pub(crate) fn share_weight(x: f64) -> f64 {
    let mut w = 1.0;
    for _ in 0..SHARE_EXPONENT {
        w *= x;
    }
    w
}

pub fn season_pool_multiplier(season: Season) -> [f64; 3] {
    match season {
        Season::NewTerm => [3.5, 2.0, 1.2],
        Season::Spring => [1.0, 1.0, 0.8],
        Season::Summer => [1.8, 1.6, 1.0],
        Season::Autumn => [0.8, 0.6, 0.3],
        Season::ExamRun => [0.6, 0.4, 0.05],
    }
}

/// 週の計算途中で使う、教室の状態のまとめ。
#[derive(Clone, Copy, Debug)]
pub struct ClassroomStats {
    /// 授業品質 0..1
    pub quality: f64,
    /// 面倒見 0..1
    pub care: f64,
    /// 講師の担当可能時間 / 必要時間
    pub coverage: f64,
    pub needed_hours: f64,
    pub available_hours: f64,
}

/// 週の処理で教室が使う、会社全体の情報。
pub struct WeekCtx<'a> {
    pub rng: &'a mut Rng,
    pub next_id: &'a mut u32,
    pub schools: &'a [School],
    pub materials: &'a Materials,
    pub calendar: Calendar,
    pub prev_calendar: Option<Calendar>,
    pub brand: f64,
    pub trust: f64,
    pub cash: Yen,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Classroom {
    pub id: u32,
    pub name: String,
    /// `World::areas` の添字
    pub area: usize,
    pub address: String,
    pub opened: Date,
    /// エリアでの認知度 0..1
    pub awareness: f64,
    /// 宿題量 1..5
    pub homework: u8,
    pub students: Vec<Student>,
    pub teachers: Vec<Teacher>,
    pub candidates: Vec<Teacher>,
    pub manager: Manager,
    pub courses: Vec<Course>,
    pub campaigns: Vec<Campaign>,
    pub ratings: Ratings,
    pub review_count: u32,
    pub reviews: Vec<Review>,
    pub history: Vec<WeekReport>,
    /// 教室の物件を所有している（家賃がかからない）
    #[serde(default)]
    pub owned: bool,
    /// 所有物件の評価額
    #[serde(default)]
    pub property_value: Yen,
}

/// 物件維持費（固定資産税・修繕費）の年率。
pub const PROPERTY_UPKEEP_RATE: f64 = 0.015;

impl Course {
    pub(crate) fn new(kind: CourseKind, name: &str, fee: Yen) -> Self {
        Course {
            kind,
            name: name.into(),
            fee,
            enabled: true,
            capacity: 60,
            last_enrolled: 0,
            prev_enrolled: 0,
            rating: 3.0,
        }
    }
}

pub(crate) fn default_courses() -> Vec<Course> {
    vec![
        Course::new(CourseKind::Main(4), "4年 本科", 25_000),
        Course::new(CourseKind::Main(5), "5年 本科", 40_000),
        Course::new(CourseKind::Main(6), "6年 本科", 55_000),
        Course::new(CourseKind::Summer, "夏期講習", 120_000),
        Course::new(CourseKind::SpecialTraining, "6年 志望校別特訓", 30_000),
    ]
}

pub(crate) fn default_campaigns() -> Vec<Campaign> {
    data::CAMPAIGNS
        .iter()
        .map(|c| Campaign {
            kind: c.kind,
            name: c.name.into(),
            description: c.description.into(),
            active: matches!(c.kind, CampaignKind::FreeTrial),
        })
        .collect()
}

impl Classroom {
    /// 新しい教室。生徒はいない状態から始まる。
    pub fn new(id: u32, name: String, area: usize, address: String, opened: Date, manager: Manager) -> Self {
        Classroom {
            id,
            name,
            area,
            address,
            opened,
            awareness: 0.15,
            homework: 3,
            students: Vec::new(),
            teachers: Vec::new(),
            candidates: Vec::new(),
            manager,
            courses: default_courses(),
            campaigns: default_campaigns(),
            ratings: Ratings { price: 3.2, teaching: 3.0, care: 3.0, safety: 3.0 },
            review_count: 0,
            reviews: Vec::new(),
            history: Vec::new(),
            owned: false,
            property_value: 0,
        }
    }

    pub fn star_teachers(&self) -> usize {
        self.teachers.iter().filter(|t| t.star).count()
    }

    /// 週あたりの物件費用（家賃 または 維持費）。
    pub fn weekly_property_cost(&self, area: &Area) -> Yen {
        if self.owned {
            (self.property_value as f64 * PROPERTY_UPKEEP_RATE / 52.0) as Yen
        } else {
            area.rent
        }
    }

    // ------------------------------------------------------------------
    // 参照
    // ------------------------------------------------------------------

    pub fn course(&self, kind: CourseKind) -> Option<&Course> {
        self.courses.iter().find(|c| c.kind == kind)
    }

    pub fn main_fee(&self, grade: u8) -> Yen {
        self.course(CourseKind::Main(grade)).map(|c| c.fee).unwrap_or(0)
    }

    pub fn main_enabled(&self, grade: u8) -> bool {
        self.course(CourseKind::Main(grade)).map(|c| c.enabled).unwrap_or(false)
    }

    pub fn campaign_active(&self, kind: CampaignKind) -> bool {
        self.campaigns.iter().any(|c| c.kind == kind && c.active)
    }

    pub fn students_in_grade(&self, grade: u8) -> u32 {
        self.students.iter().filter(|s| s.grade == grade).count() as u32
    }

    pub fn last_report(&self) -> Option<&WeekReport> {
        self.history.last()
    }

    /// 生徒の中で「気になる」順（ストレス過多・意欲低下）。
    pub fn students_of_concern(&self, limit: usize) -> Vec<&Student> {
        let mut v: Vec<&Student> = self.students.iter().filter(|s| s.concern() > 0.2).collect();
        v.sort_by(|a, b| b.concern().partial_cmp(&a.concern()).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(limit);
        v
    }

    pub fn best_teacher(&self) -> Option<&Teacher> {
        self.teachers.iter().max_by_key(|t| t.skill)
    }

    pub fn stats(&self, calendar: &Calendar, materials: &Materials) -> ClassroomStats {
        let summer = calendar.is_summer_course();
        let training = calendar.is_special_training();
        let mut needed = 0.0;
        for g in 4..=6u8 {
            let n = self.students_in_grade(g);
            needed += n.div_ceil(CLASS_SIZE) as f64 * HOURS_PER_CLASS[(g - 4) as usize];
        }
        needed += self.students.len() as f64 * SUPPORT_HOURS_PER_STUDENT;
        let extra = self.students.iter().filter(|s| s.taking_extra).count() as u32;
        if summer {
            needed += extra.div_ceil(CLASS_SIZE) as f64 * 12.0;
        }
        if training {
            needed += extra.div_ceil(CLASS_SIZE) as f64 * 4.0;
        }

        let available: f64 = self.teachers.iter().map(|t| t.weekly_hours(summer)).sum();
        let coverage = if needed > 0.0 { available / needed } else { 2.0 };

        let (mut skill_sum, mut care_sum, mut hour_sum) = (0.0, 0.0, 0.0);
        for t in &self.teachers {
            let h = t.weekly_hours(summer);
            skill_sum += t.skill as f64 * h * (1.0 - 0.3 * t.fatigue);
            care_sum += t.care as f64 * h;
            hour_sum += h;
        }
        let mut quality = if hour_sum > 0.0 { skill_sum / hour_sum / 100.0 } else { 0.0 };
        quality += 0.1 * (self.manager.teaching as f64 - 50.0) / 50.0;
        quality += materials.quality_bonus();
        if coverage < 1.0 {
            quality *= 0.6 + 0.4 * coverage;
        }
        let mut care = if hour_sum > 0.0 { care_sum / hour_sum / 100.0 * 0.7 } else { 0.0 };
        if self.campaign_active(CampaignKind::ParentSeminar) {
            care += 0.1;
        }
        if self.campaign_active(CampaignKind::StudyRoom) {
            care += 0.08;
        }
        care += 0.1 * self.manager.staffing as f64 / 100.0;
        if coverage < 1.0 {
            care *= 0.5 + 0.5 * coverage;
        }
        ClassroomStats {
            quality: clamp01(quality),
            care: clamp01(care),
            coverage,
            needed_hours: needed,
            available_hours: available,
        }
    }

    /// 学年 `gi`（0..3）での自塾の魅力度。
    pub fn attractiveness(&self, gi: usize, stats: &ClassroomStats, brand: f64, trust: f64) -> f64 {
        let grade = gi as u8 + 4;
        if !self.main_enabled(grade) {
            return 0.0;
        }
        0.30 * brand
            + 0.28 * stats.quality
            + 0.16 * stats.care
            + 0.14 * price_score(self.main_fee(grade), data::REFERENCE_FEES[gi])
            + 0.12 * self.awareness
            + 0.05 * (self.ratings.overall() - 3.0)
            + 0.05 * (trust - 0.5)
            + 0.05 * self.star_teachers().min(2) as f64
    }

    /// コース 1 人あたりの月間原価（講師コストの按分＋教材原価）。
    pub fn course_unit_cost(&self, kind: CourseKind, calendar: &Calendar, materials: &Materials) -> Yen {
        let summer = calendar.is_summer_course();
        let teacher_weekly: Yen = self.teachers.iter().map(|t| t.weekly_cost(summer)).sum();
        let stats = self.stats(calendar, materials);
        let per_hour = if stats.available_hours > 0.0 { teacher_weekly as f64 / stats.available_hours } else { 0.0 };
        let hours_per_student = match kind {
            CourseKind::Main(g) => HOURS_PER_CLASS[(g - 4) as usize] / CLASS_SIZE as f64 + SUPPORT_HOURS_PER_STUDENT,
            CourseKind::Summer => 1.2 * SUMMER_WEEKS / (52.0 / 12.0),
            CourseKind::SpecialTraining => 4.0 / CLASS_SIZE as f64,
        };
        let monthly_teacher = per_hour * hours_per_student * 52.0 / 12.0;
        let materials_cost = match kind {
            CourseKind::Main(_) => materials.cost_per_student_monthly() as f64,
            _ => 1_500.0,
        };
        (monthly_teacher + materials_cost) as Yen
    }

    // ------------------------------------------------------------------
    // 操作
    // ------------------------------------------------------------------

    pub fn set_course_fee(&mut self, index: usize, fee: Yen) {
        if let Some(c) = self.courses.get_mut(index) {
            c.fee = fee.clamp(0, 500_000);
        }
    }

    pub fn set_course_enabled(&mut self, index: usize, enabled: bool) {
        if let Some(c) = self.courses.get_mut(index) {
            c.enabled = enabled;
        }
    }

    pub fn set_homework(&mut self, level: u8) {
        self.homework = level.clamp(1, 5);
    }

    pub fn set_campaign(&mut self, index: usize, active: bool) {
        if let Some(c) = self.campaigns.get_mut(index) {
            c.active = active;
        }
    }

    pub fn hire_candidate(&mut self, index: usize) -> bool {
        if index < self.candidates.len() {
            let t = self.candidates.remove(index);
            self.teachers.push(t);
            true
        } else {
            false
        }
    }

    pub fn dismiss_teacher(&mut self, id: u32) {
        self.teachers.retain(|t| t.id != id);
    }

    pub fn set_manager_salary(&mut self, salary: Yen) {
        self.manager.salary = salary.clamp(3_000_000, 20_000_000);
    }

    pub fn set_delegation(&mut self, d: Delegation, on: bool) {
        self.manager.set_delegated(d, on);
    }

    // ------------------------------------------------------------------
    // 週送り
    // ------------------------------------------------------------------

    /// 週の前半：委任業務と講習の受講決定。市場計算の前に呼ぶ。
    pub(crate) fn begin_week(&mut self, ctx: &mut WeekCtx, news: &mut Vec<String>) {
        if ctx.calendar.week.is_multiple_of(4) {
            self.run_delegations(ctx, news);
            self.candidates = gen::make_candidates(ctx.rng, ctx.next_id);
        }
        self.decide_extra_courses(ctx);
        self.awareness *= 0.97;
        let brand = ctx.brand;
        if self.campaign_active(CampaignKind::ResultsFlyer) {
            self.awareness += 0.04 * (0.5 + brand);
        }
        if self.campaign_active(CampaignKind::WebAds) {
            self.awareness += 0.05;
        }
        if self.campaign_active(CampaignKind::ParentSeminar) {
            self.awareness += 0.02;
        }
        self.awareness = clamp01(self.awareness);
    }

    /// 入塾者を受け入れる。`inquiries` は学年別の問い合わせ数（期待値）。
    pub(crate) fn admit(&mut self, ctx: &mut WeekCtx, inquiries: [f64; 3]) -> (u32, f64) {
        let mut conversion = 0.45 + 0.3 * self.manager.sales as f64 / 100.0;
        if self.campaign_active(CampaignKind::FreeTrial) {
            conversion += 0.15;
        }
        if self.campaign_active(CampaignKind::SiblingDiscount) {
            conversion += 0.05;
        }
        let scholarship = self.campaign_active(CampaignKind::Scholarship);
        let mut new_students = 0;
        for gi in 0..3 {
            let grade = gi as u8 + 4;
            let capacity = self
                .course(CourseKind::Main(grade))
                .map(|c| c.capacity)
                .unwrap_or(0)
                .saturating_sub(self.students_in_grade(grade));
            let n = ctx.rng.round_stochastic(inquiries[gi] * conversion.min(0.95)).min(capacity);
            for _ in 0..n {
                let sch = scholarship && ctx.rng.chance(0.2);
                let s = gen::make_student(ctx.rng, ctx.next_id, ctx.schools, grade, sch);
                self.students.push(s);
                new_students += 1;
            }
        }
        (new_students, inquiries.iter().sum())
    }

    /// 週の後半：学習・退塾・講師・会計・口コミ。レポートを返す。
    pub(crate) fn finish_week(
        &mut self,
        ctx: &mut WeekCtx,
        area: &Area,
        new_students: u32,
        trials: f64,
        market_share: f64,
        mut news: Vec<String>,
    ) -> WeekReport {
        let season = ctx.calendar.season();
        let summer = ctx.calendar.is_summer_course();
        let training = ctx.calendar.is_special_training();
        let stats = self.stats(&ctx.calendar, ctx.materials);
        if new_students > 0 {
            news.insert(0, format!("新しく {new_students} 人が入塾しました"));
        }

        // --- 学習・ストレス・意欲 ---
        let hw = self.homework as f64;
        let study_room = self.campaign_active(CampaignKind::StudyRoom);
        let exam_run = season == Season::ExamRun;
        for s in self.students.iter_mut() {
            let noise_dev = ctx.rng.normal(0.0, 0.35);
            let noise_stress = ctx.rng.normal(0.0, 0.01);
            let noise_mot = ctx.rng.normal(0.0, 0.015);
            let mut mean = (stats.quality - 0.45) * 0.20 + (s.motivation - 0.5) * 0.08 + (hw - 3.0) * 0.045
                - (s.stress - 0.75).max(0.0) * 0.4
                + ctx.materials.learning_bonus(s.deviation);
            if study_room {
                mean += 0.02;
            }
            if s.taking_extra && (summer || (training && s.grade == 6)) {
                mean += 0.045;
            }
            let before = s.deviation;
            s.deviation = (s.deviation + mean + noise_dev).clamp(25.0, 78.0);
            let delta = s.deviation - before;

            let mut ds =
                0.0105 * (hw - 2.5) - stats.care * 0.02 - 0.004 + noise_stress + ctx.materials.stress_delta(before);
            if summer && s.taking_extra {
                ds += 0.01;
            }
            if exam_run && s.grade == 6 {
                ds += 0.01;
            }
            if s.parent == ParentType::Eager {
                ds += 0.003;
            }
            s.stress = clamp01(s.stress + ds);
            s.motivation = clamp01(
                s.motivation + delta * 0.03 + (stats.quality - 0.5) * 0.01 - (s.stress - 0.6).max(0.0) * 0.05
                    + (0.6 - s.motivation) * 0.01
                    + ctx.materials.motivation_delta()
                    + noise_mot,
            );
            s.weeks_enrolled += 1;
            s.history.push(s.deviation as f32);
            if s.history.len() > 26 {
                s.history.remove(0);
            }
        }

        // --- 退塾 ---
        let sibling = self.campaign_active(CampaignKind::SiblingDiscount);
        let mut churned = 0;
        let mut churn_news = Vec::new();
        let mut i = 0;
        while i < self.students.len() {
            let s = &self.students[i];
            let gi = (s.grade - 4) as usize;
            let fee_ratio = self.main_fee(s.grade) as f64 / data::REFERENCE_FEES[gi] as f64;
            let price_pain = (fee_ratio - s.budget.tolerance()).max(0.0);
            let mut p =
                0.0015 + (s.stress - 0.7).max(0.0) * 0.15 + (0.35 - s.motivation).max(0.0) * 0.08 + price_pain * 0.02
                    - stats.care * 0.001
                    + (1.0 - stats.coverage).max(0.0) * 0.06;
            if sibling {
                p -= 0.0008;
            }
            if s.grade == 6 && matches!(season, Season::Autumn | Season::ExamRun) {
                p *= 0.3;
            }
            if !self.main_enabled(s.grade) {
                p = 1.0;
            }
            if ctx.rng.chance(p.max(0.0)) {
                let reason = if s.stress > 0.7 {
                    "勉強の負担が大きすぎる"
                } else if price_pain > 0.0 {
                    "月謝が高い"
                } else if s.motivation < 0.35 {
                    "やる気が続かない"
                } else {
                    "家庭の事情"
                };
                churn_news.push(format!("{}（{}年）が退塾：{}", s.name, s.grade, reason));
                self.students.remove(i);
                churned += 1;
            } else {
                i += 1;
            }
        }
        news.extend(churn_news.into_iter().take(3));

        // --- 講師の疲労・離職 ---
        let load = if stats.available_hours > 0.0 { stats.needed_hours / stats.available_hours } else { 0.0 };
        let mut quit = Vec::new();
        for t in self.teachers.iter_mut() {
            t.loyal_weeks = t.loyal_weeks.saturating_sub(1);
            t.fatigue = clamp01(t.fatigue + (load - 1.0) * 0.1 - 0.01);
            if t.fatigue > 0.9 && ctx.rng.chance(0.04) {
                quit.push(t.id);
                news.push(format!("{}（{}）が疲労を理由に退職しました", t.name, t.kind.label()));
            }
        }
        self.teachers.retain(|t| !quit.contains(&t.id));
        if stats.coverage < 0.9 && !self.students.is_empty() {
            news.push(format!(
                "講師が足りていません（充足率 {:.0}%）。講師の疲労がたまっています",
                stats.coverage * 100.0
            ));
        }

        // --- 会計 ---
        let pnl = self.compute_pnl(ctx, area, summer, training, new_students, trials);

        // --- 評価・口コミ ---
        self.update_ratings(&stats, ctx.trust);
        self.generate_reviews(ctx.rng, &stats);
        self.update_course_counts(summer, training);

        let mut avg_dev = [0.0; 3];
        for (gi, avg) in avg_dev.iter_mut().enumerate() {
            let g = gi as u8 + 4;
            let devs: Vec<f64> = self.students.iter().filter(|s| s.grade == g).map(|s| s.deviation).collect();
            if !devs.is_empty() {
                *avg = devs.iter().sum::<f64>() / devs.len() as f64;
            }
        }
        let report = WeekReport {
            date: ctx.calendar.date(),
            school_year: ctx.calendar.school_year,
            week: ctx.calendar.week,
            pnl,
            students: [self.students_in_grade(4), self.students_in_grade(5), self.students_in_grade(6)],
            new_students,
            churned,
            market_share,
            ratings: self.ratings,
            cash: ctx.cash,
            avg_deviation: avg_dev,
            coverage: stats.coverage,
            news,
        };
        self.history.push(report.clone());
        report
    }

    fn decide_extra_courses(&mut self, ctx: &mut WeekCtx) {
        let summer = ctx.calendar.is_summer_course();
        let training = ctx.calendar.is_special_training();
        let summer_on = self.course(CourseKind::Summer).map(|c| c.enabled).unwrap_or(false);
        let training_on = self.course(CourseKind::SpecialTraining).map(|c| c.enabled).unwrap_or(false);
        let summer_fee = self.course(CourseKind::Summer).map(|c| c.fee).unwrap_or(0);
        let training_fee = self.course(CourseKind::SpecialTraining).map(|c| c.fee).unwrap_or(0);
        let was_summer = ctx.prev_calendar.map(|c| c.is_summer_course()).unwrap_or(false);
        let was_training = ctx.prev_calendar.map(|c| c.is_special_training()).unwrap_or(false);

        if summer && !was_summer {
            let ratio = summer_fee as f64 / 120_000.0;
            for s in self.students.iter_mut() {
                let rate = s.budget.extra_course_rate() * clamp01(1.5 - 0.5 * ratio);
                s.taking_extra = summer_on && ctx.rng.chance(rate);
            }
        } else if !summer && was_summer {
            for s in self.students.iter_mut() {
                s.taking_extra = false;
            }
        }
        if training && !was_training {
            let ratio = training_fee as f64 / 30_000.0;
            for s in self.students.iter_mut().filter(|s| s.grade == 6) {
                let rate = s.budget.extra_course_rate() * clamp01(1.5 - 0.5 * ratio);
                s.taking_extra = training_on && ctx.rng.chance(rate);
            }
        }
    }

    fn compute_pnl(
        &self,
        ctx: &WeekCtx,
        area: &Area,
        summer: bool,
        training: bool,
        new_students: u32,
        trials: f64,
    ) -> Pnl {
        let mut pnl = Pnl::default();
        let tuition: f64 = self.students.iter().map(|s| monthly_to_weekly(self.main_fee(s.grade))).sum();
        pnl.tuition = tuition as Yen;
        if summer {
            let fee = self.course(CourseKind::Summer).map(|c| c.fee).unwrap_or(0);
            let n = self.students.iter().filter(|s| s.taking_extra).count() as f64;
            pnl.extra_courses = (n * fee as f64 / SUMMER_WEEKS) as Yen;
        } else if training {
            let fee = self.course(CourseKind::SpecialTraining).map(|c| c.fee).unwrap_or(0);
            let n = self.students.iter().filter(|s| s.taking_extra && s.grade == 6).count();
            pnl.extra_courses = (n as f64 * monthly_to_weekly(fee)) as Yen;
        }
        pnl.admission = new_students as Yen * ADMISSION_FEE;
        let n = self.students.len() as f64;
        pnl.materials_revenue = (n * monthly_to_weekly(MATERIALS_FEE_MONTHLY)) as Yen;

        pnl.teacher_cost = self.teachers.iter().map(|t| t.weekly_cost(summer)).sum();
        pnl.manager_cost = self.manager.salary / 52;
        if self.owned {
            pnl.property_upkeep = self.weekly_property_cost(area);
        } else {
            pnl.rent = area.rent;
        }
        pnl.materials_cost = (n * monthly_to_weekly(ctx.materials.cost_per_student_monthly())) as Yen;
        pnl.depreciation = DEPRECIATION_PER_WEEK;
        pnl.other = OVERHEAD_BASE_PER_WEEK + OVERHEAD_PER_STUDENT_WEEK * self.students.len() as Yen;

        let mut marketing = 0.0;
        for c in self.campaigns.iter().filter(|c| c.active) {
            marketing += match c.kind.cost_model() {
                CostModel::Weekly(y) => y as f64,
                CostModel::RevenueRate(r) => r * tuition,
                CostModel::PerTrial(y) => y as f64 * trials,
            };
        }
        pnl.marketing = marketing as Yen;

        let op = pnl.operating_profit();
        pnl.tax = if op > 0 { (op as f64 * TAX_RATE) as Yen } else { 0 };
        pnl
    }

    fn update_ratings(&mut self, stats: &ClassroomStats, trust: f64) {
        let mut price_sum = 0.0;
        let mut weight = 0.0;
        for gi in 0..3 {
            let g = gi as u8 + 4;
            let n = self.students_in_grade(g) as f64 + 1.0;
            price_sum += price_score(self.main_fee(g), data::REFERENCE_FEES[gi]) * n;
            weight += n;
        }
        let mut safety = 2.6 + (trust - 0.5) * 2.0;
        if self.campaign_active(CampaignKind::SafetyMail) {
            safety += 1.0;
        }
        if self.campaign_active(CampaignKind::StudyRoom) {
            safety += 0.4;
        }
        let target = Ratings {
            price: (1.0 + 4.0 * price_sum / weight).clamp(1.0, 5.0),
            teaching: (1.0 + 4.0 * stats.quality).clamp(1.0, 5.0),
            care: (1.0 + 4.0 * stats.care).clamp(1.0, 5.0),
            safety: safety.clamp(1.0, 5.0),
        };
        let blend = |a: f64, b: f64| a * 0.8 + b * 0.2;
        self.ratings = Ratings {
            price: blend(self.ratings.price, target.price),
            teaching: blend(self.ratings.teaching, target.teaching),
            care: blend(self.ratings.care, target.care),
            safety: blend(self.ratings.safety, target.safety),
        };
    }

    fn update_course_counts(&mut self, summer: bool, training: bool) {
        for ci in 0..self.courses.len() {
            let kind = self.courses[ci].kind;
            let n = match kind {
                CourseKind::Main(g) => self.students_in_grade(g),
                CourseKind::Summer if summer => self.students.iter().filter(|s| s.taking_extra).count() as u32,
                CourseKind::SpecialTraining if training => {
                    self.students.iter().filter(|s| s.taking_extra && s.grade == 6).count() as u32
                }
                _ => 0,
            };
            let fee_score = match kind {
                CourseKind::Main(g) => price_score(self.courses[ci].fee, data::REFERENCE_FEES[(g - 4) as usize]),
                _ => 0.5,
            };
            let teaching = self.ratings.teaching;
            let c = &mut self.courses[ci];
            c.prev_enrolled = c.last_enrolled;
            c.last_enrolled = n;
            c.rating = (teaching * 0.75 + (1.0 + 4.0 * fee_score) * 0.25).min(5.0);
        }
    }

    fn generate_reviews(&mut self, rng: &mut Rng, stats: &ClassroomStats) {
        if self.students.is_empty() {
            return;
        }
        let count = 1 + rng.below(2);
        for _ in 0..count {
            let s = self.students[rng.below(self.students.len())].clone();
            let gi = (s.grade - 4) as usize;
            let fee_ratio = self.main_fee(s.grade) as f64 / data::REFERENCE_FEES[gi] as f64;
            let mut options: Vec<(u8, String)> = Vec::new();
            if s.stress > 0.7 {
                options.push((2, "宿題が多すぎて、子どもが疲れ切っています。".into()));
            }
            let gain = s.deviation - s.start_deviation;
            if gain > 4.0 {
                options.push((5, format!("入塾してから偏差値が {gain:.0} 上がりました！先生方に感謝しています。")));
            }
            if fee_ratio > s.budget.tolerance() {
                options.push((2, "月謝が家計に重く、続けられるか悩んでいます。".into()));
            }
            if s.weeks_enrolled < 6 && self.campaign_active(CampaignKind::FreeTrial) {
                options.push((5, "無料体験の授業がわかりやすくて、入塾を決めました。".into()));
            }
            if self.campaign_active(CampaignKind::StudyRoom) {
                options.push((4, "自習室が使えるので、毎日のように通っています。".into()));
            }
            if self.campaign_active(CampaignKind::SafetyMail) && s.parent == ParentType::Anxious {
                options.push((4, "入退室メールが届くので安心して通わせられます。".into()));
            }
            if stats.coverage < 0.95 {
                options.push((2, "先生が足りていないのか、質問できる時間が少ないです。".into()));
            }
            if stats.care > 0.6 {
                options.push((5, "面談で丁寧に相談に乗ってもらえました。".into()));
            }
            if self.campaign_active(CampaignKind::Scholarship) && !s.scholarship {
                options.push((2, "特待生ばかり優遇されている気がします。".into()));
            }
            if s.motivation > 0.75 {
                options.push((5, "子どもが「塾が楽しい」と言うようになりました。".into()));
            }
            if let Some(star) = self.teachers.iter().find(|t| t.star) {
                options.push((5, format!("{}先生の授業が目当てで入塾しました。評判どおりです。", star.name)));
            }
            if options.is_empty() {
                let neutral: [(u8, &str); 6] = [
                    (4, "落ち着いた雰囲気で、楽しく通っています。"),
                    (4, "先生の説明がていねいで、わかりやすいそうです。"),
                    (3, "可もなく不可もなく、といったところです。"),
                    (4, "友だちと励まし合いながら頑張っています。"),
                    (3, "成績はまだ横ばいですが、本人は前向きです。"),
                    (4, "質問しやすい雰囲気なのがありがたいです。"),
                ];
                let (st, text) = neutral[rng.below(neutral.len())];
                options.push((st, text.into()));
            }
            let (stars, text) = options[rng.below(options.len())].clone();
            self.reviews.insert(
                0,
                Review {
                    stars,
                    author: format!("{}年生の保護者（{}）", s.grade, s.parent.label()),
                    text,
                    positive: stars >= 4,
                },
            );
            self.review_count += 1;
        }
        self.reviews.truncate(MAX_REVIEWS);
    }

    fn run_delegations(&mut self, ctx: &mut WeekCtx, news: &mut Vec<String>) {
        let m = self.manager.clone();
        if m.delegated(Delegation::Staffing) {
            let stats = self.stats(&ctx.calendar, ctx.materials);
            if (stats.coverage < 1.05 || self.teachers.len() < 2) && !self.candidates.is_empty() {
                let idx = if ctx.rng.chance(m.staffing as f64 / 100.0) {
                    self.candidates.iter().enumerate().max_by_key(|(_, t)| t.skill).map(|(i, _)| i).unwrap_or(0)
                } else {
                    ctx.rng.below(self.candidates.len())
                };
                let name = self.candidates[idx].name.clone();
                self.hire_candidate(idx);
                news.push(format!("教室長が講師 {name} を採用しました"));
            } else if stats.coverage > 1.5 && self.teachers.len() > 2 {
                if let Some(t) =
                    self.teachers.iter().filter(|t| t.kind == TeacherKind::Student).min_by_key(|t| t.skill).cloned()
                {
                    self.dismiss_teacher(t.id);
                    news.push(format!("教室長が講師 {} のシフトを打ち切りました", t.name));
                }
            }
        }
        if m.delegated(Delegation::Pricing) {
            let stats = self.stats(&ctx.calendar, ctx.materials);
            for c in self.courses.iter_mut() {
                if let CourseKind::Main(g) = c.kind {
                    let reference = data::REFERENCE_FEES[(g - 4) as usize] as f64;
                    let target = reference * (0.85 + 0.25 * stats.quality + 0.1 * ctx.brand);
                    let target = (target / 1000.0) as Yen * 1000;
                    if c.fee < target {
                        c.fee += 1000;
                    } else if c.fee > target {
                        c.fee -= 1000;
                    }
                }
            }
        }
        if m.delegated(Delegation::Marketing) {
            let rich = ctx.cash > 3_000_000;
            let brand = ctx.brand;
            let skill = m.sales as f64 / 100.0;
            for c in self.campaigns.iter_mut() {
                c.active = match c.kind {
                    CampaignKind::FreeTrial | CampaignKind::SafetyMail => true,
                    CampaignKind::ResultsFlyer => brand > 0.4 && rich,
                    CampaignKind::ParentSeminar => skill > 0.6,
                    CampaignKind::WebAds => rich && skill > 0.7,
                    CampaignKind::StudyRoom => rich,
                    _ => c.active,
                };
            }
        }
    }
}

/// エリアの入塾希望者を、自塾と競合で分け合う。自塾の問い合わせ数（学年別）とシェアを返す。
pub(crate) fn distribute_area_demand(
    area_idx: usize,
    area: &Area,
    season: Season,
    ours: Option<[f64; 3]>,
    competitors: &mut [Competitor],
    rng: &mut Rng,
) -> [f64; 3] {
    let pools = season_pool_multiplier(season);
    let mut inquiries = [0.0; 3];
    for gi in 0..3 {
        let pool = BASE_POOL[gi] * pools[gi] * area.pool;
        let ours_w = ours.map(|a| share_weight(a[gi].max(0.0))).unwrap_or(0.0);
        let comp_w: Vec<f64> = competitors
            .iter()
            .map(|c| c.branch(area_idx).map(|b| share_weight(c.attractiveness(b, gi))).unwrap_or(0.0))
            .collect();
        let total = ours_w + comp_w.iter().sum::<f64>();
        if total <= 0.0 {
            continue;
        }
        inquiries[gi] = pool * ours_w / total;
        for (ci, c) in competitors.iter_mut().enumerate() {
            if comp_w[ci] <= 0.0 {
                continue;
            }
            let gain = rng.round_stochastic(pool * comp_w[ci] / total * 0.65);
            if let Some(b) = c.branch_mut(area_idx) {
                b.students[gi] += gain;
            }
        }
    }
    inquiries
}
