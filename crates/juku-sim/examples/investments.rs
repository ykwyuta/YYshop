//! 投資の効果：スター講師・自社教材・物件を、標準プレイに足したときの 3 年間の差。
//!   cargo run -p juku-sim --example investments --release
use juku_sim::bot::{play_with, Strategy};
use juku_sim::materials::*;
use juku_sim::*;

fn run(label: &str, setup: impl Fn(&mut World) + Copy) {
    let seeds = 10;
    let mut profit = 0.0;
    let mut val = 0.0;
    let mut fc = 0.0;
    let mut students = 0.0;
    for s in 0..seeds {
        let r = play_with(4000 + s, Strategy::Standard, 3, setup);
        profit += r.years.iter().map(|y| y.net_profit as f64).sum::<f64>();
        val += r.last().valuation as f64 + r.last().debt as f64 * 0.0;
        fc += r.years.iter().map(|y| y.first_choice_rate).sum::<f64>() / 3.0;
        students += r.last().students as f64;
    }
    let n = seeds as f64;
    println!(
        "{label:<24} 3年累計利益 {:>6.0}万 / 3年後企業価値 {:>6.0}万 / 第一志望 {:>3.0}% / 生徒 {:.0}",
        profit / n / 1e4,
        val / n / 1e4,
        fc / n * 100.0,
        students / n
    );
}

fn main() {
    run("標準", |_| {});
    run("スター講師（好条件・確定）", |w| {
        w.cash += 20_000_000; // 比較のため資金を足す（下の「資金のみ」と比べる）
        let (comp, star) = w.star_targets()[0];
        for _ in 0..50 {
            w.competitors[comp].offer_cooldown = 0;
            if w.offer_star(comp, star, 0, OfferLevel::Good).unwrap_or(false) {
                break;
            }
        }
    });
    run("資金のみ +2000万", |w| {
        w.cash += 20_000_000;
    });
    run("自社教材（全軸 0.75）", |w| {
        w.cash += 20_000_000;
        w.launch_own_materials().unwrap();
        w.materials.axes = [0.75; AXIS_COUNT];
        w.cash -= 4_000_000; // 改訂費用の目安
    });
    run("自社教材（立ち上げのみ）", |w| {
        w.cash += 20_000_000;
        w.launch_own_materials().unwrap();
    });
    run("物件購入（資金を足して）", |w| {
        w.cash += 95_000_000;
        w.buy_property(0).unwrap();
    });
    run("資金のみ +9500万", |w| {
        w.cash += 95_000_000;
    });
}
