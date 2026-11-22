use juku_sim::materials::{AXIS_CLARITY, AXIS_TREND};
use juku_sim::*;

/// イベントは常に B（長期の信頼）を選んで進める。
fn run(world: &mut World, weeks: u32) {
    for _ in 0..weeks {
        if world.pending_event.is_some() {
            world.resolve_event(Choice::B);
        }
        assert!(world.step_week());
    }
}

/// 講師が足りなくならないよう、充足率が下がったら採用する（人間のプレイヤーの代わり）。
fn run_managed(world: &mut World, weeks: u32) {
    for _ in 0..weeks {
        for ci in 0..world.classrooms.len() {
            if world.stats(ci).coverage < 1.05 && !world.classrooms[ci].candidates.is_empty() {
                world.classroom_mut(ci).hire_candidate(0);
            }
        }
        if world.pending_event.is_some() {
            world.resolve_event(Choice::B);
        }
        world.step_week();
    }
}

#[test]
fn same_seed_same_result() {
    let mut a = World::new(123);
    let mut b = World::new(123);
    run(&mut a, 104);
    run(&mut b, 104);
    assert_eq!(a.cash, b.cash);
    assert_eq!(a.total_students(), b.total_students());
    assert_eq!(a.exam_results.len(), 2);
    assert_eq!(a.exam_results[1].total_passes(), b.exam_results[1].total_passes());
    assert_eq!(a.rival_log.len(), b.rival_log.len());
}

#[test]
fn one_year_produces_exam_results_and_promotions() {
    let mut w = World::new(1);
    run(&mut w, 52);
    assert_eq!(w.calendar.school_year, 2028);
    assert_eq!(w.calendar.week, 0);
    assert_eq!(w.exam_results.len(), 1);
    let r = &w.exam_results[0];
    assert!(r.candidates > 0);
    assert!(r.total_passes() > 0);
    assert!(w.unseen_exam_result);
    assert!(w.classrooms[0].students_in_grade(6) > 0);
    assert_eq!(w.history.len(), 52);
    assert_eq!(w.classrooms[0].history.len(), 52);
}

#[test]
fn default_play_is_not_bankrupt() {
    let mut w = World::new(99);
    run(&mut w, 52);
    assert!(w.cash > 0, "cash = {}", w.cash);
    assert!(w.total_students() > 30, "students = {}", w.total_students());
}

#[test]
fn heavy_homework_raises_stress() {
    let mut light = World::new(5);
    let mut heavy = World::new(5);
    light.classroom_mut(0).set_homework(1);
    heavy.classroom_mut(0).set_homework(5);
    run(&mut light, 20);
    run(&mut heavy, 20);
    let avg = |w: &World| {
        let s = &w.classrooms[0].students;
        s.iter().map(|s| s.stress).sum::<f64>() / s.len() as f64
    };
    assert!(avg(&heavy) > avg(&light) + 0.1);
}

#[test]
fn raising_price_hurts_price_rating() {
    let mut cheap = World::new(8);
    let mut pricey = World::new(8);
    for i in 0..3 {
        let fee = pricey.classrooms[0].courses[i].fee;
        pricey.classroom_mut(0).set_course_fee(i, fee * 2);
    }
    run(&mut cheap, 8);
    run(&mut pricey, 8);
    assert!(pricey.classrooms[0].ratings.price < cheap.classrooms[0].ratings.price);
}

#[test]
fn pending_event_blocks_step() {
    let mut w = World::new(3);
    w.pending_event = Some(GameEvent {
        kind: EventKind::HomeworkRequest,
        classroom: Some(0),
        title: String::new(),
        body: String::new(),
        choice_a: String::new(),
        choice_b: String::new(),
    });
    assert!(!w.step_week());
    w.resolve_event(Choice::A);
    assert_eq!(w.classrooms[0].homework, 4);
    assert!(w.step_week());
}

#[test]
fn save_roundtrip() {
    let mut w = World::new(11);
    run(&mut w, 10);
    let bytes = postcard::to_allocvec(&w).unwrap();
    let mut back: World = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(back.cash, w.cash);
    run(&mut back, 5);
    run(&mut w, 5);
    assert_eq!(back.cash, w.cash);
}

#[test]
fn delegation_runs_without_player() {
    let mut w = World::new(21);
    for d in Delegation::ALL {
        w.classroom_mut(0).set_delegation(d, true);
    }
    for _ in 0..156 {
        // 会社全体の判断（実績の上積み）だけはプレイヤーが答える
        if w.pending_event.is_some() {
            assert!(w.pending_event.as_ref().unwrap().classroom.is_none(), "教室のイベントは教室長が処理する");
            w.resolve_event(Choice::B);
        }
        w.step_week();
    }
    assert_eq!(w.exam_results.len(), 3);
}

