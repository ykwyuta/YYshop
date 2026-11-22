//! ヘッドレスでバランスを確認する。
//!   cargo run -p juku-sim --example balance --release -- [年数] [シード数]
use juku_sim::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let years: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);
    let seeds: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
    for seed in 0..seeds {
        let mut w = World::new(seed);
        for _ in 0..years * 52 {
            if w.pending_event.is_some() {
                w.resolve_event(Choice::B);
            }
            w.step_week();
        }
        let r = w.last_report().unwrap();
        println!(
            "seed {seed}: 現金 {} / 生徒 {:?} / シェア {:.0}% / 評価 {:.2} / ブランド {:.2} / 講師 {}",
            format_yen(w.cash),
            r.students,
            r.market_share * 100.0,
            r.ratings.overall(),
            w.brand,
            w.teachers.len(),
        );
        for e in &w.exam_results {
            println!(
                "    {}年度: 受験 {} 人 / 合格 {} 件 / 第一志望合格率 {:.0}% / ブランド {:.2}→{:.2}",
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
