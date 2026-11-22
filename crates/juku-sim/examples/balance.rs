//! ヘッドレスでバランスを確認する。戦略ごとに複数シードで回し、年度ごとの平均を表示する。
//!   cargo run -p juku-sim --example balance --release -- [年数] [シード数]
use juku_sim::bot::{average, play, play_with, Strategy};

fn man(v: f64) -> String {
    format!("{:>7.0}万", v / 10_000.0)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let years: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
    let seeds: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    for strategy in Strategy::ALL {
        let runs: Vec<_> = (0..seeds).map(|s| play(1000 + s, strategy, years)).collect();
        println!("== {} ==", strategy.label());
        println!(
            " 年 |   現金   |  借入  | 企業価値 | 年間利益 | 教室 | 生徒 | 講師 | ﾌﾞﾗﾝﾄﾞ | 信頼 | 評価 | 第一志望 | 入塾 | 退塾 | ｼｪｱ | 市場 | 赤字週 | 炎上"
        );
        for y in 0..years as usize {
            println!(
                " {:>2} | {} | {} | {} | {} | {:>4.1} | {:>4.0} | {:>4.1} | {:>6.2} | {:>4.2} | {:>4.2} | {:>7.0}% | {:>4.0} | {:>4.0} | {:>2.0}% | {:>4.0} | {:>6.1} | {:>4.1}",
                y + 1,
                man(average(&runs, y, |s| s.cash as f64)),
                man(average(&runs, y, |s| s.debt as f64)),
                man(average(&runs, y, |s| s.valuation as f64)),
                man(average(&runs, y, |s| s.net_profit as f64)),
                average(&runs, y, |s| s.classrooms as f64),
                average(&runs, y, |s| s.students as f64),
                average(&runs, y, |s| s.teachers as f64),
                average(&runs, y, |s| s.brand),
                average(&runs, y, |s| s.trust),
                average(&runs, y, |s| s.rating),
                average(&runs, y, |s| s.first_choice_rate) * 100.0,
                average(&runs, y, |s| s.new_students as f64),
                average(&runs, y, |s| s.churned as f64),
                average(&runs, y, |s| s.share) * 100.0,
                average(&runs, y, |s| s.market_total as f64),
                average(&runs, y, |s| s.negative_weeks as f64),
                average(&runs, y, |s| s.scandals as f64),
            );
        }
        let bankrupt = runs.iter().filter(|r| r.last().bankrupt).count();
        if bankrupt > 0 {
            println!(" 倒産: {bankrupt}/{seeds}");
        }
        let y = years as usize - 1;
        let listed = runs.iter().filter(|r| r.last().listed).count();
        let first_listed: Vec<usize> =
            runs.iter().filter_map(|r| r.years.iter().position(|s| s.listed).map(|p| p + 1)).collect();
        if listed > 0 || average(&runs, y, |s| s.sales_contracts as f64) > 0.0 {
            println!(
                " 上場 {listed}/{seeds}（上場した年 {first_listed:?}）／外販 契約 {:.0} 塾・年商 {}／塾長の個人資産 {}",
                average(&runs, y, |s| s.sales_contracts as f64),
                man(average(&runs, y, |s| s.sales_revenue as f64)),
                man(average(&runs, y, |s| s.personal_assets as f64)),
            );
        }
    }

    // 価格：標準戦略で月謝を変えたときの 5 年後
    println!("== 月謝のスイープ（標準・{years} 年後の平均）==");
    for pct in [70, 85, 100, 115, 130, 150] {
        let runs: Vec<_> = (0..seeds)
            .map(|s| {
                play_with(1000 + s, Strategy::Standard, years, |w| {
                    let c = w.classroom_mut(0);
                    for i in 0..3 {
                        let fee = c.courses[i].fee;
                        c.set_course_fee(i, fee * pct / 100);
                    }
                })
            })
            .collect();
        let y = years as usize - 1;
        let total: f64 =
            runs.iter().map(|r| r.years.iter().map(|s| s.net_profit as f64 / 1.0).sum::<f64>()).sum::<f64>()
                / runs.len() as f64;
        println!(
            " {pct:>3}% | 累計利益 {} | 企業価値 {} | 生徒 {:>4.0} | 評価 {:.2}",
            man(total),
            man(average(&runs, y, |s| s.valuation as f64)),
            average(&runs, y, |s| s.students as f64),
            average(&runs, y, |s| s.rating),
        );
    }

    // 宿題の量
    println!("== 宿題の量のスイープ（標準・{years} 年後の平均）==");
    for level in 1..=5u8 {
        let runs: Vec<_> = (0..seeds)
            .map(|s| play_with(1000 + s, Strategy::Standard, years, |w| w.classroom_mut(0).set_homework(level)))
            .collect();
        let y = years as usize - 1;
        let total: f64 = runs.iter().map(|r| r.years.iter().map(|s| s.net_profit as f64).sum::<f64>()).sum::<f64>()
            / runs.len() as f64;
        println!(
            " Lv{level} | 累計利益 {} | 生徒 {:>4.0} | 退塾/年 {:>4.0} | 第一志望 {:>3.0}% | ブランド {:.2}",
            man(total),
            average(&runs, y, |s| s.students as f64),
            average(&runs, y, |s| s.churned as f64),
            (0..years as usize).map(|yy| average(&runs, yy, |s| s.first_choice_rate)).sum::<f64>() / years as f64
                * 100.0,
            average(&runs, y, |s| s.brand),
        );
    }
}