// ---------------------------------------------------------------------------
// 複数教室
// ---------------------------------------------------------------------------

#[test]
fn open_second_classroom() {
    let mut w = World::new(31);
    w.cash = 30_000_000;
    let areas = w.open_areas();
    assert_eq!(areas.len(), w.areas.len() - 1);
    let area = areas[0];
    let cost = w.areas[area].open_cost;
    let cash = w.cash;
    let ci = w.open_classroom(area, 0).expect("open");
    assert_eq!(w.classrooms.len(), 2);
    assert_eq!(w.cash, cash - cost);
    assert!(!w.open_areas().contains(&area));
    assert!(w.open_classroom(area, 0).is_err(), "同じエリアには 2 つ目を開けない");

    run_managed(&mut w, 26);
    assert!(w.classrooms[ci].students.len() > 5, "新教室にも生徒が入る: {}", w.classrooms[ci].students.len());
    let last = w.last_week().unwrap();
    assert_eq!(last.classrooms.len(), 2);
    assert!(last.hq.overhead > 0, "2 教室目から本部費がかかる");
    // 開校費は開校した週のレポートに載る
    assert_eq!(w.history[0].hq.opening, cost);
}

#[test]
fn cannot_open_without_cash() {
    let mut w = World::new(32);
    w.cash = 1_000;
    let area = w.open_areas()[0];
    assert!(w.open_classroom(area, 0).is_err());
    assert_eq!(w.classrooms.len(), 1);
}

#[test]
fn exams_aggregate_all_classrooms() {
    let mut w = World::new(33);
    w.cash = 50_000_000;
    let area = w.open_areas()[0];
    w.open_classroom(area, 0).unwrap();
    run_managed(&mut w, 52);
    let r = &w.exam_results[0];
    let names: std::collections::HashSet<&str> = r.entries.iter().map(|e| e.classroom.as_str()).collect();
    assert!(names.len() >= 2, "両方の教室の 6 年生が受験する: {names:?}");
}

// ---------------------------------------------------------------------------
// 自社教材
// ---------------------------------------------------------------------------

#[test]
fn own_materials_launch_and_develop() {
    let mut w = World::new(41);
    assert!(!w.start_material_project(AXIS_CLARITY, 1), "市販教材は開発できない");
    let cash = w.cash;
    w.launch_own_materials().unwrap();
    assert!(w.launch_own_materials().is_err());
    assert_eq!(w.cash, cash - materials::LAUNCH_COST);
    let before = w.materials.axes[AXIS_CLARITY];
    assert!(w.start_material_project(AXIS_CLARITY, 1));
    run(&mut w, materials::PROJECT_WEEKS);
    assert!(w.materials.axes[AXIS_CLARITY] > before);
    let dev: Yen = w.history.iter().map(|h| h.hq.development).sum();
    assert!(dev > 0);
    assert_eq!(w.history[0].hq.launch, materials::LAUNCH_COST);
}

#[test]
fn strong_materials_improve_exam_results() {
    let mut passes = [0u32; 2];
    for seed in 0..6 {
        for (k, strong) in [false, true].iter().enumerate() {
            let mut w = World::new(500 + seed);
            if *strong {
                w.launch_own_materials().unwrap();
                w.materials.axes = [0.95, 0.6, 0.95, 0.8, 0.95];
            }
            run_managed(&mut w, 51);
            // 年度替わりで入試対応が古くなる前に入試を迎える
            if *strong {
                assert!(w.materials.axes[AXIS_TREND] > 0.9);
            }
            run_managed(&mut w, 1);
            passes[k] += w.exam_results[0].first_choice_passes;
        }
    }
    assert!(passes[1] > passes[0], "強い教材のほうが第一志望の合格が多い: {passes:?}");
}

// ---------------------------------------------------------------------------
// 競合
// ---------------------------------------------------------------------------

#[test]
fn competitors_act_over_time() {
    let mut w = World::new(51);
    run_managed(&mut w, 104);
    assert!(!w.rival_log.is_empty(), "2 年のうちに競合が何か動く");
    let fees_changed = w.competitors.iter().any(|c| c.branches.iter().any(|b| b.fees != c.base_fees));
    assert!(fees_changed || w.rival_log.iter().any(|m| m.kind != RivalMoveKind::PriceCut));
}

