//! 会社全体（複数の教室・自社教材・競合塾）の状態と、週送りの処理。

use serde::{Deserialize, Serialize};

use crate::calendar::{Calendar, Season};
use crate::classroom::{clamp01, distribute_area_demand, Classroom, WeekCtx};
use crate::data;
use crate::gen;
use crate::market::{self, Area, Competitor, PlayerPresence, RivalMove};
use crate::materials::{self, MaterialSource, Materials};
use crate::model::*;
use crate::report::*;
use crate::rng::Rng;

pub const SAVE_VERSION: u32 = 3;
/// 2 教室目からかかる本部の運営費（教室ごと・週）。
const HQ_OVERHEAD_PER_EXTRA_CLASSROOM: Yen = 40_000;
const MAX_RIVAL_LOG: usize = 40;

/// ロジスティック関数の代わり（超越関数を使わない）。x=0 で 0.5。
fn soft_step(x: f64) -> f64 {
    0.5 + 0.5 * x / (1.0 + x.abs())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub save_version: u32,
    pub rng: Rng,
    pub calendar: Calendar,
    pub company_name: String,
    pub cash: Yen,
    /// ブランド力（合格実績）0..1。会社全体で共有する
    pub brand: f64,
    /// 隠しパラメータ：信頼 0..1
    pub trust: f64,
    pub materials: Materials,
    pub areas: Vec<Area>,
    pub classrooms: Vec<Classroom>,
    pub competitors: Vec<Competitor>,
    pub schools: Vec<School>,
    pub history: Vec<CompanyWeek>,
    pub exam_results: Vec<ExamResult>,
    pub rival_log: Vec<RivalMove>,
    /// 新しい教室の教室長候補
    pub manager_candidates: Vec<Manager>,
    pub pending_event: Option<GameEvent>,
    /// 新しい入試結果が出て、まだ表示していない
    pub unseen_exam_result: bool,
    pub padding_this_year: bool,
    /// 今週すでに支払った一時費用（開校・教材立ち上げ）。レポートに載せる
    pub(crate) pending_hq: HqCosts,
    /// 今週の損益に載らないお金の動き（物件・買収・借入）
    pub(crate) pending_flows: crate::finance::CapitalFlows,
    /// 銀行からの借入
    pub loans: Vec<crate::finance::Loan>,
    /// 資金がマイナスの週が続いている数
    pub negative_weeks: u32,
    pub(crate) next_id: u32,
    weeks_since_event: u32,
}

impl World {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let calendar = Calendar::new(2027);
        let mut next_id = 1;
        let schools: Vec<School> = data::SCHOOLS
            .iter()
            .map(|s| School { name: s.name.into(), deviation: s.deviation, tier: s.tier })
            .collect();
        let areas: Vec<Area> = data::AREAS
            .iter()
            .map(|a| Area {
                name: a.name.into(),
                address: a.address.into(),
                description: a.description.into(),
                pool: a.pool,
                rent: a.rent,
                open_cost: a.open_cost,
                property_price: a.property_price,
            })
            .collect();
        let mut competitors: Vec<Competitor> = data::COMPETITORS.iter().map(Competitor::from_def).collect();
        for c in competitors.iter_mut() {
            let n = match c.style {
                crate::market::CompetitorStyle::Elite => 2,
                crate::market::CompetitorStyle::Independent => 0,
                _ => 1,
            };
            for _ in 0..n {
                c.stars.push(gen::make_star(&mut rng, &mut next_id));
            }
        }
        // 緑町の個人塾は最初から後継者を探している
        if let Some(c) = competitors.iter_mut().find(|c| c.name == "まなび舎 森田塾") {
            c.for_sale = true;
        }

