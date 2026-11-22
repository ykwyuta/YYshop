//! ゲーム世界の状態と、週送りの処理。

use serde::{Deserialize, Serialize};

use crate::calendar::{Calendar, Season};
use crate::data;
use crate::model::*;
use crate::report::*;
use crate::rng::Rng;

/// 1 クラスの定員。
const CLASS_SIZE: u32 = 20;
/// 学年別・1 クラスあたりの週授業時間。
const HOURS_PER_CLASS: [f64; 3] = [5.0, 8.0, 11.0];
/// 生徒 1 人あたりの質問対応・採点時間（週）。
const SUPPORT_HOURS_PER_STUDENT: f64 = 0.4;
/// 週あたりの市場の入塾希望者（4・5・6 年）。
const BASE_POOL: [f64; 3] = [3.0, 1.6, 0.8];
const RENT_PER_WEEK: Yen = 120_000;
/// 事務員・光熱費など。
const OVERHEAD_PER_WEEK: Yen = 70_000;
const DEPRECIATION_PER_WEEK: Yen = 25_000;
const ADMISSION_FEE: Yen = 20_000;
const MATERIALS_FEE_MONTHLY: Yen = 4_500;
const MATERIALS_COST_MONTHLY: Yen = 2_600;
const TAX_RATE: f64 = 0.30;
const SUMMER_WEEKS: f64 = 6.0;
const MAX_REVIEWS: usize = 8;

pub const SAVE_VERSION: u32 = 1;

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// 月額を週額に換算。
fn monthly_to_weekly(m: Yen) -> f64 {
    m as f64 * 12.0 / 52.0
}

/// ロジスティック関数の代わり（超越関数を使わない）。x=0 で 0.5。
fn soft_step(x: f64) -> f64 {
    0.5 + 0.5 * x / (1.0 + x.abs())
}

fn pow4(x: f64) -> f64 {
    let x2 = x * x;
    x2 * x2
}

/// 市場標準に対する価格の魅力（0..1）。
fn price_score(fee: Yen, reference: Yen) -> f64 {
    let ratio = fee as f64 / reference as f64;
    clamp01(1.3 - 0.8 * ratio)
}

fn season_pool_multiplier(season: Season) -> [f64; 3] {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub save_version: u32,
    pub rng: Rng,
    pub calendar: Calendar,
    pub classroom_name: String,
    pub address: String,
    pub cash: Yen,
    /// ブランド力（合格実績）0..1
    pub brand: f64,
    /// 地域での認知度 0..1
    pub awareness: f64,
    /// 隠しパラメータ：信頼 0..1
    pub trust: f64,
    /// 宿題量 1..5
    pub homework: u8,
    pub students: Vec<Student>,
    pub teachers: Vec<Teacher>,
    pub candidates: Vec<Teacher>,
    pub manager: Manager,
    pub courses: Vec<Course>,
    pub campaigns: Vec<Campaign>,
    pub competitors: Vec<Competitor>,
    pub schools: Vec<School>,
    pub ratings: Ratings,
    pub review_count: u32,
    pub reviews: Vec<Review>,
    pub history: Vec<WeekReport>,
    pub exam_results: Vec<ExamResult>,
    pub pending_event: Option<GameEvent>,
    /// 新しい入試結果が出て、まだ表示していない
    pub unseen_exam_result: bool,
    pub padding_this_year: bool,
    next_id: u32,
    weeks_since_event: u32,
}

impl World {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let calendar = Calendar::new(2027);
        let schools = data::SCHOOLS
            .iter()
            .map(|s| School { name: s.name.into(), deviation: s.deviation, tier: s.tier })
            .collect();
        let competitors = data::COMPETITORS
            .iter()
            .map(|c| Competitor {
                name: c.name.into(),
                style: c.style.into(),
                address: c.address.into(),
                brand: c.brand,
                quality: c.quality,
                care: c.care,
                fees: c.fees,
                students: c.students,
                color: c.color,
            })
            .collect();
        let campaigns = data::CAMPAIGNS
            .iter()
            .map(|c| Campaign {
                kind: c.kind,
                name: c.name.into(),
                description: c.description.into(),
                active: matches!(c.kind, CampaignKind::FreeTrial),
            })
            .collect();
        let courses = vec![
            Course::new(CourseKind::Main(4), "4年 本科", 25_000),
            Course::new(CourseKind::Main(5), "5年 本科", 40_000),
            Course::new(CourseKind::Main(6), "6年 本科", 55_000),
            Course::new(CourseKind::Summer, "夏期講習", 120_000),
            Course::new(CourseKind::SpecialTraining, "6年 志望校別特訓", 30_000),
        ];
        let manager = Manager {
            name: (*rng.pick(data::MANAGER_NAMES)).into(),
            staffing: 55 + rng.below(30) as u32,
            teaching: 55 + rng.below(30) as u32,
            sales: 55 + rng.below(30) as u32,
            ethics: 50 + rng.below(40) as u32,
            salary: 6_000_000,
            hired: calendar.date(),
            delegations: [false; 4],
        };