#[test]
fn rival_opens_where_player_dominates() {
    // 誰もいない緑町に出店して独占すると、いずれ競合が乗り込んでくる
    let mut opened = 0;
    for seed in 0..4 {
        let mut w = World::new(600 + seed);
        w.cash = 50_000_000;
        let midori = w.areas.iter().position(|a| a.name == "緑町").unwrap();
        w.open_classroom(midori, 0).unwrap();
        run_managed(&mut w, 104);
        if w.rival_log.iter().any(|m| m.kind == RivalMoveKind::Open && m.area == midori) {
            opened += 1;
        }
    }
    assert!(opened >= 1, "どこかのシードで競合が出店する");
}

// ---------------------------------------------------------------------------
// 銀行融資
// ---------------------------------------------------------------------------

#[test]
fn borrow_and_repay() {
    let mut w = World::new(71);
    let limit = w.credit_limit();
    assert!(limit >= 5_000_000, "最初から少しは借りられる: {limit}");
    assert!(w.borrow(limit + 1_000_000, 5).is_err(), "枠を超えては借りられない");
    let cash = w.cash;
    w.borrow(5_000_000, 5).unwrap();
    assert_eq!(w.cash, cash + 5_000_000);
    assert_eq!(w.debt(), 5_000_000);
    assert!(w.credit_limit() < limit, "借りると枠が減る");
    run_managed(&mut w, 10);
    assert!(w.debt() < 5_000_000, "毎週返済される");
    let interest: Yen = w.history.iter().map(|h| h.hq.interest).sum();
    assert!(interest > 0);
    assert_eq!(w.history[0].flows.loan_in, 5_000_000);
    let id = w.loans[0].id;
    w.repay_loan(id).unwrap();
    assert_eq!(w.debt(), 0);
}

#[test]
fn loan_is_repaid_over_term() {
    let mut w = World::new(72);
    w.borrow(2_000_000, 1).unwrap();
    run_managed(&mut w, 52);
    assert!(w.loans.is_empty(), "1 年ローンは 52 週で完済");
}

// ---------------------------------------------------------------------------
// 物件
// ---------------------------------------------------------------------------

#[test]
fn buying_property_replaces_rent_with_upkeep() {
    let mut w = World::new(81);
    let price = w.property_price(0);
    assert!(w.buy_property(0).is_err(), "最初の資金では買えない");
    w.cash = price + 10_000_000;
    w.buy_property(0).unwrap();
    assert!(w.classrooms[0].owned);
    assert_eq!(w.property_assets(), price);
    run_managed(&mut w, 2);
    let pnl = &w.classrooms[0].last_report().unwrap().pnl;
    assert_eq!(pnl.rent, 0);
    assert!(pnl.property_upkeep > 0 && pnl.property_upkeep < w.areas[0].rent, "維持費は家賃より安い");
    assert_eq!(w.history[0].flows.property_purchase, price);
    let proceeds = w.sell_property(0).unwrap();
    assert!(proceeds < price && proceeds > price * 9 / 10);
    assert!(!w.classrooms[0].owned);
}

#[test]
fn property_appreciates_yearly() {
    let mut w = World::new(82);
    w.cash = 200_000_000;
    w.buy_property(0).unwrap();
    let v = w.classrooms[0].property_value;
    run_managed(&mut w, 52);
    assert!(w.classrooms[0].property_value > v);
}

// ---------------------------------------------------------------------------
// 買収
// ---------------------------------------------------------------------------

#[test]
fn acquire_independent_into_new_classroom() {
    let mut w = World::new(91);
    let targets = w.acquisition_targets();
    assert!(!targets.is_empty(), "最初から売りに出ている塾がある");
    let t = targets[0];
    let area = w.competitors[t].branches[0].area;
    assert!(!w.classrooms.iter().any(|c| c.area == area));
    let price = w.acquisition_price(t);
    w.cash = price + 1_000_000;
    let before = w.total_students();
    let summary = w.acquire(t).unwrap();
    assert!(!summary.is_empty());
    assert_eq!(w.classrooms.len(), 2);
    let room = w.classrooms.iter().find(|c| c.area == area).unwrap();
    assert!(!room.students.is_empty() && !room.teachers.is_empty());
    assert!(w.total_students() > before);
    assert!(w.competitors[t].acquired);
    assert!(w.acquisition_targets().iter().all(|&x| x != t));
    assert!(w.acquire(t).is_err(), "同じ塾は 2 度買えない");
    run_managed(&mut w, 8);
    assert!(w.competitors[t].branches.is_empty(), "買収した塾は市場から消える");
}

