//! 新しい教室が何年で黒字化するか（エリア別）。
//!   cargo run -p juku-sim --example new_classroom --release
use juku_sim::*;

fn main() {
    for area in 1..5 {
        let mut nets = [0i64; 3];
        let mut students = [0u32; 3];
        let seeds = 8;
        for seed in 0..seeds {
            let mut w = World::new(2000 + seed);
            w.cash = 30_000_000;
            let ci = w.open_classroom(area, 0).unwrap();
            for week in 0..156 {
                for c in 0..w.classrooms.len() {
                    if w.stats(c).coverage < 1.05 && !w.classrooms[c].candidates.is_empty() {
                        w.classroom_mut(c).hire_candidate(0);
                    }
                }
                if w.pending_event.is_some() {
                    w.resolve_event(Choice::B);
                }
                w.step_week();
                let r = w.classrooms[ci].last_report().unwrap();
                nets[(week / 52) as usize] += r.pnl.net_profit();
                if week % 52 == 51 {
                    students[(week / 52) as usize] += r.total_students();
                }
            }
        }
        println!(
            "{}: 年間純利益 {:?} 万円 / 年度末の生徒 {:?}",
            w_name(area),
            nets.iter().map(|n| n / seeds as i64 / 10_000).collect::<Vec<_>>(),
            students.iter().map(|s| s / seeds as u32).collect::<Vec<_>>()
        );
    }
}

fn w_name(area: usize) -> &'static str {
    juku_sim::data::AREAS[area].name
}
