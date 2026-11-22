//! 塾の年度カレンダー。年度は 2 月 1 日に始まり、1 年 = 52 週。

use serde::{Deserialize, Serialize};

pub const WEEKS_PER_YEAR: u32 = 52;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    /// 日付の前後を比べるための通し番号（日数ではない）。
    pub fn ordinal(&self) -> i64 {
        self.year as i64 * 512 + self.month as i64 * 32 + self.day as i64
    }

    pub fn label(&self) -> String {
        format!("{}年{}月{}日", self.year, self.month, self.day)
    }

    pub fn short_label(&self) -> String {
        format!("{}/{}", self.month, self.day)
    }
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        _ => 28,
    }
}

/// 年度内の位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calendar {
    /// 年度（2 月始まり）。
    pub school_year: i32,
    /// 年度内の週（0..52）。
    pub week: u32,
}

impl Calendar {
    pub fn new(school_year: i32) -> Self {
        Calendar { school_year, week: 0 }
    }

    pub fn date(&self) -> Date {
        let mut y = self.school_year;
        let mut m = 2;
        let mut d = 1 + self.week * 7;
        loop {
            let dim = days_in_month(y, m);
            if d <= dim {
                break;
            }
            d -= dim;
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
        }
        Date { year: y, month: m, day: d }
    }

    /// 次の週へ。年度が変わったら true。
    pub fn advance(&mut self) -> bool {
        self.week += 1;
        if self.week >= WEEKS_PER_YEAR {
            self.week = 0;
            self.school_year += 1;
            true
        } else {
            false
        }
    }

    pub fn season(&self) -> Season {
        let d = self.date();
        match (d.month, d.day) {
            (2, _) | (3, _) => Season::NewTerm,
            (4..=6, _) => Season::Spring,
            (7, day) if day >= 20 => Season::Summer,
            (7, _) => Season::Spring,
            (8, _) => Season::Summer,
            (9..=11, _) => Season::Autumn,
            _ => Season::ExamRun,
        }
    }

    pub fn is_summer_course(&self) -> bool {
        self.season() == Season::Summer
    }

    /// 6 年生の志望校別特訓（9 月〜1 月）。
    pub fn is_special_training(&self) -> bool {
        matches!(self.season(), Season::Autumn | Season::ExamRun)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Season {
    /// 2〜3 月：新学年・入塾ピーク
    NewTerm,
    /// 4〜7 月中旬
    Spring,
    /// 夏期講習
    Summer,
    /// 9〜11 月
    Autumn,
    /// 12〜1 月：直前期
    ExamRun,
}

impl Season {
    pub fn label(self) -> &'static str {
        match self {
            Season::NewTerm => "新学年",
            Season::Spring => "春〜初夏",
            Season::Summer => "夏期講習",
            Season::Autumn => "秋・志望校別特訓",
            Season::ExamRun => "直前期",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_week_is_feb_1() {
        let c = Calendar::new(2027);
        assert_eq!(c.date(), Date { year: 2027, month: 2, day: 1 });
    }

    #[test]
    fn last_week_is_january() {
        let c = Calendar { school_year: 2027, week: 51 };
        let d = c.date();
        assert_eq!((d.year, d.month), (2028, 1));
    }

    #[test]
    fn summer_exists() {
        let weeks: Vec<u32> = (0..52).filter(|w| Calendar { school_year: 2027, week: *w }.is_summer_course()).collect();
        assert!(weeks.len() >= 5 && weeks.len() <= 7, "{weeks:?}");
    }

    #[test]
    fn advance_rolls_year() {
        let mut c = Calendar { school_year: 2027, week: 51 };
        assert!(c.advance());
        assert_eq!(c, Calendar { school_year: 2028, week: 0 });
    }
}