#[test]
fn acquire_merges_into_existing_classroom() {
    let mut w = World::new(92);
    // けやき台の個人塾を売りに出させて買う
    let t = w.competitors.iter().position(|c| c.name == "小林算数教室").unwrap();
    w.competitors[t].for_sale = true;
    w.cash = 50_000_000;
    let before = w.classrooms[0].students.len();
    w.acquire(t).unwrap();
    assert_eq!(w.classrooms.len(), 1, "同じエリアなので統合");
    assert!(w.classrooms[0].students.len() > before);
}

#[test]
fn majors_cannot_be_acquired() {
    let mut w = World::new(93);
    w.cash = 500_000_000;
    w.competitors[0].for_sale = true;
    assert!(w.acquire(0).is_err());
}

#[test]
fn small_schools_come_up_for_sale_over_time() {
    let mut w = World::new(94);
    run_managed(&mut w, 156);
    let listed = w.rival_log.iter().any(|m| m.kind == RivalMoveKind::ForSale) || w.acquisition_targets().len() > 1;
    assert!(listed, "3 年のうちに新たに売りに出る塾がある");
}

// ---------------------------------------------------------------------------
// スター講師
// ---------------------------------------------------------------------------

#[test]
fn star_poaching_success_and_cooldown() {
    let mut successes = 0;
    for seed in 0..10 {
        let mut w = World::new(1000 + seed);
        w.cash = 100_000_000;
        let (comp, star) = w.star_targets()[0];
        let quality = w.competitors[comp].quality;
        let teachers = w.classrooms[0].teachers.len();
        let chance = w.star_offer_chance(comp, OfferLevel::Generous);
        assert!(chance > w.star_offer_chance(comp, OfferLevel::Standard));
        match w.offer_star(comp, star, 0, OfferLevel::Generous).unwrap() {
            true => {
                successes += 1;
                assert_eq!(w.classrooms[0].teachers.len(), teachers + 1);
                assert!(w.classrooms[0].teachers.iter().any(|t| t.star));
                assert!(w.competitors[comp].quality < quality);
                assert!(w.competitors[comp].grudge > 0, "引き抜かれた側は恨む");
            }
            false => assert_eq!(w.classrooms[0].teachers.len(), teachers),
        }
        assert!(w.offer_star(comp, 0, 0, OfferLevel::Generous).is_err(), "交渉後はしばらく声をかけられない");
    }
    assert!(successes >= 3, "破格の条件ならそれなりに成功する: {successes}/10");
}

#[test]
fn star_teacher_boosts_attractiveness() {
    let mut w = World::new(1100);
    let stats = w.stats(0);
    let before = w.classrooms[0].attractiveness(0, &stats, w.brand, w.trust);
    let mut star = w.classrooms[0].teachers[0].clone();
    star.star = true;
    star.id = 9999;
    w.classrooms[0].teachers.push(star);
    let stats = w.stats(0);
    let after = w.classrooms[0].attractiveness(0, &stats, w.brand, w.trust);
    assert!(after > before);
}

#[test]
fn bankruptcy_after_long_negative_cash() {
    let mut w = World::new(1200);
    w.cash = -1_000_000;
    // 赤字を出し続けるよう、全コースを休講にする
    for i in 0..w.classrooms[0].courses.len() {
        w.classroom_mut(0).set_course_enabled(i, false);
    }
    let mut weeks = 0;
    while !w.bankrupt && weeks < 60 {
        if w.pending_event.is_some() {
            w.resolve_event(Choice::B);
        }
        w.step_week();
        weeks += 1;
    }
    assert!(w.bankrupt, "資金がマイナスのままなら倒産する");
    assert!(weeks <= juku_sim::world::BANKRUPTCY_WEEKS);
    assert!(!w.step_week(), "倒産後は進めない");
    let interest: Yen = w.history.iter().map(|h| h.hq.interest).sum();
    assert!(interest > 0, "当座貸越の利息がかかる");
}

#[test]
fn retention_raise_is_capped_and_grants_loyalty() {
    let mut w = World::new(1201);
    let id = w.classrooms[0].teachers[0].id;
    for _ in 0..30 {
        w.pending_event = Some(GameEvent {
            kind: EventKind::Poaching { teacher_id: id, competitor: 0 },
            classroom: Some(0),
            title: String::new(),
            body: String::new(),
            choice_a: String::new(),
            choice_b: String::new(),
        });
        w.resolve_event(Choice::A);
    }
    let t = w.classrooms[0].teachers.iter().find(|t| t.id == id).unwrap();
    assert!(t.pay <= MAX_PRO_PAY, "昇給を重ねても上限を超えない: {}", t.pay);
    assert!(t.loyal_weeks > 0, "引き止めた講師はしばらく引き抜かれない");
}
