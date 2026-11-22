//! 生徒・講師・教室長の生成。

use crate::calendar::Date;
use crate::data;
use crate::model::*;
use crate::rng::Rng;

pub fn alloc_id(next_id: &mut u32) -> u32 {
    let id = *next_id;
    *next_id += 1;
    id
}

pub fn random_name(rng: &mut Rng) -> String {
    let s = *rng.pick(data::SURNAMES);
    let g = *rng.pick(data::GIVEN_NAMES);
    format!("{s} {g}")
}

/// 偏差値 `aim` に最も近い学校。
pub fn school_near(schools: &[School], aim: f64, prefer_public: bool) -> usize {
    let mut best = 0;
    let mut best_d = f64::MAX;
    for (i, s) in schools.iter().enumerate() {
        if prefer_public != (s.tier == SchoolTier::Public) {
            continue;
        }
        let d = (s.deviation - aim).abs();
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

pub fn make_student(rng: &mut Rng, next_id: &mut u32, schools: &[School], grade: u8, scholarship: bool) -> Student {
    let id = alloc_id(next_id);
    let name = random_name(rng);
    let deviation = if scholarship { rng.normal(63.0, 4.0) } else { rng.normal(48.0, 9.0) }.clamp(28.0, 75.0);
    let budget = match rng.below(10) {
        0..=2 => Budget::Low,
        3..=7 => Budget::Mid,
        _ => Budget::High,
    };
    let parent = match rng.below(10) {
        0..=3 => ParentType::Eager,
        4..=6 => ParentType::Relaxed,
        _ => ParentType::Anxious,
    };
    let aim = deviation + rng.range(2.0, 9.0);
    let prefer_public = rng.chance(0.15);
    let target_school = school_near(schools, aim, prefer_public);
    Student {
        id,
        name,
        grade,
        deviation,
        start_deviation: deviation,
        motivation: rng.range(0.45, 0.8),
        stress: rng.range(0.1, 0.35),
        target_school,
        budget,
        parent,
        weeks_enrolled: 0,
        scholarship,
        taking_extra: false,
        history: vec![deviation as f32],
    }
}

pub fn make_teacher(rng: &mut Rng, next_id: &mut u32, kind: TeacherKind) -> Teacher {
    let id = alloc_id(next_id);
    let name = random_name(rng);
    let (skill, care, pay) = match kind {
        TeacherKind::Pro => {
            let skill = rng.normal(66.0, 11.0).clamp(35.0, 98.0);
            let care = rng.normal(55.0, 15.0).clamp(10.0, 98.0);
            let pay = 3_800_000.0 + (skill - 40.0) * 55_000.0 + rng.range(0.0, 300_000.0);
            (skill, care, (pay / 10_000.0) as Yen * 10_000)
        }
        TeacherKind::Student => {
            let skill = rng.normal(50.0, 12.0).clamp(20.0, 90.0);
            let care = rng.normal(55.0, 15.0).clamp(10.0, 98.0);
            let pay = 1_300.0 + (skill - 20.0) * 14.0;
            (skill, care, (pay / 50.0) as Yen * 50)
        }
    };
    Teacher { id, name, kind, skill: skill as u32, care: care as u32, pay, fatigue: 0.1, star: false }
}

pub fn make_candidates(rng: &mut Rng, next_id: &mut u32) -> Vec<Teacher> {
    vec![
        make_teacher(rng, next_id, TeacherKind::Pro),
        make_teacher(rng, next_id, TeacherKind::Student),
        make_teacher(rng, next_id, TeacherKind::Student),
    ]
}

/// 教室長の候補。能力が高いほど年俸も高い。
pub fn make_manager(rng: &mut Rng, hired: Date, used_names: &[String]) -> Manager {
    let mut name: String = (*rng.pick(data::MANAGER_NAMES)).into();
    for _ in 0..10 {
        if !used_names.contains(&name) {
            break;
        }
        name = (*rng.pick(data::MANAGER_NAMES)).into();
    }
    let staffing = 45 + rng.below(45) as u32;
    let teaching = 45 + rng.below(45) as u32;
    let sales = 45 + rng.below(45) as u32;
    let ethics = 35 + rng.below(60) as u32;
    let ability = (staffing + teaching + sales) as i64;
    let salary = (4_000_000 + (ability - 135) * 40_000).max(3_500_000) / 100_000 * 100_000;
    Manager { name, staffing, teaching, sales, ethics, salary, hired, delegations: [false; 4] }
}

/// 競合塾の看板講師。
pub fn make_star(rng: &mut Rng, next_id: &mut u32) -> Teacher {
    let mut t = make_teacher(rng, next_id, TeacherKind::Pro);
    t.skill = 84 + rng.below(14) as u32;
    t.care = 50 + rng.below(45) as u32;
    t.pay = (7_500_000 + (t.skill as Yen - 84) * 250_000 + rng.below(10) as Yen * 100_000) / 100_000 * 100_000;
    t.star = true;
    t
}

/// 買収した塾から引き継ぐ講師。塾の授業力 `quality`（0..1）に近い腕前。
pub fn make_inherited_teacher(rng: &mut Rng, next_id: &mut u32, quality: f64, pro: bool) -> Teacher {
    let kind = if pro { TeacherKind::Pro } else { TeacherKind::Student };
    let mut t = make_teacher(rng, next_id, kind);
    t.skill = (rng.normal(quality * 100.0, 8.0)).clamp(30.0, 95.0) as u32;
    t
}
