//! 秘書の報告：状況を作って、具体的な内容が報告されるかを確かめる。
use juku_sim::advisor::{advise, Priority, Topic};
use juku_sim::*;

fn step(w: &mut World, weeks: u32) {
    for _ in 0..weeks {
        if w.pending_event.is_some() {
            w.resolve_event(Choice::B);
        }
        w.step_week();
    }
}

#[test]
fn quiet_start_has_no_urgent_reports() {
    let w = World::new(1);
    assert!(advise(&w).iter().all(|a| a.priority != Priority::Urgent));
}

#[test]
fn teacher_shortage_names_classroom_and_candidate() {
    let mut w = World::new(2);
    step(&mut w, 2);
    // 講師を 1 人だけ残す
    let keep = w.classrooms[0].teachers[0].id;
    w.classrooms[0].teachers.retain(|t| t.id == keep);
    let reports = advise(&w);
    let a = reports.iter().find(|a| a.title.contains("講師が足りません")).expect("講師不足が報告される");
    assert_eq!(a.priority, Priority::Urgent);
    assert_eq!(a.topic, Topic::Classroom);
    assert_eq!(a.classroom, Some(0));
    assert!(a.body.contains("吉祥寺校"));
    assert!(a.body.contains("充足率"));
    let candidate = &w.classrooms[0].candidates.iter().max_by_key(|t| t.skill).unwrap().name;
    assert!(a.body.contains(candidate.as_str()), "採用候補の名前を挙げる: {}", a.body);
    assert_eq!(reports[0].priority, Priority::Urgent, "至急のものが先頭に来る");
}

#[test]
fn negative_cash_warns_about_bankruptcy() {
    let mut w = World::new(3);
    step(&mut w, 4);
    w.cash = -2_000_000;
    let a = advise(&w).into_iter().find(|a| a.title.contains("資金がマイナス")).expect("資金ショートが報告される");
    assert_eq!(a.priority, Priority::Urgent);
    assert!(a.body.contains("倒産"));
    assert!(a.body.contains("融資枠"));
}

#[test]
fn rival_price_cut_in_our_area_is_reported_with_prices() {
    let mut w = World::new(4);
    step(&mut w, 3);
    // 吉祥寺でスタディリンクが値下げした、という動きを作る
    let comp = w.competitors.iter().position(|c| c.name == "スタディリンク").unwrap();
    let date = w.calendar.date();
    w.rival_log.insert(
        0,
        RivalMove {
            date,
            competitor: comp,
            area: 0,
            kind: RivalMoveKind::PriceCut,
            text: "スタディリンクが吉祥寺で月謝を 1 割値下げしました".into(),
        },
    );
    let a = advise(&w).into_iter().find(|a| a.title.contains("値下げ")).expect("値下げが報告される");
    assert!(a.title.contains("スタディリンク") && a.title.contains("吉祥寺"));
    assert!(a.body.contains("吉祥寺校"), "自塾の月謝と比べる: {}", a.body);
    assert!(a.body.contains("シェア"));
    assert_eq!(a.topic, Topic::Marketing);
}

#[test]
fn school_for_sale_is_suggested_with_price() {
    let w = World::new(5);
    let target = w.acquisition_targets()[0];
    let name = w.competitors[target].name.clone();
    let a = advise(&w).into_iter().find(|a| a.title.contains(&name)).expect("売りに出ている塾を知らせる");
    assert_eq!(a.priority, Priority::Opportunity);
    assert!(a.body.contains(&format_yen(w.acquisition_price(target))));
}

#[test]
fn churn_report_explains_causes() {
    let mut w = World::new(6);
    step(&mut w, 2);
    w.classroom_mut(0).set_homework(5);
    for s in w.classrooms[0].students.iter_mut().take(10) {
        s.stress = 0.9;
    }
    let a = advise(&w).into_iter().find(|a| a.title.contains("退塾")).expect("退塾の兆しを知らせる");
    assert!(a.body.contains("ストレス"));
    assert!(a.body.contains("宿題"));
    assert_eq!(a.topic, Topic::Courses, "宿題が多いならコース画面へ案内する");
}

#[test]
fn every_report_is_concrete() {
    // どの報告にも、具体的な数字か名前が入っている
    let mut w = World::new(7);
    for week in 0..(3 * 52) {
        juku_sim::bot::act_week(&mut w, juku_sim::bot::Strategy::Growth, week);
        w.step_week();
        for a in advise(&w) {
            assert!(!a.title.is_empty() && !a.body.is_empty());
            let has_number = a.body.chars().any(|c| c.is_ascii_digit());
            assert!(has_number, "数字の入っていない報告: {} / {}", a.title, a.body);
        }
    }
}
