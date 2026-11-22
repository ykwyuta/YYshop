//! 中学受験塾経営シミュレーションのゲームロジック。
//!
//! このクレートは描画・OS・時刻に一切依存しない。乱数はシードから決定論的に生成するため、
//! Windows と WebAssembly で同じ操作をすれば同じ結果になる。

pub mod calendar;
pub mod classroom;
pub mod data;
mod gen;
pub mod market;
pub mod materials;
pub mod model;
pub mod report;
pub mod rng;
pub mod world;

pub use calendar::{Calendar, Date, Season};
pub use classroom::{Classroom, ClassroomStats};
pub use market::{Area, Branch, Competitor, CompetitorStyle, RivalMove, RivalMoveKind};
pub use materials::{MaterialSource, Materials};
pub use model::*;
pub use report::*;
pub use world::{World, SAVE_VERSION};

/// 金額を「¥1,234,567」形式で表示する。
pub fn format_yen(v: Yen) -> String {
    let neg = v < 0;
    let digits = v.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    if neg {
        format!("-¥{out}")
    } else {
        format!("¥{out}")
    }
}

/// 数値を 3 桁区切りで表示する（符号付き）。
pub fn format_signed(v: i64) -> String {
    let s = format_yen(v.abs()).trim_start_matches('¥').to_string();
    if v > 0 {
        format!("+{s}")
    } else if v < 0 {
        format!("-{s}")
    } else {
        "0".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yen_format() {
        assert_eq!(format_yen(0), "¥0");
        assert_eq!(format_yen(1_234_567), "¥1,234,567");
        assert_eq!(format_yen(-1000), "-¥1,000");
        assert_eq!(format_signed(2500), "+2,500");
        assert_eq!(format_signed(-12), "-12");
    }
}
