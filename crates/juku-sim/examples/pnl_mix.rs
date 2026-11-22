//! 成熟した教室の損益の内訳（売上に対する費用の割合）。
//!   cargo run -p juku-sim --example pnl_mix --release
use juku_sim::bot::{act_week, Strategy};
use juku_sim::*;
fn main() {
    let mut w = World::new(1000);
    for week in 0..(8 * 52) {
        act_week(&mut w, Strategy::Growth, week);
        w.step_week();
    }
    for c in &w.classrooms {
        let n = 13;
        let mut sum = Pnl::default();
        for r in c.history.iter().rev().take(n) {
            let p = &r.pnl;
            sum.tuition += p.tuition;
            sum.extra_courses += p.extra_courses;
            sum.admission += p.admission;
            sum.materials_revenue += p.materials_revenue;
            sum.teacher_cost += p.teacher_cost;
            sum.manager_cost += p.manager_cost;
            sum.rent += p.rent;
            sum.property_upkeep += p.property_upkeep;
            sum.materials_cost += p.materials_cost;
            sum.marketing += p.marketing;
            sum.depreciation += p.depreciation;
            sum.other += p.other;
            sum.tax += p.tax;
        }
        let rev = sum.revenue() as f64;
        let pct = |v: i64| format!("{:>3.0}%", v as f64 / rev * 100.0);
        println!(
            "{:<8} 生徒{:>4} 講師{:>3} 売上/週{:>9} | 講師{} 教室長{} 家賃{} 教材{} 広告{} 他{} | 営業利益{} 純利益{}",
            c.name,
            c.students.len(),
            c.teachers.len(),
            sum.revenue() / n as i64,
            pct(sum.teacher_cost),
            pct(sum.manager_cost),
            pct(sum.rent + sum.property_upkeep),
            pct(sum.materials_cost),
            pct(sum.marketing),
            pct(sum.other + sum.depreciation),
            pct(sum.operating_profit()),
            pct(sum.net_profit())
        );
        let fees: Vec<i64> = c.courses.iter().map(|k| k.fee).collect();
        println!(
            "         月謝 {:?} | 授業料{} 講習{} 入塾金{} 模試教材{}",
            fees,
            pct(sum.tuition),
            pct(sum.extra_courses),
            pct(sum.admission),
            pct(sum.materials_revenue)
        );
    }
}

#[allow(dead_code)]
fn unused() {}
