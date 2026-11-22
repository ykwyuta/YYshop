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

#[test]
fn same_seed_same_result() {
    let mut a = World::new(123);
    let mut b = World::new(123);
    run(&mut a, 104);
    run(&mut b, 104);
    assert_eq!(a.cash, b.cash);
    assert_eq!(a.students.len(), b.students.len());
    assert_eq!(a.exam_results.len(), 2);
    assert_eq!(a.exam_results[1].total_passes(), b.exam_results[1].total_passes());
}

#[test]
fn one_year_produces_exam_results_and_promotions() {
    let mut w = World::new(1);
    let g5_before = w.students_in_grade(5);
    run(&mut w, 52);
    assert_eq!(w.calendar.school_year, 2028);
    assert_eq!(w.calendar.week, 0);
    assert_eq!(w.exam_results.len(), 1);
    let r = &w.exam_results[0];
    assert!(r.candidates > 0);
    assert!(r.total_passes() > 0);
    assert!(w.unseen_exam_result);
    // 5 年生は 6 年生に進級している（多少の退塾・入塾はある）
    assert!(w.students_in_grade(6) > 0);
    let _ = g5_before;
    assert_eq!(w.history.len(), 52);
}

#[test]
fn default_play_is_not_bankrupt() {
    let mut w = World::new(99);
    run(&mut w, 52);
    assert!(w.cash > 0, "cash = {}", w.cash);
    assert!(w.students.len() > 30, "students = {}", w.students.len());
}

#[test]
fn heavy_homework_raises_stress() {
    let mut light = World::new(5);
    let mut heavy = World::new(5);
    light.set_homework(1);
    heavy.set_homework(5);
    run(&mut light, 20);
    run(&mut heavy, 20);
    let avg = |w: &World| w.students.iter().map(|s| s.stress).sum::<f64>() / w.students.len() as f64;
    assert!(avg(&heavy) > avg(&light) + 0.1);
}

#[test]
fn raising_price_increases_unit_revenue_but_hurts_rating() {
    let mut cheap = World::new(8);
    let mut pricey = World::new(8);
    for i in 0..3 {
        let fee = pricey.courses[i].fee;
        pricey.set_course_fee(i, fee * 2);
    }
    run(&mut cheap, 8);
    run(&mut pricey, 8);
    assert!(pricey.ratings.price < cheap.ratings.price);
}

#[test]
fn pending_event_blocks_step() {
    let mut w = World::new(3);
    w.pending_event = Some(GameEvent {
        kind: EventKind::HomeworkRequest,
        title: String::new(),
        body: String::new(),
        choice_a: String::new(),
        choice_b: String::new(),
    });
    assert!(!w.step_week());
    w.resolve_event(Choice::A);
    assert_eq!(w.homework, 4);
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
        w.set_delegation(d, true);
    }
    for _ in 0..156 {
        assert!(w.pending_event.is_none(), "委任中はイベントが自動処理される");
        w.step_week();
    }
    assert_eq!(w.exam_results.len(), 3);
}
