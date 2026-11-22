//! 秘書の報告を見る。自動プレイで進めながら、ところどころで報告を書き出す。
//!   cargo run -p juku-sim --example secretary --release -- [戦略 0-5] [シード]
use juku_sim::advisor::advise;
use juku_sim::bot::{act_week, Strategy};
use juku_sim::World;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let strategy = Strategy::ALL[args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2)];
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let mut w = World::new(seed);
    for week in 0..(4 * 52) {
        act_week(&mut w, strategy, week);
        w.step_week();
        if week % 26 == 20 {
            println!("===== {} ({}) =====", w.calendar.date().label(), strategy.label());
            for a in advise(&w).iter().take(6) {
                println!("[{}] {}\n    {}", a.priority.label(), a.title, a.body);
            }
        }
    }
}
