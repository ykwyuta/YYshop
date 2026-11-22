//! 週次レポート・入試結果・イベント。

use serde::{Deserialize, Serialize};

use crate::calendar::Date;
use crate::model::{Ratings, Yen};

/// 週次の損益計算書。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pnl {
    // 売上
    pub tuition: Yen,
    pub extra_courses: Yen,
    pub admission: Yen,
    pub materials_revenue: Yen,
    // 費用
    pub teacher_cost: Yen,
    pub manager_cost: Yen,
    pub rent: Yen,
    pub materials_cost: Yen,
    pub marketing: Yen,
    pub depreciation: Yen,
    pub other: Yen,
    // 税
    pub tax: Yen,
}

impl Pnl {
    pub fn revenue_lines(&self) -> [(&'static str, Yen); 4] {
        [
            ("授業料", self.tuition),
            ("講習・特訓", self.extra_courses),
            ("入塾金", self.admission),
            ("模試・教材費", self.materials_revenue),
        ]
    }

    pub fn cost_lines(&self) -> [(&'static str, Yen); 7] {
        [
            ("講師人件費", self.teacher_cost),
            ("教室長給与", self.manager_cost),
            ("家賃", self.rent),
            ("教材・模試原価", self.materials_cost),
            ("広告宣伝費", self.marketing),
            ("減価償却費", self.depreciation),
            ("その他", self.other),
        ]
    }

    pub fn revenue(&self) -> Yen {
        self.revenue_lines().iter().map(|(_, v)| v).sum()
    }

    pub fn costs(&self) -> Yen {
        self.cost_lines().iter().map(|(_, v)| v).sum()
    }

    pub fn operating_profit(&self) -> Yen {
        self.revenue() - self.costs()
    }

    pub fn net_profit(&self) -> Yen {
        self.operating_profit() - self.tax
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeekReport {
    pub date: Date,
    pub school_year: i32,
    pub week: u32,
    pub pnl: Pnl,
    /// 学年別在籍数（4・5・6 年）
    pub students: [u32; 3],
    pub new_students: u32,
    pub churned: u32,
    /// 地域の生徒シェア（自塾）0..1
    pub market_share: f64,
    pub ratings: Ratings,
    pub cash: Yen,
    /// 平均偏差値（4・5・6 年）
    pub avg_deviation: [f64; 3],
    /// 授業の充足率（講師の担当可能時間 / 必要時間）
    pub coverage: f64,
    pub news: Vec<String>,
}

impl WeekReport {
    pub fn total_students(&self) -> u32 {
        self.students.iter().sum()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExamEntry {
    pub student_name: String,
    pub school: usize,
    pub passed: bool,
    pub first_choice: bool,
    pub start_deviation: f64,
    pub final_deviation: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchoolTally {
    pub school: usize,
    pub takers: u32,
    pub passes: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExamResult {
    pub school_year: i32,
    pub candidates: u32,
    pub entries: Vec<ExamEntry>,
    pub tallies: Vec<SchoolTally>,
    pub first_choice_passes: u32,
    pub brand_before: f64,
    pub brand_after: f64,
    /// 実績水増しを行った
    pub padded: bool,
    /// 心に残る合格ストーリー
    pub highlights: Vec<String>,
}

impl ExamResult {
    pub fn total_passes(&self) -> u32 {
        self.tallies.iter().map(|t| t.passes).sum()
    }

    pub fn first_choice_rate(&self) -> f64 {
        if self.candidates == 0 {
            0.0
        } else {
            self.first_choice_passes as f64 / self.candidates as f64
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    HomeworkRequest,
    StrugglingSenior { student_id: u32 },
    Poaching { teacher_id: u32 },
    ResultPadding,
}

/// 倫理ジレンマなどの判断イベント。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameEvent {
    pub kind: EventKind,
    pub title: String,
    pub body: String,
    pub choice_a: String,
    pub choice_b: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Choice {
    A,
    B,
}
