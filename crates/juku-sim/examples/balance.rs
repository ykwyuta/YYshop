//! ヘッドレスでバランスを確認する。
//!   cargo run -p juku-sim --example balance --release -- [年数] [シード数]
//!
//! 「放置」（何もしない）と「標準」（講師が足りなければ採用する）の 2 通りで回す。
use juku_sim::*;

fn play(seed: u64, years: u32, managed: bool) -> World {
    let mut w = World::new(seed);
    for _ in 0..years * 52 {
        if managed {
            for ci in 0..w.classrooms.len() {
                if w.stats(ci).coverage < 1.05 && !w.classrooms[ci].candidates.is_empty() {
                    w.classroom_mut(ci).hire_candidate(0);
                }
            }
        }
        if w.pending_event.is_some() {
            w.resolve_event(Choice::B);
        }
        w.step_week();
    }
    w
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let years: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);
    let seeds: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
    for managed in [false, true] {
        println!("== {} ==", if managed { "標準（講師を補充）" } else { "放置" });
        for seed in 0..seeds {
            let w = play(seed, years, managed);
            let c = &w.classrooms[0];
            let r = c.last_report().unwrap();
            let annual: Yen = w.history.iter().rev().take(52).map(|h| h.net()).sum();
            println!(
                "seed {seed}: 現金 {} / 年間純利益 {} / 生徒 {:?} / シェア {:.0}% / 評価 {:.2} / ブランド {:.2} / 講師 {} / 競合の動き {}",
                format_yen(w.cash),
                format_yen(annual),
                r.students,
                r.market_share * 100.0,
                r.ratings.overall(),
                w.brand,
                c.teachers.len(),
                w.rival_log.len(),
            );
            for e in &w.exam_results {
                println!(
                    "    {}年度: 受験 {} 人 / 合格 {} 件 / 第一志望 {:.0}% / ブランド {:.2}→{:.2}",
                    e.school_year,
                    e.candidates,
                    e.total_passes(),
                    e.first_choice_rate() * 100.0,
                    e.brand_before,
                    e.brand_after
                );
            }
        }
    }
}
