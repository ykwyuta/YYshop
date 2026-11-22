//! 表示用の書式。

pub use juku_sim::{format_signed, format_yen};

pub fn pct(v: f64) -> String {
    format!("{:.0}%", v * 100.0)
}

/// 前週比の割合（例："+4%"）。
pub fn pct_change(cur: i64, prev: i64) -> String {
    if prev == 0 {
        return "—".into();
    }
    let p = (cur - prev) as f64 / prev.abs() as f64 * 100.0;
    if p >= 0.0 {
        format!("+{p:.0}%")
    } else {
        format!("{p:.0}%")
    }
}

/// 大きい金額を「万円」単位で短く（グラフの目盛り等）。
pub fn man(v: i64) -> String {
    let m = v as f64 / 10_000.0;
    if m.abs() >= 10_000.0 {
        format!("{:.1}億円", m / 10_000.0)
    } else {
        format!("{m:.0}万円")
    }
}