        let mut world = World {
            save_version: SAVE_VERSION,
            rng,
            calendar,
            classroom_name: "あおぞら進学塾".into(),
            address: "けやき台2-5-33".into(),
            cash: 8_000_000,
            brand: 0.35,
            awareness: 0.4,
            trust: 0.6,
            homework: 3,
            students: Vec::new(),
            teachers: Vec::new(),
            candidates: Vec::new(),
            manager,
            courses,
            campaigns,
            competitors,
            schools,
            ratings: Ratings { price: 3.4, teaching: 3.3, care: 3.4, safety: 3.2 },
            review_count: 128,
            reviews: Vec::new(),
            history: Vec::new(),
            exam_results: Vec::new(),
            pending_event: None,
            unseen_exam_result: false,
            padding_this_year: false,
            next_id: 1,
            weeks_since_event: 0,
        };

        let pro = world.make_teacher(TeacherKind::Pro);
        world.teachers.push(pro);
        for _ in 0..4 {
            let t = world.make_teacher(TeacherKind::Student);
            world.teachers.push(t);
        }
        for (grade, n) in [(4u8, 22), (5, 24), (6, 20)] {
            for _ in 0..n {
                let mut s = world.make_student(grade, false);
                s.weeks_enrolled = 4 + world.rng.below(60) as u32;
                world.students.push(s);
            }
        }
        world.refresh_candidates();
        for c in world.courses.iter_mut() {
            if let CourseKind::Main(g) = c.kind {
                let n = world.students.iter().filter(|s| s.grade == g).count() as u32;
                c.last_enrolled = n;
                c.prev_enrolled = n;
            }
            c.rating = 3.4;
        }
        world
    }

    // ------------------------------------------------------------------
    // 生成
    // ------------------------------------------------------------------

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn random_name(&mut self) -> String {
        let s = *self.rng.pick(data::SURNAMES);
        let g = *self.rng.pick(data::GIVEN_NAMES);
        format!("{s} {g}")
    }

    fn make_student(&mut self, grade: u8, scholarship_candidate: bool) -> Student {
        let id = self.alloc_id();
        let name = self.random_name();
        let deviation = if scholarship_candidate { self.rng.normal(63.0, 4.0) } else { self.rng.normal(48.0, 9.0) }
            .clamp(28.0, 75.0);
        let budget = match self.rng.below(10) {
            0..=2 => Budget::Low,
            3..=7 => Budget::Mid,
            _ => Budget::High,
        };
        let parent = match self.rng.below(10) {
            0..=3 => ParentType::Eager,
            4..=6 => ParentType::Relaxed,
            _ => ParentType::Anxious,
        };
        let aim = deviation + self.rng.range(2.0, 9.0);
        let prefer_public = self.rng.chance(0.15);
        let target_school = self.school_near(aim, prefer_public);
        Student {
            id,
            name,
            grade,
            deviation,
            start_deviation: deviation,
            motivation: self.rng.range(0.45, 0.8),
            stress: self.rng.range(0.1, 0.35),
            target_school,
            budget,
            parent,
            weeks_enrolled: 0,
            scholarship: scholarship_candidate,
            taking_extra: false,
            history: vec![deviation as f32],
        }
    }

    /// 偏差値 `aim` に最も近い学校。
    fn school_near(&self, aim: f64, prefer_public: bool) -> usize {
        let mut best = 0;
        let mut best_d = f64::MAX;
        for (i, s) in self.schools.iter().enumerate() {
            if prefer_public != (s.tier == SchoolTier::Public) {
                continue;
            }
            let d = (s.deviation - aim).abs();
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        best
    }

    fn make_teacher(&mut self, kind: TeacherKind) -> Teacher {
        let id = self.alloc_id();
        let name = self.random_name();
        let (skill, care, pay) = match kind {
            TeacherKind::Pro => {
                let skill = self.rng.normal(66.0, 11.0).clamp(35.0, 98.0);
                let care = self.rng.normal(55.0, 15.0).clamp(10.0, 98.0);
                let pay = 3_800_000.0 + (skill - 40.0) * 55_000.0 + self.rng.range(0.0, 300_000.0);
                (skill, care, (pay / 10_000.0) as Yen * 10_000)
            }
            TeacherKind::Student => {
                let skill = self.rng.normal(50.0, 12.0).clamp(20.0, 90.0);
                let care = self.rng.normal(55.0, 15.0).clamp(10.0, 98.0);
                let pay = 1_300.0 + (skill - 20.0) * 14.0;
                (skill, care, (pay / 50.0) as Yen * 50)
            }
        };
        Teacher { id, name, kind, skill: skill as u32, care: care as u32, pay, fatigue: 0.1 }
    }

    fn refresh_candidates(&mut self) {
        self.candidates.clear();
        let pro = self.make_teacher(TeacherKind::Pro);
        self.candidates.push(pro);
        for _ in 0..2 {
            let t = self.make_teacher(TeacherKind::Student);
            self.candidates.push(t);
        }
    }

    // ------------------------------------------------------------------
    // 参照
    // ------------------------------------------------------------------

    pub fn course(&self, kind: CourseKind) -> Option<&Course> {
        self.courses.iter().find(|c| c.kind == kind)
    }

    fn main_fee(&self, grade: u8) -> Yen {
        self.course(CourseKind::Main(grade)).map(|c| c.fee).unwrap_or(0)
    }

    fn main_enabled(&self, grade: u8) -> bool {
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

    /// 企業価値の目安。
    pub fn valuation(&self) -> Yen {
        let recent: Vec<&WeekReport> = self.history.iter().rev().take(13).collect();
        let annual = if recent.is_empty() {
            0.0
        } else {
            recent.iter().map(|r| r.pnl.net_profit() as f64).sum::<f64>() / recent.len() as f64 * 52.0
        };
        self.cash + (annual.max(0.0) * 6.0) as Yen + (self.brand * 30_000_000.0) as Yen
    }

    /// コース 1 人あたりの月間原価（講師コストの按分＋教材原価）。
    pub fn course_unit_cost(&self, kind: CourseKind) -> Yen {
        let summer = self.calendar.is_summer_course();
        let teacher_weekly: Yen = self.teachers.iter().map(|t| t.weekly_cost(summer)).sum();
        let stats = self.classroom_stats();
        let per_hour = if stats.available_hours > 0.0 { teacher_weekly as f64 / stats.available_hours } else { 0.0 };
        let hours_per_student = match kind {
            CourseKind::Main(g) => HOURS_PER_CLASS[(g - 4) as usize] / CLASS_SIZE as f64 + SUPPORT_HOURS_PER_STUDENT,
            CourseKind::Summer => 1.2 * SUMMER_WEEKS / (52.0 / 12.0),
            CourseKind::SpecialTraining => 4.0 / CLASS_SIZE as f64,
        };
        let monthly_teacher = per_hour * hours_per_student * 52.0 / 12.0;
        let materials = match kind {
            CourseKind::Main(_) => MATERIALS_COST_MONTHLY as f64,
            _ => 1_500.0,
        };
        (monthly_teacher + materials) as Yen
    }

    pub fn classroom_stats(&self) -> ClassroomStats {
        let summer = self.calendar.is_summer_course();
        let training = self.calendar.is_special_training();
        let mut needed = 0.0;
        for g in 4..=6u8 {
            let n = self.students_in_grade(g);
            let classes = n.div_ceil(CLASS_SIZE);
            needed += classes as f64 * HOURS_PER_CLASS[(g - 4) as usize];
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
            let fatigue_penalty = 1.0 - 0.3 * t.fatigue;
            skill_sum += t.skill as f64 * h * fatigue_penalty;
            care_sum += t.care as f64 * h;
            hour_sum += h;
        }
        let mut quality = if hour_sum > 0.0 { skill_sum / hour_sum / 100.0 } else { 0.0 };
        quality += 0.1 * (self.manager.teaching as f64 - 50.0) / 50.0;
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

    // ------------------------------------------------------------------
    // プレイヤーの操作
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

    pub fn resolve_event(&mut self, choice: Choice) {
        let Some(ev) = self.pending_event.take() else { return };
        self.apply_event_choice(ev.kind, choice);
    }

    fn apply_event_choice(&mut self, kind: EventKind, choice: Choice) {
        match (kind, choice) {
            (EventKind::HomeworkRequest, Choice::A) => {
                self.homework = (self.homework + 1).min(5);
                self.trust -= 0.02;
            }
            (EventKind::HomeworkRequest, Choice::B) => {
                self.cash -= 60_000;
                self.trust += 0.03;
                for s in self.students.iter_mut().filter(|s| s.parent == ParentType::Eager) {
                    s.motivation = clamp01(s.motivation + 0.05);
                }
            }
            (EventKind::StrugglingSenior { student_id }, Choice::A) => {
                let safer = self
                    .students
                    .iter()
                    .find(|s| s.id == student_id)
                    .map(|s| self.school_near(s.deviation - 3.0, false));
                if let (Some(safer), Some(s)) = (safer, self.students.iter_mut().find(|s| s.id == student_id)) {
                    s.target_school = safer;
                    s.motivation = clamp01(s.motivation - 0.2);
                }
                self.trust -= 0.04;
            }
            (EventKind::StrugglingSenior { student_id }, Choice::B) => {
                self.cash -= 80_000;
                if let Some(s) = self.students.iter_mut().find(|s| s.id == student_id) {
                    s.motivation = clamp01(s.motivation + 0.2);
                    s.deviation += 1.5;
                }
                self.trust += 0.04;
            }
            (EventKind::Poaching { teacher_id }, Choice::A) => {
                if let Some(t) = self.teachers.iter_mut().find(|t| t.id == teacher_id) {
                    t.pay = t.pay * 6 / 5;
                    t.fatigue = (t.fatigue - 0.2).max(0.0);
                }
            }
            (EventKind::Poaching { teacher_id }, Choice::B) => {
                self.dismiss_teacher(teacher_id);
            }
            (EventKind::ResultPadding, Choice::A) => {
                self.padding_this_year = true;
                self.trust -= 0.15;
            }
            (EventKind::ResultPadding, Choice::B) => {
                self.padding_this_year = false;
                self.trust += 0.05;
            }
        }
        self.trust = clamp01(self.trust);
    }

    // ------------------------------------------------------------------
    // 週送り
    // ------------------------------------------------------------------

    /// 1 週進める。判断待ちのイベントがある場合は進めない（false を返す）。
    pub fn step_week(&mut self) -> bool {
        if self.pending_event.is_some() {
            return false;
        }
        let mut news = Vec::new();
        let season = self.calendar.season();
        let summer = self.calendar.is_summer_course();
        let training = self.calendar.is_special_training();
        let week = self.calendar.week;

        if week.is_multiple_of(4) {
            self.run_delegations(&mut news);
            self.refresh_candidates();
        }

        // 季節講習・特訓の受講決定
        self.decide_extra_courses(summer, training);

        let stats = self.classroom_stats();

        // --- 集客 ---
        self.awareness *= 0.97;
        if self.campaign_active(CampaignKind::ResultsFlyer) {
            self.awareness += 0.04 * (0.5 + self.brand);
        }
        if self.campaign_active(CampaignKind::WebAds) {
            self.awareness += 0.05;
        }
        if self.campaign_active(CampaignKind::ParentSeminar) {
            self.awareness += 0.02;
        }
        self.awareness = clamp01(self.awareness);

        let pools = season_pool_multiplier(season);
        let mut new_students = 0u32;
        let mut trial_count = 0.0;
        let mut our_attr_sum = 0.0;
        let mut all_attr_sum = 0.0;
        for gi in 0..3 {
            let grade = gi as u8 + 4;
            let pool = BASE_POOL[gi] * pools[gi];
            let ours = if self.main_enabled(grade) {
                0.30 * self.brand
                    + 0.25 * stats.quality
                    + 0.15 * stats.care
                    + 0.20 * price_score(self.main_fee(grade), data::REFERENCE_FEES[gi])
                    + 0.10 * self.awareness
                    + 0.05 * (self.ratings.overall() - 3.0)
            } else {
                0.0
            };
            let ours_w = pow4(ours.max(0.0));
            let comp_w: Vec<f64> = self
                .competitors
                .iter()
                .map(|c| {
                    pow4(
                        0.30 * c.brand
                            + 0.25 * c.quality
                            + 0.15 * c.care
                            + 0.20 * price_score(c.fees[gi], data::REFERENCE_FEES[gi])
                            + 0.06,
                    )
                })
                .collect();
            let total_w = ours_w + comp_w.iter().sum::<f64>();
            our_attr_sum += ours_w;
            all_attr_sum += total_w;
            if total_w <= 0.0 {
                continue;
            }

            let mut conversion = 0.45 + 0.3 * self.manager.sales as f64 / 100.0;
            if self.campaign_active(CampaignKind::FreeTrial) {
                conversion += 0.15;
            }
            if self.campaign_active(CampaignKind::SiblingDiscount) {
                conversion += 0.05;
            }
            let inquiries = pool * ours_w / total_w;
            trial_count += inquiries;
            let capacity = self
                .course(CourseKind::Main(grade))
                .map(|c| c.capacity)
                .unwrap_or(0)
                .saturating_sub(self.students_in_grade(grade));
            let n = self.rng.round_stochastic(inquiries * conversion.min(0.95)).min(capacity);
            for _ in 0..n {
                let sch = self.campaign_active(CampaignKind::Scholarship) && self.rng.chance(0.2);
                let s = self.make_student(grade, sch);
                self.students.push(s);
                new_students += 1;
            }
            for (ci, c) in self.competitors.iter_mut().enumerate() {
                let gain = pool * comp_w[ci] / total_w * 0.6;
                c.students[gi] += self.rng.round_stochastic(gain);
            }
        }
        if new_students > 0 {
            news.push(format!("新しく {new_students} 人が入塾しました"));
        }

        // --- 学習・ストレス・意欲 ---
        let hw = self.homework as f64;
        let study_room = self.campaign_active(CampaignKind::StudyRoom);
        let exam_run = season == Season::ExamRun;
        for i in 0..self.students.len() {
            let noise_dev = self.rng.normal(0.0, 0.35);
            let noise_stress = self.rng.normal(0.0, 0.01);
            let noise_mot = self.rng.normal(0.0, 0.015);
            let s = &mut self.students[i];
            let mut mean = (stats.quality - 0.45) * 0.20 + (s.motivation - 0.5) * 0.08 + (hw - 3.0) * 0.025
                - (s.stress - 0.75).max(0.0) * 0.4;
            if study_room {
                mean += 0.02;
            }
            if s.taking_extra && (summer || (training && s.grade == 6)) {
                mean += 0.045;
            }
            let before = s.deviation;
            s.deviation = (s.deviation + mean + noise_dev).clamp(25.0, 78.0);
            let delta = s.deviation - before;

            let mut ds = 0.012 * (hw - 2.5) - stats.care * 0.02 - 0.004 + noise_stress;
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
        let mut churn_reasons: Vec<String> = Vec::new();
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
            if self.rng.chance(p.max(0.0)) {
                let reason = if s.stress > 0.7 {
                    "勉強の負担が大きすぎる"
                } else if price_pain > 0.0 {
                    "月謝が高い"
                } else if s.motivation < 0.35 {
                    "やる気が続かない"
                } else {
                    "家庭の事情"
                };
                churn_reasons.push(format!("{}（{}年）が退塾：{}", s.name, s.grade, reason));
                self.students.remove(i);
                churned += 1;
            } else {
                i += 1;
            }
        }
        news.extend(churn_reasons.into_iter().take(3));

        // --- 講師の疲労・離職 ---
        let load = if stats.available_hours > 0.0 { stats.needed_hours / stats.available_hours } else { 0.0 };
        let mut quit = Vec::new();
        for t in self.teachers.iter_mut() {
            t.fatigue = clamp01(t.fatigue + (load - 1.0) * 0.1 - 0.01);
            if t.fatigue > 0.9 && self.rng.chance(0.04) {
                quit.push(t.id);
                news.push(format!("{}（{}）が疲労を理由に退職しました", t.name, t.kind.label()));
            }
        }
        self.teachers.retain(|t| !quit.contains(&t.id));
        if stats.coverage < 0.9 {
            news.push(format!(
                "講師が足りていません（充足率 {:.0}%）。講師の疲労がたまっています",
                stats.coverage * 100.0
            ));
        }

        // --- 競合 ---
        for c in self.competitors.iter_mut() {
            for gi in 0..3 {
                let leave = self.rng.round_stochastic(c.students[gi] as f64 * 0.004);
                c.students[gi] = c.students[gi].saturating_sub(leave);
            }
        }
        if self.rng.chance(0.02) {
            let ci = self.rng.below(self.competitors.len());
            let c = &mut self.competitors[ci];
            for f in c.fees.iter_mut() {
                *f = (*f * 92 / 100 / 1000) * 1000;
            }
            news.push(format!("{}が月謝を値下げしました", c.name));
        }

        // --- 会計 ---
        let pnl = self.compute_pnl(&stats, summer, training, new_students, trial_count);
        self.cash += pnl.net_profit();

        // --- 評価・口コミ ---
        self.update_ratings(&stats);
        self.generate_reviews(&stats);

        // --- コースの受講者数 ---
        for ci in 0..self.courses.len() {
            let kind = self.courses[ci].kind;
            let n = match kind {
                CourseKind::Main(g) => self.students_in_grade(g),
                CourseKind::Summer => {
                    if summer {
                        self.students.iter().filter(|s| s.taking_extra).count() as u32
                    } else {
                        0
                    }
                }
                CourseKind::SpecialTraining => {
                    if training {
                        self.students.iter().filter(|s| s.taking_extra && s.grade == 6).count() as u32
                    } else {
                        0
                    }
                }
            };
            let fee_score = match kind {
                CourseKind::Main(g) => price_score(self.courses[ci].fee, data::REFERENCE_FEES[(g - 4) as usize]),
                _ => 0.5,
            };
            let c = &mut self.courses[ci];
            c.prev_enrolled = c.last_enrolled;
            c.last_enrolled = n;
            c.rating = (self.ratings.teaching * 0.75 + (1.0 + 4.0 * fee_score) * 0.25).min(5.0);
        }

        // --- 不祥事 ---
        if self.trust < 0.35 && self.rng.chance((0.35 - self.trust) * 0.15) {
            self.brand = (self.brand - 0.1).max(0.0);
            self.awareness = (self.awareness - 0.1).max(0.0);
            self.ratings.safety = (self.ratings.safety - 1.0).max(1.0);
            self.trust += 0.1;
            news.push("口コミサイトで指導方針が批判され、炎上しました".into());
        }

        // --- レポート ---
        let mut avg_dev = [0.0; 3];
        for (gi, avg) in avg_dev.iter_mut().enumerate() {
            let g = gi as u8 + 4;
            let devs: Vec<f64> = self.students.iter().filter(|s| s.grade == g).map(|s| s.deviation).collect();
            if !devs.is_empty() {
                *avg = devs.iter().sum::<f64>() / devs.len() as f64;
            }
        }
        let comp_students: u32 = self.competitors.iter().map(|c| c.total_students()).sum();
        let ours = self.students.len() as u32;
        let report = WeekReport {
            date: self.calendar.date(),
            school_year: self.calendar.school_year,
            week,
            pnl,
            students: [self.students_in_grade(4), self.students_in_grade(5), self.students_in_grade(6)],
            new_students,
            churned,
            market_share: if ours + comp_students > 0 { ours as f64 / (ours + comp_students) as f64 } else { 0.0 },
            ratings: self.ratings,
            cash: self.cash,
            avg_deviation: avg_dev,
            coverage: stats.coverage,
            news,
        };
        let _ = (our_attr_sum, all_attr_sum);
        self.history.push(report);

        // --- イベント ---
        self.weeks_since_event += 1;
        self.maybe_trigger_event();

        // --- 暦を進める ---
        if self.calendar.advance() {
            self.run_entrance_exams();
            self.promote_grades();
        }
        true
    }

    fn decide_extra_courses(&mut self, summer: bool, training: bool) {
        let summer_on = self.course(CourseKind::Summer).map(|c| c.enabled).unwrap_or(false);
        let training_on = self.course(CourseKind::SpecialTraining).map(|c| c.enabled).unwrap_or(false);
        let summer_fee = self.course(CourseKind::Summer).map(|c| c.fee).unwrap_or(0);
        let training_fee = self.course(CourseKind::SpecialTraining).map(|c| c.fee).unwrap_or(0);
        let prev = self.history.last().map(|r| Calendar { school_year: r.school_year, week: r.week });
        let was_summer = prev.map(|c| c.is_summer_course()).unwrap_or(false);
        let was_training = prev.map(|c| c.is_special_training()).unwrap_or(false);

        if summer && !was_summer {
            let ratio = summer_fee as f64 / 120_000.0;
            for i in 0..self.students.len() {
                let rate = self.students[i].budget.extra_course_rate() * clamp01(1.5 - 0.5 * ratio);
                let take = summer_on && self.rng.chance(rate);
                self.students[i].taking_extra = take;
            }
        } else if !summer && was_summer {
            for s in self.students.iter_mut() {
                s.taking_extra = false;
            }
        }
        if training && !was_training {
            let ratio = training_fee as f64 / 30_000.0;
            for i in 0..self.students.len() {
                if self.students[i].grade != 6 {
                    continue;
                }
                let rate = self.students[i].budget.extra_course_rate() * clamp01(1.5 - 0.5 * ratio);
                let take = training_on && self.rng.chance(rate);
                self.students[i].taking_extra = take;
            }
        }
    }

    fn compute_pnl(&self, stats: &ClassroomStats, summer: bool, training: bool, new_students: u32, trials: f64) -> Pnl {
        let mut pnl = Pnl::default();
        let mut tuition = 0.0;
        for s in &self.students {
            tuition += monthly_to_weekly(self.main_fee(s.grade));
        }
        pnl.tuition = tuition as Yen;

        let extra_n = self.students.iter().filter(|s| s.taking_extra).count() as f64;
        if summer {
            let fee = self.course(CourseKind::Summer).map(|c| c.fee).unwrap_or(0);
            pnl.extra_courses = (extra_n * fee as f64 / SUMMER_WEEKS) as Yen;
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
        pnl.rent = RENT_PER_WEEK;
        pnl.materials_cost = (n * monthly_to_weekly(MATERIALS_COST_MONTHLY)) as Yen;
        pnl.depreciation = DEPRECIATION_PER_WEEK;
        pnl.other = OVERHEAD_PER_WEEK;

        let mut marketing = 0.0;
        for c in self.campaigns.iter().filter(|c| c.active) {
            marketing += match c.kind.cost_model() {
                CostModel::Weekly(y) => y as f64,
                CostModel::RevenueRate(r) => r * tuition,
                CostModel::PerTrial(y) => y as f64 * trials,
            };
        }
        pnl.marketing = marketing as Yen;
        let _ = stats;

        let op = pnl.operating_profit();
        pnl.tax = if op > 0 { (op as f64 * TAX_RATE) as Yen } else { 0 };
        pnl
    }

    fn update_ratings(&mut self, stats: &ClassroomStats) {
        let mut price_sum = 0.0;
        let mut weight = 0.0;
        for gi in 0..3 {
            let g = gi as u8 + 4;
            let n = self.students_in_grade(g) as f64 + 1.0;
            price_sum += price_score(self.main_fee(g), data::REFERENCE_FEES[gi]) * n;
            weight += n;
        }
        let price = 1.0 + 4.0 * price_sum / weight;
        let teaching = 1.0 + 4.0 * stats.quality;
        let care = 1.0 + 4.0 * stats.care;
        let mut safety = 2.6 + (self.trust - 0.5) * 2.0;
        if self.campaign_active(CampaignKind::SafetyMail) {
            safety += 1.0;
        }
        if self.campaign_active(CampaignKind::StudyRoom) {
            safety += 0.4;
        }
        let target = Ratings {
            price: price.clamp(1.0, 5.0),
            teaching: teaching.clamp(1.0, 5.0),
            care: care.clamp(1.0, 5.0),
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

    fn generate_reviews(&mut self, stats: &ClassroomStats) {
        if self.students.is_empty() {
            return;
        }
        let count = 1 + self.rng.below(2);
        for _ in 0..count {
            let idx = self.rng.below(self.students.len());
            let s = self.students[idx].clone();
            let gi = (s.grade - 4) as usize;
            let fee_ratio = self.main_fee(s.grade) as f64 / data::REFERENCE_FEES[gi] as f64;
            let mut options: Vec<(u8, String)> = Vec::new();
            if s.stress > 0.7 {
                options.push((2, "宿題が多すぎて、子どもが疲れ切っています。".into()));
            }
            let gain = s.deviation - s.start_deviation;
            if gain > 4.0 {
                options.push((5, format!("入塾してから偏差値が {:.0} 上がりました！先生方に感謝しています。", gain)));
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
            if options.is_empty() {
                options.push((4, "落ち着いた雰囲気で、楽しく通っています。".into()));
            }
            let (stars, text) = options[self.rng.below(options.len())].clone();
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

    fn maybe_trigger_event(&mut self) {
        if self.pending_event.is_some() || self.weeks_since_event < 5 {
            return;
        }
        let season = self.calendar.season();
        let week = self.calendar.week;
        let mut ev: Option<GameEvent> = None;

        if week == 49 && self.students.iter().any(|s| s.grade == 6 && s.scholarship) {
            ev = Some(GameEvent {
                kind: EventKind::ResultPadding,
                title: "合格実績の「上積み」提案".into(),
                body:
                    "本部スタッフが「特待生に多くの学校を受けてもらえば、合格者数を大きく見せられます」と提案してきた。"
                        .into(),
                choice_a: "特待生に多くの学校を受験させ、合格数を稼ぐ".into(),
                choice_b: "本人が志望する学校だけを受験させる".into(),
            });
        } else if self.rng.chance(0.06) {
            let pick = self.rng.below(3);
            match pick {
                0 => {
                    ev = Some(GameEvent {
                        kind: EventKind::HomeworkRequest,
                        title: "宿題を増やしてほしい".into(),
                        body: "教育熱心な保護者たちから「もっと宿題を出してほしい」という要望が届いた。".into(),
                        choice_a: "全体の宿題量を 1 段階増やす".into(),
                        choice_b: "希望する家庭にだけ個別課題を出す（¥60,000）".into(),
                    });
                }
                1 if matches!(season, Season::Autumn | Season::ExamRun) => {
                    let found = self
                        .students
                        .iter()
                        .filter(|s| s.grade == 6)
                        .find(|s| self.schools[s.target_school].deviation - s.deviation > 6.0)
                        .map(|s| (s.id, s.name.clone(), self.schools[s.target_school].name.clone()));
                    if let Some((id, name, school)) = found {
                        ev = Some(GameEvent {
                            kind: EventKind::StrugglingSenior { student_id: id },
                            title: "第一志望に届かない 6 年生".into(),
                            body: format!(
                                "{name}さんの成績は、第一志望の{school}にまだ届いていない。合格率を守るなら志望校を下げさせる手もあるが……"
                            ),
                            choice_a: "志望校を下げるよう説得する".into(),
                            choice_b: "挑戦を支える個別指導をつける（¥80,000）".into(),
                        });
                    }
                }
                _ => {
                    let found = self
                        .teachers
                        .iter()
                        .filter(|t| t.skill >= 70)
                        .max_by_key(|t| t.skill)
                        .map(|t| (t.id, t.name.clone()));
                    if let Some((id, name)) = found {
                        let rival = self.competitors[self.rng.below(self.competitors.len())].name.clone();
                        ev = Some(GameEvent {
                            kind: EventKind::Poaching { teacher_id: id },
                            title: "人気講師の引き抜き".into(),
                            body: format!("{rival}が、人気講師の{name}先生に好条件で移籍を持ちかけている。"),
                            choice_a: "給与を 2 割上げて引き止める".into(),
                            choice_b: "本人の意思を尊重して送り出す".into(),
                        });
                    }
                }
            }
        }

        if let Some(ev) = ev {
            self.weeks_since_event = 0;
            if self.manager.delegated(Delegation::Trouble) {
                let choice = if self.rng.chance(self.manager.ethics as f64 / 100.0) { Choice::B } else { Choice::A };
                let label = if choice == Choice::A { &ev.choice_a } else { &ev.choice_b };
                let msg = format!("教室長が「{}」に対応：{}", ev.title, label);
                self.apply_event_choice(ev.kind, choice);
                if let Some(r) = self.history.last_mut() {
                    r.news.push(msg);
                }
            } else {
                self.pending_event = Some(ev);
            }
        }
    }

    fn run_delegations(&mut self, news: &mut Vec<String>) {
        let m = self.manager.clone();
        if m.delegated(Delegation::Staffing) {
            let stats = self.classroom_stats();
            if stats.coverage < 1.05 && !self.candidates.is_empty() {
                let idx = if self.rng.chance(m.staffing as f64 / 100.0) {
                    self.candidates.iter().enumerate().max_by_key(|(_, t)| t.skill).map(|(i, _)| i).unwrap_or(0)
                } else {
                    self.rng.below(self.candidates.len())
                };
                let name = self.candidates[idx].name.clone();
                self.hire_candidate(idx);
                news.push(format!("教室長が講師 {name} を採用しました"));
            } else if stats.coverage > 1.5 {
                if let Some(t) =
                    self.teachers.iter().filter(|t| t.kind == TeacherKind::Student).min_by_key(|t| t.skill).cloned()
                {
                    self.dismiss_teacher(t.id);
                    news.push(format!("教室長が講師 {} のシフトを打ち切りました", t.name));
                }
            }
        }
        if m.delegated(Delegation::Pricing) {
            let stats = self.classroom_stats();
            for c in self.courses.iter_mut() {
                if let CourseKind::Main(g) = c.kind {
                    let reference = data::REFERENCE_FEES[(g - 4) as usize] as f64;
                    let target = reference * (0.85 + 0.25 * stats.quality + 0.1 * self.brand);
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
            let rich = self.cash > 3_000_000;
            let brand = self.brand;
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

    fn run_entrance_exams(&mut self) {
        let year = self.calendar.school_year - 1;
        let brand_before = self.brand;
        let mut entries = Vec::new();
        let mut tallies: Vec<SchoolTally> =
            (0..self.schools.len()).map(|i| SchoolTally { school: i, takers: 0, passes: 0 }).collect();
        let mut first_choice_passes = 0;
        let seniors: Vec<Student> = self.students.iter().filter(|s| s.grade == 6).cloned().collect();

        for s in &seniors {
            let mut plan = vec![s.target_school];
            // 併願校（安全校）
            let safety = self.school_near(s.deviation - 7.0, false);
            if safety != s.target_school {
                plan.push(safety);
            }
            if self.padding_this_year && s.scholarship {
                for (i, sch) in self.schools.iter().enumerate() {
                    if !plan.contains(&i) && sch.deviation <= s.deviation + 2.0 && plan.len() < 7 {
                        plan.push(i);
                    }
                }
            }
            for (k, &school) in plan.iter().enumerate() {
                let x = (s.deviation - self.schools[school].deviation + 2.0) / 4.0;
                let passed = self.rng.chance(soft_step(x));
                tallies[school].takers += 1;
                if passed {
                    tallies[school].passes += 1;
                }
                if k == 0 && passed {
                    first_choice_passes += 1;
                }
                entries.push(ExamEntry {
                    student_name: s.name.clone(),
                    school,
                    passed,
                    first_choice: k == 0,
                    start_deviation: s.start_deviation,
                    final_deviation: s.deviation,
                });
            }
        }

        let score: f64 = tallies.iter().map(|t| t.passes as f64 * self.schools[t.school].tier.weight()).sum();
        self.brand = clamp01(self.brand * 0.55 + clamp01(score / 60.0) * 0.45);

        let mut stories: Vec<&ExamEntry> = entries.iter().filter(|e| e.first_choice && e.passed).collect();
        stories.sort_by(|a, b| {
            let ga = self.schools[a.school].deviation - a.start_deviation;
            let gb = self.schools[b.school].deviation - b.start_deviation;
            gb.partial_cmp(&ga).unwrap_or(std::cmp::Ordering::Equal)
        });
        let highlights = stories
            .iter()
            .take(3)
            .map(|e| {
                format!(
                    "{}さん：入塾時の偏差値 {:.0} から、第一志望の{}に合格！",
                    e.student_name, e.start_deviation, self.schools[e.school].name
                )
            })
            .collect();

        tallies.retain(|t| t.takers > 0);
        tallies.sort_by(|a, b| {
            self.schools[b.school]
                .deviation
                .partial_cmp(&self.schools[a.school].deviation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.exam_results.push(ExamResult {
            school_year: year,
            candidates: seniors.len() as u32,
            entries,
            tallies,
            first_choice_passes,
            brand_before,
            brand_after: self.brand,
            padded: self.padding_this_year,
            highlights,
        });
        self.padding_this_year = false;
        self.unseen_exam_result = true;
    }

    fn promote_grades(&mut self) {
        self.students.retain(|s| s.grade < 6);
        for s in self.students.iter_mut() {
            s.grade += 1;
            s.taking_extra = false;
        }
        for c in self.competitors.iter_mut() {
            c.students = [0, c.students[0], c.students[1]];
        }
    }

    // ------------------------------------------------------------------
    // セーブ
    // ------------------------------------------------------------------

    /// 生徒の中で「気になる」順（ストレス過多・意欲低下）。
    pub fn students_of_concern(&self, limit: usize) -> Vec<&Student> {
        let mut v: Vec<&Student> = self.students.iter().filter(|s| s.concern() > 0.2).collect();
        v.sort_by(|a, b| b.concern().partial_cmp(&a.concern()).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(limit);
        v
    }
}

impl Course {
    fn new(kind: CourseKind, name: &str, fee: Yen) -> Self {
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