        let manager = gen::make_manager(&mut rng, calendar.date(), &[]);
        let mut first = Classroom::new(
            gen::alloc_id(&mut next_id),
            format!("{}校", areas[0].name),
            0,
            areas[0].address.clone(),
            calendar.date(),
            manager,
        );
        first.awareness = 0.4;
        first.review_count = 128;
        first.ratings = Ratings { price: 3.4, teaching: 3.3, care: 3.4, safety: 3.2 };
        first.teachers.push(gen::make_teacher(&mut rng, &mut next_id, TeacherKind::Pro));
        for _ in 0..4 {
            first.teachers.push(gen::make_teacher(&mut rng, &mut next_id, TeacherKind::Student));
        }
        for (grade, n) in [(4u8, 22), (5, 24), (6, 20)] {
            for _ in 0..n {
                let mut s = gen::make_student(&mut rng, &mut next_id, &schools, grade, false);
                s.weeks_enrolled = 4 + rng.below(60) as u32;
                first.students.push(s);
            }
        }
        first.candidates = gen::make_candidates(&mut rng, &mut next_id);
        for c in first.courses.iter_mut() {
            c.rating = 3.4;
        }
        for g in 4..=6u8 {
            let n = first.students_in_grade(g);
            if let Some(c) = first.courses.iter_mut().find(|c| c.kind == CourseKind::Main(g)) {
                c.last_enrolled = n;
                c.prev_enrolled = n;
            }
        }

