//! ゲーム内のエンティティ。

use serde::{Deserialize, Serialize};

/// 金額（円）。浮動小数点の誤差を避けるため整数で持つ。
pub type Yen = i64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchoolTier {
    /// 最難関
    Top,
    /// 難関
    Upper,
    /// 中堅
    Middle,
    /// 公立中高一貫
    Public,
}

impl SchoolTier {
    pub fn label(self) -> &'static str {
        match self {
            SchoolTier::Top => "最難関",
            SchoolTier::Upper => "難関",
            SchoolTier::Middle => "中堅",
            SchoolTier::Public => "公立一貫",
        }
    }

    /// 合格実績としての重み。
    pub fn weight(self) -> f64 {
        match self {
            SchoolTier::Top => 3.0,
            SchoolTier::Upper => 2.0,
            SchoolTier::Public => 1.5,
            SchoolTier::Middle => 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct School {
    pub name: String,
    pub deviation: f64,
    pub tier: SchoolTier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Budget {
    Low,
    Mid,
    High,
}

impl Budget {
    /// 月謝をどこまで許容できるか（市場標準に対する倍率）。
    pub fn tolerance(self) -> f64 {
        match self {
            Budget::Low => 0.9,
            Budget::Mid => 1.15,
            Budget::High => 1.5,
        }
    }

    /// 季節講習・特訓の受講率。
    pub fn extra_course_rate(self) -> f64 {
        match self {
            Budget::Low => 0.55,
            Budget::Mid => 0.85,
            Budget::High => 0.97,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParentType {
    /// 教育熱心
    Eager,
    /// おおらか
    Relaxed,
    /// 心配性
    Anxious,
}

impl ParentType {
    pub fn label(self) -> &'static str {
        match self {
            ParentType::Eager => "教育熱心",
            ParentType::Relaxed => "おおらか",
            ParentType::Anxious => "心配性",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Student {
    pub id: u32,
    pub name: String,
    /// 学年（4〜6）
    pub grade: u8,
    /// 偏差値
    pub deviation: f64,
    /// 入塾時の偏差値
    pub start_deviation: f64,
    /// 意欲 0..1
    pub motivation: f64,
    /// ストレス 0..1
    pub stress: f64,
    /// 第一志望校（`World::schools` の添字）
    pub target_school: usize,
    pub budget: Budget,
    pub parent: ParentType,
    pub weeks_enrolled: u32,
    /// 特待生
    pub scholarship: bool,
    /// 今季の講習・特訓を受講中
    pub taking_extra: bool,
    /// 直近の偏差値（グラフ用、最大 26 週）
    pub history: Vec<f32>,
}

impl Student {
    pub fn concern(&self) -> f64 {
        let s = (self.stress - 0.5).max(0.0) * 2.0;
        let m = (0.5 - self.motivation).max(0.0) * 2.0;
        s.max(m)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeacherKind {
    /// 正社員講師
    Pro,
    /// 学生講師
    Student,
}

impl TeacherKind {
    pub fn label(self) -> &'static str {
        match self {
            TeacherKind::Pro => "社員講師",
            TeacherKind::Student => "学生講師",
        }
    }

    /// 週あたりの担当可能時間。
    pub fn weekly_hours(self) -> f64 {
        match self {
            TeacherKind::Pro => 28.0,
            TeacherKind::Student => 12.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Teacher {
    pub id: u32,
    pub name: String,
    pub kind: TeacherKind,
    /// 授業力 0..100
    pub skill: u32,
    /// 面倒見 0..100
    pub care: u32,
    /// 社員講師は年俸、学生講師は時給
    pub pay: Yen,
    /// 疲労 0..1
    pub fatigue: f64,
}

impl Teacher {
    pub fn weekly_cost(&self, summer: bool) -> Yen {
        match self.kind {
            TeacherKind::Pro => self.pay / 52,
            TeacherKind::Student => {
                let hours = self.kind.weekly_hours() * if summer { 1.8 } else { 1.0 };
                (self.pay as f64 * hours) as Yen
            }
        }
    }

    pub fn weekly_hours(&self, summer: bool) -> f64 {
        let base = self.kind.weekly_hours();
        match (self.kind, summer) {
            (TeacherKind::Student, true) => base * 1.8,
            (TeacherKind::Pro, true) => base * 1.2,
            _ => base,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CourseKind {
    /// 学年別本科（月謝）
    Main(u8),
    /// 夏期講習（総額）
    Summer,
    /// 6 年志望校別特訓（月額）
    SpecialTraining,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Course {
    pub kind: CourseKind,
    pub name: String,
    /// 本科・特訓は月額、講習は総額
    pub fee: Yen,
    pub enabled: bool,
    /// 定員（本科のみ有効）
    pub capacity: u32,
    /// 先週の受講者数
    pub last_enrolled: u32,
    pub prev_enrolled: u32,
    /// 授業評価 0..5
    pub rating: f64,
}

impl Course {
    pub fn fee_step(&self) -> Yen {
        1_000
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignKind {
    FreeTrial,
    ResultsFlyer,
    ParentSeminar,
    StudyRoom,
    SiblingDiscount,
    Scholarship,
    SafetyMail,
    WebAds,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CostModel {
    /// 週あたり固定費
    Weekly(Yen),
    /// 授業料に対する割合
    RevenueRate(f64),
    /// 新規体験生 1 人あたり
    PerTrial(Yen),
}

impl CampaignKind {
    pub fn cost_model(self) -> CostModel {
        match self {
            CampaignKind::FreeTrial => CostModel::PerTrial(4_000),
            CampaignKind::ResultsFlyer => CostModel::Weekly(30_000),
            CampaignKind::ParentSeminar => CostModel::Weekly(15_000),
            CampaignKind::StudyRoom => CostModel::Weekly(22_000),
            CampaignKind::SiblingDiscount => CostModel::RevenueRate(0.03),
            CampaignKind::Scholarship => CostModel::RevenueRate(0.05),
            CampaignKind::SafetyMail => CostModel::Weekly(8_000),
            CampaignKind::WebAds => CostModel::Weekly(40_000),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Campaign {
    pub kind: CampaignKind,
    pub name: String,
    pub description: String,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Delegation {
    /// 講師の採用・シフト
    Staffing,
    /// カリキュラム・料金
    Pricing,
    /// 生徒募集
    Marketing,
    /// 保護者対応・トラブル
    Trouble,
}

impl Delegation {
    pub const ALL: [Delegation; 4] =
        [Delegation::Staffing, Delegation::Pricing, Delegation::Marketing, Delegation::Trouble];

    pub fn label(self) -> &'static str {
        match self {
            Delegation::Staffing => "講師の採用とシフトを任せる",
            Delegation::Pricing => "コースの料金設定を任せる",
            Delegation::Marketing => "生徒募集の施策を任せる",
            Delegation::Trouble => "保護者対応とトラブルを任せる",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manager {
    pub name: String,
    /// 講師管理
    pub staffing: u32,
    /// 指導・カリキュラム
    pub teaching: u32,
    /// 営業（入塾面談）
    pub sales: u32,
    /// 倫理
    pub ethics: u32,
    pub salary: Yen,
    pub hired: crate::calendar::Date,
    pub delegations: [bool; 4],
}

impl Manager {
    pub fn delegated(&self, d: Delegation) -> bool {
        self.delegations[d.index()]
    }

    pub fn set_delegated(&mut self, d: Delegation, on: bool) {
        self.delegations[d.index()] = on;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Competitor {
    pub name: String,
    pub style: String,
    pub address: String,
    pub brand: f64,
    pub quality: f64,
    pub care: f64,
    pub fees: [Yen; 3],
    /// 学年別の在籍数（4・5・6 年）
    pub students: [u32; 3],
    pub color: [u8; 3],
}

impl Competitor {
    pub fn total_students(&self) -> u32 {
        self.students.iter().sum()
    }
}

/// 口コミ。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Review {
    pub stars: u8,
    pub author: String,
    pub text: String,
    pub positive: bool,
}

/// 評価の 4 軸（0..5）。
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Ratings {
    pub price: f64,
    pub teaching: f64,
    pub care: f64,
    pub safety: f64,
}

impl Ratings {
    pub fn overall(&self) -> f64 {
        (self.price + self.teaching + self.care + self.safety) / 4.0
    }
}