        let mut world = World {
            save_version: SAVE_VERSION,
            rng,
            calendar,
            company_name: "あおぞら進学塾".into(),
            cash: 8_000_000,
            brand: 0.35,
            trust: 0.6,
            materials: Materials::default(),
            areas,
            classrooms: vec![first],
            competitors,
            schools,
            history: Vec::new(),
            exam_results: Vec::new(),
            rival_log: Vec::new(),
            manager_candidates: Vec::new(),
            pending_event: None,
            unseen_exam_result: false,
            padding_this_year: false,
            pending_hq: HqCosts::default(),
            pending_flows: Default::default(),
            loans: Vec::new(),
            negative_weeks: 0,
            next_id,
            weeks_since_event: 0,
        };
        world.refresh_manager_candidates();
        world
    }

    fn refresh_manager_candidates(&mut self) {
        let used: Vec<String> = self.classrooms.iter().map(|c| c.manager.name.clone()).collect();
        let date = self.calendar.date();
        self.manager_candidates = (0..3).map(|_| gen::make_manager(&mut self.rng, date, &used)).collect();
    }

    // ------------------------------------------------------------------
    // 参照
    // ------------------------------------------------------------------

    pub fn classroom(&self, ci: usize) -> &Classroom {
        &self.classrooms[ci]
    }

    pub fn classroom_mut(&mut self, ci: usize) -> &mut Classroom {
        &mut self.classrooms[ci]
    }

    pub fn total_students(&self) -> u32 {
        self.classrooms.iter().map(|c| c.students.len() as u32).sum()
    }

    pub fn last_week(&self) -> Option<&CompanyWeek> {
        self.history.last()
    }

    pub fn stats(&self, ci: usize) -> crate::classroom::ClassroomStats {
        self.classrooms[ci].stats(&self.calendar, &self.materials)
    }

    pub fn course_unit_cost(&self, ci: usize, kind: CourseKind) -> Yen {
        self.classrooms[ci].course_unit_cost(kind, &self.calendar, &self.materials)
    }

    /// 自塾が教室を持っていないエリア（開校できる場所）。
    pub fn open_areas(&self) -> Vec<usize> {
        (0..self.areas.len()).filter(|a| !self.classrooms.iter().any(|c| c.area == *a)).collect()
    }

    /// エリアの在籍シェア：(名前, 在籍数, 色, 自塾か)。
    pub fn area_shares(&self, area: usize) -> Vec<(String, u32, [u8; 3], bool)> {
        let mut v = Vec::new();
        for c in self.classrooms.iter().filter(|c| c.area == area) {
            v.push((c.name.clone(), c.students.len() as u32, [0xE3, 0x9B, 0x3A], true));
        }
        for comp in &self.competitors {
            if let Some(b) = comp.branch(area) {
                v.push((comp.name.clone(), b.total(), comp.color, false));
            }
        }
        v
    }

    /// 企業価値の目安：現金 ＋ 物件 ＋ 直近の利益 × 6 ＋ ブランド − 借入残高。
    pub fn valuation(&self) -> Yen {
        let recent: Vec<&CompanyWeek> = self.history.iter().rev().take(13).collect();
        let annual = if recent.is_empty() {
            0.0
        } else {
            recent.iter().map(|r| r.net() as f64).sum::<f64>() / recent.len() as f64 * 52.0
        };
        self.cash + self.property_assets() + (annual.max(0.0) * 6.0) as Yen + (self.brand * 30_000_000.0) as Yen
            - self.debt()
    }

    // ------------------------------------------------------------------
    // 会社としての操作
    // ------------------------------------------------------------------

    /// 自社教材を立ち上げる。
    pub fn launch_own_materials(&mut self) -> Result<(), String> {
        if self.materials.is_own() {
            return Err("すでに自社教材です".into());
        }
        if self.cash < materials::LAUNCH_COST {
            return Err("資金が足りません".into());
        }
        self.cash -= materials::LAUNCH_COST;
        self.pending_hq.launch += materials::LAUNCH_COST;
        self.materials.source = MaterialSource::Own;
        self.materials.launched = Some(self.calendar.date());
        Ok(())
    }

    pub fn start_material_project(&mut self, axis: usize, direction: i8) -> bool {
        self.materials.start_project(axis, direction)
    }

    pub fn cancel_material_project(&mut self) {
        self.materials.project = None;
    }

    /// 新しい教室を開く。開いた教室の添字を返す。
    pub fn open_classroom(&mut self, area: usize, manager_index: usize) -> Result<usize, String> {
        if !self.open_areas().contains(&area) {
            return Err("このエリアにはすでに教室があります".into());
        }
        let Some(manager) = self.manager_candidates.get(manager_index).cloned() else {
            return Err("教室長を選んでください".into());
        };
        let cost = self.areas[area].open_cost;
        if self.cash < cost {
            return Err("資金が足りません".into());
        }
        self.cash -= cost;
        self.pending_hq.opening += cost;
        self.manager_candidates.remove(manager_index);

        let a = &self.areas[area];
        let mut manager = manager;
        manager.hired = self.calendar.date();
        let mut c = Classroom::new(
            gen::alloc_id(&mut self.next_id),
            format!("{}校", a.name),
            area,
            a.address.clone(),
            self.calendar.date(),
            manager,
        );
        // 最初の講師 2 人と、既存教室の料金を引き継ぐ
        for _ in 0..2 {
            c.teachers.push(gen::make_teacher(&mut self.rng, &mut self.next_id, TeacherKind::Student));
        }
        c.candidates = gen::make_candidates(&mut self.rng, &mut self.next_id);
        if let Some(src) = self.classrooms.first() {
            for (dst, s) in c.courses.iter_mut().zip(src.courses.iter()) {
                dst.fee = s.fee;
            }
        }
        // 開校すると地域でそれなりに話題になる
        c.awareness = 0.25 + 0.2 * self.brand;
        self.classrooms.push(c);
        if self.manager_candidates.is_empty() {
            self.refresh_manager_candidates();
        }
        Ok(self.classrooms.len() - 1)
    }

    pub fn resolve_event(&mut self, choice: Choice) {
        let Some(ev) = self.pending_event.take() else { return };
        self.apply_event_choice(&ev, choice);
    }

    fn apply_event_choice(&mut self, ev: &GameEvent, choice: Choice) {
        let ci = ev.classroom.unwrap_or(0).min(self.classrooms.len() - 1);
        match (ev.kind, choice) {
            (EventKind::HomeworkRequest, Choice::A) => {
                let c = &mut self.classrooms[ci];
                c.homework = (c.homework + 1).min(5);
                self.trust -= 0.02;
            }
            (EventKind::HomeworkRequest, Choice::B) => {
                self.cash -= 60_000;
                self.trust += 0.03;
                for s in self.classrooms[ci].students.iter_mut().filter(|s| s.parent == ParentType::Eager) {
                    s.motivation = clamp01(s.motivation + 0.05);
                }
            }
            (EventKind::StrugglingSenior { student_id }, Choice::A) => {
                let schools = &self.schools;
                if let Some(s) = self.classrooms[ci].students.iter_mut().find(|s| s.id == student_id) {
                    s.target_school = gen::school_near(schools, s.deviation - 3.0, false);
                    s.motivation = clamp01(s.motivation - 0.2);
                }
                self.trust -= 0.04;
            }
            (EventKind::StrugglingSenior { student_id }, Choice::B) => {
                self.cash -= 80_000;
                if let Some(s) = self.classrooms[ci].students.iter_mut().find(|s| s.id == student_id) {
                    s.motivation = clamp01(s.motivation + 0.2);
                    s.deviation += 1.5;
                }
                self.trust += 0.04;
            }
            (EventKind::Poaching { teacher_id, .. }, Choice::A) => {
                if let Some(t) = self.classrooms[ci].teachers.iter_mut().find(|t| t.id == teacher_id) {
                    t.pay = t.pay * 6 / 5;
                    t.fatigue = (t.fatigue - 0.2).max(0.0);
                }
            }
            (EventKind::Poaching { teacher_id, competitor }, Choice::B) => {
                self.classrooms[ci].dismiss_teacher(teacher_id);
                // 引き抜いた競合の授業力が上がる
                if let Some(c) = self.competitors.get_mut(competitor) {
                    c.quality = (c.quality + 0.02).min(0.95);
                }
            }
            (EventKind::ResultPadding, Choice::A) => {
                self.padding_this_year = true;
                self.trust -= 0.15;
            }
            (EventKind::ResultPadding, Choice::B) => {
                self.padding_this_year = false;
                self.trust += 0.05;
            }
        }
        self.trust = clamp01(self.trust);
    }

    // ------------------------------------------------------------------
    // 週送り
    // ------------------------------------------------------------------

    /// 1 週進める。判断待ちのイベントがある場合は進めない（false を返す）。
    pub fn step_week(&mut self) -> bool {
        if self.pending_event.is_some() {
            return false;
        }
        let calendar = self.calendar;
        let season = calendar.season();
        let prev_calendar = self.history.last().map(|r| Calendar { school_year: r.school_year, week: r.week });
        let n = self.classrooms.len();
        let mut news: Vec<Vec<String>> = vec![Vec::new(); n];
        let mut company_news = Vec::new();

        let reports = {
            let World { rng, next_id, schools, materials, classrooms, competitors, areas, brand, trust, cash, .. } =
                self;
            let mut ctx = WeekCtx {
                rng,
                next_id,
                schools,
                materials,
                calendar,
                prev_calendar,
                brand: *brand,
                trust: *trust,
                cash: *cash,
            };

            for (ci, c) in classrooms.iter_mut().enumerate() {
                c.begin_week(&mut ctx, &mut news[ci]);
            }

            // エリアごとに入塾希望者を分け合う
            let mut admitted = vec![(0u32, 0.0f64, 0.0f64); n];
            for (ai, area) in areas.iter().enumerate() {
                let ours_idx = classrooms.iter().position(|c| c.area == ai);
                let ours = ours_idx.map(|ci| {
                    let c = &classrooms[ci];
                    let stats = c.stats(&calendar, ctx.materials);
                    [0, 1, 2].map(|gi| c.attractiveness(gi, &stats, ctx.brand, ctx.trust))
                });
                let inquiries = distribute_area_demand(ai, area, season, ours, competitors, ctx.rng);
                if let Some(ci) = ours_idx {
                    let (new, trials) = classrooms[ci].admit(&mut ctx, inquiries);
                    admitted[ci] = (new, trials, 0.0);
                }
            }
            market::competitor_churn(competitors, ctx.rng);

            // エリアのシェア
            for (ci, c) in classrooms.iter().enumerate() {
                let ours = c.students.len() as f64;
                let others: u32 = competitors.iter().map(|k| k.students_in_area(c.area)).sum();
                admitted[ci].2 = if ours + others as f64 > 0.0 { ours / (ours + others as f64) } else { 0.0 };
            }

            let mut reports = Vec::with_capacity(n);
            for (ci, c) in classrooms.iter_mut().enumerate() {
                let (new, trials, share) = admitted[ci];
                let r = c.finish_week(&mut ctx, &areas[c.area], new, trials, share, std::mem::take(&mut news[ci]));
                ctx.cash += r.pnl.net_profit();
                reports.push(r);
            }
            *cash = ctx.cash;
            reports
        };

        // --- 本部 ---
        let mut hq = std::mem::take(&mut self.pending_hq);
        let mut flows = std::mem::take(&mut self.pending_flows);
        self.service_loans(&mut hq, &mut flows, &mut company_news);
        let (dev_cost, finished) = self.materials.advance();
        hq.development += dev_cost;
        if let Some(axis) = finished {
            company_news.push(format!("自社教材の「{}」の改訂が完了しました", materials::AXIS_LABELS[axis]));
        }
        hq.overhead = HQ_OVERHEAD_PER_EXTRA_CLASSROOM * (n as Yen - 1).max(0);
        self.cash -= dev_cost + hq.overhead;

        // --- 競合の動き（4 週ごと）---
        if calendar.week % 4 == 2 {
            self.run_competitors(&mut company_news);
        }

        // --- 資金ショート ---
        if self.cash < 0 {
            self.negative_weeks += 1;
            self.trust = (self.trust - 0.01).max(0.0);
            company_news.push(format!(
                "資金がマイナスです（{}週目）。取引先や保護者の信頼が下がっています。融資や経費削減を検討してください",
                self.negative_weeks
            ));
        } else {
            self.negative_weeks = 0;
        }

        // --- 不祥事 ---
        if self.trust < 0.35 && self.rng.chance((0.35 - self.trust) * 0.15) {
            self.brand = (self.brand - 0.1).max(0.0);
            for c in self.classrooms.iter_mut() {
                c.awareness = (c.awareness - 0.1).max(0.0);
                c.ratings.safety = (c.ratings.safety - 1.0).max(1.0);
            }
            self.trust += 0.1;
            company_news.push("口コミサイトで指導方針が批判され、炎上しました".into());
        }

        self.history.push(CompanyWeek {
            date: calendar.date(),
            school_year: calendar.school_year,
            week: calendar.week,
            classrooms: self
                .classrooms
                .iter()
                .zip(reports.iter())
                .map(|(c, r)| (c.name.clone(), r.pnl.revenue(), r.pnl.net_profit(), r.total_students()))
                .collect(),
            hq,
            flows,
            cash: self.cash,
            news: company_news,
        });

        // --- イベント ---
        self.weeks_since_event += 1;
        self.maybe_trigger_event();

        if calendar.week.is_multiple_of(8) {
            self.refresh_manager_candidates();
        }

        // --- 暦を進める ---
        if self.calendar.advance() {
            self.run_entrance_exams();
            self.new_school_year();
        }
        true
    }

    fn run_competitors(&mut self, company_news: &mut Vec<String>) {
        let presence: Vec<PlayerPresence> = self
            .classrooms
            .iter()
            .enumerate()
            .map(|(ci, c)| PlayerPresence {
                area: c.area,
                classroom: ci,
                students: c.students.len() as u32,
                weeks_open: c.history.len() as u32,
                best_teacher: c.best_teacher().map(|t| (t.id, t.skill)),
            })
            .collect();
        let date = self.calendar.date();
        let (moves, poach) =
            market::competitor_turn(&mut self.competitors, &self.areas, &presence, &mut self.rng, date);
        for m in moves {
            // 自塾の教室があるエリアの動きは、その教室のニュースにも載せる
            if let Some(c) = self.classrooms.iter_mut().find(|c| c.area == m.area) {
                if let Some(r) = c.history.last_mut() {
                    r.news.push(m.text.clone());
                }
                company_news.push(m.text.clone());
            }
            self.rival_log.insert(0, m);
        }
        self.rival_log.truncate(MAX_RIVAL_LOG);

        if let Some(p) = poach {
            if self.pending_event.is_none() {
                let c = &self.classrooms[p.classroom];
                if let Some(t) = c.teachers.iter().find(|t| t.id == p.teacher_id) {
                    let rival = &self.competitors[p.competitor];
                    self.rival_log.insert(
                        0,
                        RivalMove {
                            date,
                            competitor: p.competitor,
                            area: c.area,
                            kind: market::RivalMoveKind::Poach,
                            text: format!("{}が{}の{}先生に移籍を持ちかけました", rival.name, c.name, t.name),
                        },
                    );
                    let ev = GameEvent {
                        kind: EventKind::Poaching { teacher_id: t.id, competitor: p.competitor },
                        classroom: Some(p.classroom),
                        title: "人気講師の引き抜き".into(),
                        body: format!(
                            "{}が、{}の人気講師・{}先生（授業力 {}）に好条件で移籍を持ちかけている。",
                            rival.name, c.name, t.name, t.skill
                        ),
                        choice_a: "給与を 2 割上げて引き止める".into(),
                        choice_b: "本人の意思を尊重して送り出す".into(),
                    };
                    self.raise_event(ev);
                }
            }
        }
    }

    /// イベントを出す。保護者対応・トラブルを委任されていれば教室長が判断する。
    fn raise_event(&mut self, ev: GameEvent) {
        self.weeks_since_event = 0;
        let ci = ev.classroom.unwrap_or(0).min(self.classrooms.len() - 1);
        let manager = &self.classrooms[ci].manager;
        if ev.classroom.is_some() && manager.delegated(Delegation::Trouble) {
            let ethics = manager.ethics as f64 / 100.0;
            let choice = if self.rng.chance(ethics) { Choice::B } else { Choice::A };
            let label = if choice == Choice::A { &ev.choice_a } else { &ev.choice_b };
            let msg = format!("教室長が「{}」に対応：{}", ev.title, label);
            if let Some(r) = self.classrooms[ci].history.last_mut() {
                r.news.push(msg);
            }
            self.apply_event_choice(&ev, choice);
        } else {
            self.pending_event = Some(ev);
        }
    }

    fn maybe_trigger_event(&mut self) {
        if self.pending_event.is_some() || self.weeks_since_event < 5 {
            return;
        }
        let season = self.calendar.season();
        let week = self.calendar.week;
        let any_scholar = self.classrooms.iter().any(|c| c.students.iter().any(|s| s.grade == 6 && s.scholarship));
        if week == 49 && any_scholar {
            self.raise_event(GameEvent {
                kind: EventKind::ResultPadding,
                classroom: None,
                title: "合格実績の「上積み」提案".into(),
                body:
                    "本部スタッフが「特待生に多くの学校を受けてもらえば、合格者数を大きく見せられます」と提案してきた。"
                        .into(),
                choice_a: "特待生に多くの学校を受験させ、合格数を稼ぐ".into(),
                choice_b: "本人が志望する学校だけを受験させる".into(),
            });
            return;
        }
        if !self.rng.chance(0.06) {
            return;
        }
        let ci = self.rng.below(self.classrooms.len());
        let cname = self.classrooms[ci].name.clone();
        if self.rng.chance(0.5) || !matches!(season, Season::Autumn | Season::ExamRun) {
            self.raise_event(GameEvent {
                kind: EventKind::HomeworkRequest,
                classroom: Some(ci),
                title: "宿題を増やしてほしい".into(),
                body: format!("{cname}の教育熱心な保護者たちから「もっと宿題を出してほしい」という要望が届いた。"),
                choice_a: "全体の宿題量を 1 段階増やす".into(),
                choice_b: "希望する家庭にだけ個別課題を出す（¥60,000）".into(),
            });
        } else {
            let schools = &self.schools;
            let found = self.classrooms[ci]
                .students
                .iter()
                .filter(|s| s.grade == 6)
                .find(|s| schools[s.target_school].deviation - s.deviation > 6.0)
                .map(|s| (s.id, s.name.clone(), schools[s.target_school].name.clone()));
            if let Some((id, name, school)) = found {
                self.raise_event(GameEvent {
                    kind: EventKind::StrugglingSenior { student_id: id },
                    classroom: Some(ci),
                    title: "第一志望に届かない 6 年生".into(),
                    body: format!(
                        "{cname}の{name}さんの成績は、第一志望の{school}にまだ届いていない。合格率を守るなら志望校を下げさせる手もあるが……"
                    ),
                    choice_a: "志望校を下げるよう説得する".into(),
                    choice_b: "挑戦を支える個別指導をつける（¥80,000）".into(),
                });
            }
        }
    }

    fn run_entrance_exams(&mut self) {
        let year = self.calendar.school_year - 1;
        let brand_before = self.brand;
        let mut entries = Vec::new();
        let mut tallies: Vec<SchoolTally> =
            (0..self.schools.len()).map(|i| SchoolTally { school: i, takers: 0, passes: 0 }).collect();
        let mut first_choice_passes = 0;
        let mut candidates = 0;

        for c in &self.classrooms {
            for s in c.students.iter().filter(|s| s.grade == 6) {
                candidates += 1;
                let mut plan = vec![s.target_school];
                let safety = gen::school_near(&self.schools, s.deviation - 7.0, false);
                if safety != s.target_school {
                    plan.push(safety);
                }
                if self.padding_this_year && s.scholarship {
                    for (i, sch) in self.schools.iter().enumerate() {
                        if !plan.contains(&i) && sch.deviation <= s.deviation + 2.0 && plan.len() < 7 {
                            plan.push(i);
                        }
                    }
                }
                for (k, &school) in plan.iter().enumerate() {
                    let sch = &self.schools[school];
                    let bonus = self.materials.exam_bonus(sch.tier == SchoolTier::Top);
                    let x = (s.deviation + bonus - sch.deviation + 2.0) / 4.0;
                    let passed = self.rng.chance(soft_step(x));
                    tallies[school].takers += 1;
                    if passed {
                        tallies[school].passes += 1;
                    }
                    if k == 0 && passed {
                        first_choice_passes += 1;
                    }
                    entries.push(ExamEntry {
                        student_name: s.name.clone(),
                        classroom: c.name.clone(),
                        school,
                        passed,
                        first_choice: k == 0,
                        start_deviation: s.start_deviation,
                        final_deviation: s.deviation,
                    });
                }
            }
        }

        let score: f64 = tallies.iter().map(|t| t.passes as f64 * self.schools[t.school].tier.weight()).sum();
        self.brand = clamp01(self.brand * 0.55 + clamp01(score / 60.0) * 0.45);

        let mut stories: Vec<&ExamEntry> = entries.iter().filter(|e| e.first_choice && e.passed).collect();
        stories.sort_by(|a, b| {
            let ga = self.schools[a.school].deviation - a.start_deviation;
            let gb = self.schools[b.school].deviation - b.start_deviation;
            gb.partial_cmp(&ga).unwrap_or(std::cmp::Ordering::Equal)
        });
        let multi = self.classrooms.len() > 1;
        let highlights = stories
            .iter()
            .take(3)
            .map(|e| {
                let who = if multi {
                    format!("{}・{}さん", e.classroom, e.student_name)
                } else {
                    format!("{}さん", e.student_name)
                };
                format!(
                    "{who}：入塾時の偏差値 {:.0} から、第一志望の{}に合格！",
                    e.start_deviation, self.schools[e.school].name
                )
            })
            .collect();

        tallies.retain(|t| t.takers > 0);
        tallies.sort_by(|a, b| {
            self.schools[b.school]
                .deviation
                .partial_cmp(&self.schools[a.school].deviation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.exam_results.push(ExamResult {
            school_year: year,
            candidates,
            entries,
            tallies,
            first_choice_passes,
            brand_before,
            brand_after: self.brand,
            padded: self.padding_this_year,
            highlights,
        });
        self.padding_this_year = false;
        self.unseen_exam_result = true;
    }

    fn new_school_year(&mut self) {
        for c in self.classrooms.iter_mut() {
            c.students.retain(|s| s.grade < 6);
            for s in c.students.iter_mut() {
                s.grade += 1;
                s.taking_extra = false;
            }
        }
        market::competitor_new_year(&mut self.competitors, &mut self.rng);
        self.materials.yearly_decay();
        // 地価は年 2% 上がる
        for a in self.areas.iter_mut() {
            a.property_price = a.property_price * 102 / 100;
        }
        for c in self.classrooms.iter_mut().filter(|c| c.owned) {
            c.property_value = c.property_value * 102 / 100;
        }
        // 引き抜かれた看板講師の穴を、競合は 1 年かけて埋める
        for c in self.competitors.iter_mut().filter(|c| !c.acquired && !c.branches.is_empty()) {
            let want = match c.style {
                crate::market::CompetitorStyle::Elite => 2,
                crate::market::CompetitorStyle::Independent => 0,
                _ => 1,
            };
            while c.stars.len() < want {
                c.stars.push(gen::make_star(&mut self.rng, &mut self.next_id));
            }
        }
    }
}
