//! 画面とゲーム進行。

mod art;
mod fmt;
pub mod images;
mod portraits;
mod screens;

use juku_platform::{App, Ctx};
use juku_render::{Color, FontWeight, Rect};
use juku_sim::{Choice, World};
use juku_ui::{Align, Id, Ui};

pub const SAVE_KEY: &str = "autosave";
/// 秘書の自動報告の設定。
const AUTO_REPORT_KEY: &str = "auto_report";
/// チュートリアルを最後まで見た（またはスキップした）。
const TUTORIAL_KEY: &str = "tutorial_done";
/// 画面の最大幅（PC の横長ウィンドウでも縦長のレイアウトを保つ）
const MAX_WIDTH: f32 = 520.0;
const HEADER_H: f32 = 150.0;
const TAB_H: f32 = 70.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Classroom,
    Courses,
    Marketing,
    Accounting,
    Hq,
}

impl Tab {
    const ALL: [Tab; 5] = [Tab::Classroom, Tab::Courses, Tab::Marketing, Tab::Accounting, Tab::Hq];

    fn label(self) -> &'static str {
        match self {
            Tab::Classroom => "教室",
            Tab::Courses => "コース",
            Tab::Marketing => "集客",
            Tab::Accounting => "経理",
            Tab::Hq => "本部",
        }
    }
}

/// 確認してから実行する操作。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    NewGame,
    Reload,
    BuyProperty(usize),
    SellProperty(usize),
    /// 塾の買収（competitors の添字）
    Acquire(usize),
    /// 繰り上げ返済（Loan の id）
    Repay(u32),
    StartIpo,
    CancelIpo,
    StopSales,
}

/// スター講師への声かけ（モーダル表示中）。
#[derive(Clone, Copy, Debug)]
struct StarOffer {
    competitor: usize,
    star: usize,
    /// 配属先の教室
    classroom: usize,
}

/// 秘書のウインドウ（表示中）。
struct SecretaryView {
    items: Vec<juku_sim::advisor::Advice>,
    index: usize,
}

pub struct GameApp {
    world: Option<World>,
    has_save: bool,
    /// 保存データの要約（スタート画面に出す）
    save_summary: Option<screens::title::SaveSummary>,
    tab: Tab,
    /// 表示中の教室（World::classrooms の添字）
    selected: usize,
    /// コース画面のセグメント（0 = 本科, 1 = 講習・特訓）
    course_segment: usize,
    /// 経理画面のセグメント（0 = 教室, 1 = 全社）
    accounting_segment: usize,
    /// 開校するエリア（教室長を選ぶモーダルを表示中）
    opening_area: Option<usize>,
    /// 表示中の入試結果（exam_results の添字）
    exam_view: Option<usize>,
    toast: Option<(String, f64)>,
    confirm: Option<Confirm>,
    star_offer: Option<StarOffer>,
    /// 本部タブのセクション
    hq_section: usize,
    /// タウンマップで選んでいるエリア
    map_area: Option<usize>,
    /// 秘書のウインドウ
    secretary: Option<SecretaryView>,
    /// もう伝えた報告（同じことを何度も言わない）
    seen_advice: std::collections::HashSet<String>,
    /// 週を進めたとき、新しい重要な報告があれば秘書が知らせる
    auto_report: bool,
    /// チュートリアル（表示中）
    tutorial: Option<screens::tutorial::Tutorial>,
    tutorial_done: bool,
    seed_counter: u64,
}

impl GameApp {
    pub fn new(storage: &mut juku_platform::Storage) -> Self {
        let save_summary = storage
            .load(SAVE_KEY)
            .and_then(|b| postcard::from_bytes::<World>(&b).ok())
            .filter(|w| w.save_version == juku_sim::SAVE_VERSION)
            .map(|w| screens::title::SaveSummary::from_world(&w));
        GameApp {
            world: None,
            has_save: save_summary.is_some(),
            save_summary,
            tab: Tab::Classroom,
            selected: 0,
            course_segment: 0,
            accounting_segment: 0,
            opening_area: None,
            exam_view: None,
            toast: None,
            confirm: None,
            star_offer: None,
            hq_section: 0,
            map_area: None,
            secretary: None,
            seen_advice: Default::default(),
            auto_report: storage.load(AUTO_REPORT_KEY).map(|b| b != b"0").unwrap_or(true),
            tutorial: None,
            tutorial_done: storage.load(TUTORIAL_KEY).is_some(),
            seed_counter: 0,
        }
    }

    fn save(&mut self, ctx: &mut Ctx) {
        if let Some(w) = &self.world {
            match postcard::to_allocvec(w) {
                Ok(bytes) => {
                    if let Err(e) = ctx.storage.save(SAVE_KEY, &bytes) {
                        log::warn!("保存に失敗しました: {e}");
                    } else {
                        self.has_save = true;
                        self.save_summary = Some(screens::title::SaveSummary::from_world(w));
                    }
                }
                Err(e) => log::warn!("保存データの作成に失敗しました: {e}"),
            }
        }
    }

    fn load(&mut self, ctx: &mut Ctx) -> bool {
        let w = ctx.storage.load(SAVE_KEY).and_then(|b| postcard::from_bytes::<World>(&b).ok());
        match w {
            Some(w) if w.save_version == juku_sim::SAVE_VERSION => {
                self.world = Some(w);
                self.selected = 0;
                true
            }
            _ => false,
        }
    }

    fn new_game(&mut self, time: f64) {
        self.seed_counter += 1;
        let seed = (time * 1000.0) as u64 ^ self.seed_counter.wrapping_mul(0x9E37_79B9);
        self.world = Some(World::new(seed));
        self.tab = Tab::Classroom;
        self.selected = 0;
        self.secretary = None;
        self.map_area = None;
        self.seen_advice.clear();
        // はじめて遊ぶ人には、秘書が画面を案内する
        if !self.tutorial_done {
            self.tutorial = Some(screens::tutorial::Tutorial::new());
        }
    }

    fn show_toast(&mut self, msg: impl Into<String>, now: f64) {
        self.toast = Some((msg.into(), now + 3.5));
    }
}

impl App for GameApp {
    const SPRITE_ATLAS_PNGS: &'static [&'static [u8]] = portraits::ATLAS_PNGS;

    fn update(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        let s = ui.screen();
        let w = s.w.min(MAX_WIDTH);
        let x0 = s.x + (s.w - w) * 0.5;
        let theme = ui.theme;
        // 横の余白（PC の横長ウィンドウ）
        if w < s.w {
            ui.dl.rect(Rect::new(s.x, s.y, x0 - s.x, s.h), theme.band, 0.0);
            ui.dl.rect(Rect::new(x0 + w, s.y, s.right() - x0 - w, s.h), theme.band, 0.0);
        }
        art::chalkboard(ui, Rect::new(x0, s.y, w, s.h));

        if self.world.is_none() {
            match screens::title::draw(
                ui,
                Rect::new(x0, s.y, w, s.h),
                self.save_summary.as_ref().filter(|_| self.has_save),
                !self.tutorial_done,
            ) {
                Some(screens::title::TitleAction::Continue) => {
                    if !self.load(ctx) {
                        self.has_save = false;
                        let now = ui.input.time;
                        self.show_toast("保存データを読み込めませんでした", now);
                    }
                }
                Some(screens::title::TitleAction::NewGame) => {
                    let now = ui.input.time;
                    self.new_game(now);
                    self.save(ctx);
                }
                None => {}
            }
            self.draw_toast(ui, x0, w);
            return;
        }

        let modal_open = {
            let world = self.world.as_ref().unwrap();
            world.pending_event.is_some()
                || world.bankrupt
                || world.unseen_exam_result
                || self.exam_view.is_some()
                || self.confirm.is_some()
                || self.opening_area.is_some()
                || self.star_offer.is_some()
                || self.secretary.is_some()
                || self.tutorial.is_some()
        };
        // チュートリアル中は、案内しているタブを開く
        if let Some(tab) = self.tutorial.as_ref().and_then(|t| t.tab()) {
            self.tab = tab;
        }
        let n_classrooms = self.world.as_ref().unwrap().classrooms.len();
        self.selected = self.selected.min(n_classrooms - 1);
        let ci = self.selected;
        ui.input_enabled = !modal_open;

        let viewport = Rect::new(x0, s.y + HEADER_H, w, s.h - HEADER_H - TAB_H);
        let tab = self.tab;
        // 本部タブは会社全体、それ以外は教室ごとにスクロール位置を覚える
        let id = Id::new("tab-scroll").with(tab as u64).with(if tab == Tab::Hq { 0 } else { ci as u64 + 1 });
        {
            let world = self.world.as_mut().unwrap();
            let seg = &mut self.course_segment;
            let acc_seg = &mut self.accounting_segment;
            let hq_seg = &mut self.hq_section;
            let map_area = &mut self.map_area;
            let mut hq_action = None;
            let is_web = ctx.is_web;
            let auto_report = self.auto_report;
            ui.scroll_area(id, viewport, 0.0, |ui| {
                ui.padded(16.0, 18.0, |ui| {
                    ui.set_spacing(16.0);
                    match tab {
                        Tab::Classroom => screens::classroom::draw(ui, world, ci),
                        Tab::Courses => screens::courses::draw(ui, world, ci, seg),
                        Tab::Marketing => screens::marketing::draw(ui, world, ci),
                        Tab::Accounting => screens::accounting::draw(ui, world, ci, acc_seg),
                        Tab::Hq => hq_action = screens::hq::draw(ui, world, is_web, hq_seg, auto_report, map_area),
                    }
                });
            });
            let now = ui.input.time;
            match hq_action {
                Some(screens::hq::HqAction::ShowExam(i)) => self.exam_view = Some(i),
                Some(screens::hq::HqAction::NewGame) => self.confirm = Some(Confirm::NewGame),
                Some(screens::hq::HqAction::Save) => {
                    self.save(ctx);
                    self.show_toast("保存しました", now);
                }
                Some(screens::hq::HqAction::Select(i)) => {
                    self.selected = i;
                    self.tab = Tab::Classroom;
                }
                Some(screens::hq::HqAction::Open(area)) => self.opening_area = Some(area),
                Some(screens::hq::HqAction::Toast(msg)) => self.show_toast(msg, now),
                Some(screens::hq::HqAction::ForceReload) => self.confirm = Some(Confirm::Reload),
                Some(screens::hq::HqAction::AutoReport(on)) => self.set_auto_report(on, ctx),
                Some(screens::hq::HqAction::Tutorial) => {
                    self.tutorial = Some(screens::tutorial::Tutorial::new());
                    self.selected = 0;
                }
                Some(screens::hq::HqAction::Confirm(c)) => self.confirm = Some(c),
                Some(screens::hq::HqAction::Star(competitor, star)) => {
                    self.star_offer = Some(StarOffer { competitor, star, classroom: ci })
                }
                None => {}
            }
        }

        // ヘッダー
        let header = Rect::new(x0, s.y, w, HEADER_H);
        let notable = {
            let world = self.world.as_ref().unwrap();
            juku_sim::advisor::advise(world)
                .iter()
                .filter(|a| a.priority >= juku_sim::advisor::Priority::Important)
                .count()
        };
        let hr = {
            let world = self.world.as_mut().unwrap();
            screens::header::draw(ui, header, world, ci, notable)
        };
        if hr.stepped {
            self.save(ctx);
            self.after_week();
        }
        if let Some(next) = hr.switch {
            self.selected = next;
        }
        if hr.secretary {
            let items = juku_sim::advisor::advise(self.world.as_ref().unwrap());
            for a in &items {
                self.seen_advice.insert(a.key.clone());
            }
            self.secretary = Some(SecretaryView { items, index: 0 });
        }

        // タブバー
        let bar = Rect::new(x0, s.bottom() - TAB_H, w, TAB_H);
        if let Some(t) = draw_tab_bar(ui, bar, self.tab) {
            self.tab = t;
        }

        // モーダル。このフレームで開いたばかりのモーダルには入力を渡さない
        // （開くきっかけになったタップが、同じ位置にあるモーダルのボタンまで押してしまうため）
        ui.input_enabled = modal_open;
        let area = Rect::new(x0, s.y + HEADER_H, w, s.h - HEADER_H - TAB_H);
        if let Some(tut) = &mut self.tutorial {
            let layout = screens::tutorial::Layout { screen: s, x0, w, header_h: HEADER_H, tab_h: TAB_H };
            let world = self.world.as_mut().unwrap();
            if let screens::tutorial::Outcome::Done = tut.draw(ui, world, &layout) {
                self.tutorial = None;
                self.tutorial_done = true;
                let _ = ctx.storage.save(TUTORIAL_KEY, b"1");
                self.tab = Tab::Classroom;
                self.save(ctx);
            }
            self.draw_toast(ui, x0, w);
            return;
        }
        if let Some(view) = &mut self.secretary {
            if self
                .world
                .as_ref()
                .map(|w| w.pending_event.is_none() && !w.unseen_exam_result && !w.bankrupt)
                .unwrap_or(false)
            {
                match screens::secretary::reports(ui, area, &view.items, view.index) {
                    Some(screens::secretary::Action::Prev) => view.index = view.index.saturating_sub(1),
                    Some(screens::secretary::Action::Next) => view.index += 1,
                    Some(screens::secretary::Action::Close) => self.secretary = None,
                    Some(screens::secretary::Action::Goto) => {
                        if let Some(a) = view.items.get(view.index) {
                            let (tab, section) = topic_target(a.topic);
                            self.tab = tab;
                            if let Some(sec) = section {
                                self.hq_section = sec;
                            }
                            if let Some(ci) = a.classroom {
                                self.selected = ci;
                            }
                        }
                        self.secretary = None;
                    }
                    None => {}
                }
                self.draw_toast(ui, x0, w);
                return;
            }
        }
        let world = self.world.as_mut().unwrap();
        if world.bankrupt {
            if screens::modals::bankrupt(ui, world) {
                let now = ui.input.time;
                self.new_game(now);
                self.save(ctx);
            }
        } else if let Some(ev) = &world.pending_event {
            if let Some(choice) = screens::modals::event(ui, ev, world) {
                world.resolve_event(choice);
                if choice == Choice::A || choice == Choice::B {
                    self.save(ctx);
                }
            }
        } else if world.unseen_exam_result {
            let idx = world.exam_results.len() - 1;
            if screens::modals::exam(ui, world, idx) {
                world.unseen_exam_result = false;
                self.save(ctx);
            }
        } else if let Some(idx) = self.exam_view {
            if screens::modals::exam(ui, world, idx) {
                self.exam_view = None;
            }
        } else if let Some(area) = self.opening_area {
            match screens::modals::open_classroom(ui, world, area) {
                Some(Some(manager)) => match world.open_classroom(area, manager) {
                    Ok(new_ci) => {
                        self.opening_area = None;
                        self.selected = new_ci;
                        self.tab = Tab::Classroom;
                        let name = world.classrooms[new_ci].name.clone();
                        self.save(ctx);
                        let now = ui.input.time;
                        self.show_toast(format!("{name}を開校しました"), now);
                    }
                    Err(e) => {
                        self.opening_area = None;
                        let now = ui.input.time;
                        self.show_toast(e, now);
                    }
                },
                Some(None) => self.opening_area = None,
                None => {}
            }
        } else if let Some(offer) = self.star_offer {
            match screens::modals::star_offer(ui, world, offer.competitor, offer.star, offer.classroom) {
                Some(screens::modals::StarChoice::Classroom(next)) => {
                    self.star_offer = Some(StarOffer { classroom: next, ..offer });
                }
                Some(screens::modals::StarChoice::Offer(level)) => {
                    self.star_offer = None;
                    let name = world.competitors[offer.competitor].stars[offer.star].name.clone();
                    let msg = match world.offer_star(offer.competitor, offer.star, offer.classroom, level) {
                        Ok(true) => format!("{name}先生が{}に来てくれました！", world.classrooms[offer.classroom].name),
                        Ok(false) => format!("{name}先生に断られました"),
                        Err(e) => e,
                    };
                    self.save(ctx);
                    let now = ui.input.time;
                    self.show_toast(msg, now);
                }
                Some(screens::modals::StarChoice::Cancel) => self.star_offer = None,
                None => {}
            }
        } else if let Some(c) = self.confirm {
            let (title, body, ok) = confirm_text(world, c);
            match screens::modals::confirm(ui, &title, &body, &ok) {
                Some(true) => {
                    self.confirm = None;
                    let now = ui.input.time;
                    let result: Result<String, String> = match c {
                        Confirm::NewGame => {
                            self.new_game(now);
                            Ok(String::new())
                        }
                        Confirm::Reload => {
                            self.save(ctx);
                            ctx.force_reload();
                            Ok("再読み込みしています…".into())
                        }
                        Confirm::BuyProperty(ci) => world.buy_property(ci).map(|_| "物件を購入しました".into()),
                        Confirm::SellProperty(ci) => {
                            world.sell_property(ci).map(|v| format!("物件を {} で売却しました", fmt::format_yen(v)))
                        }
                        Confirm::Acquire(comp) => world.acquire(comp).map(|lines| lines.join("。")),
                        Confirm::Repay(id) => world.repay_loan(id).map(|_| "繰り上げ返済しました".into()),
                        Confirm::StartIpo => world.start_ipo().map(|_| "上場準備を始めました".into()),
                        Confirm::CancelIpo => {
                            world.cancel_ipo();
                            Ok("上場準備をやめました".into())
                        }
                        Confirm::StopSales => {
                            world.stop_material_sales();
                            Ok("教材の外販をやめました".into())
                        }
                    };
                    if c != Confirm::Reload {
                        self.save(ctx);
                    }
                    match result {
                        Ok(msg) if !msg.is_empty() => self.show_toast(msg, now),
                        Ok(_) => {}
                        Err(e) => self.show_toast(e, now),
                    }
                }
                Some(false) => self.confirm = None,
                None => {}
            }
        }
        self.draw_toast(ui, x0, w);
    }
}

impl GameApp {
    /// 週を進めた後：新しく起きた重要なことがあれば、秘書が知らせる。
    fn after_week(&mut self) {
        if !self.auto_report {
            return;
        }
        let Some(world) = &self.world else { return };
        let fresh: Vec<juku_sim::advisor::Advice> = juku_sim::advisor::advise(world)
            .into_iter()
            .filter(|a| a.priority >= juku_sim::advisor::Priority::Important && !self.seen_advice.contains(&a.key))
            .collect();
        if fresh.is_empty() {
            return;
        }
        for a in &fresh {
            self.seen_advice.insert(a.key.clone());
        }
        self.secretary = Some(SecretaryView { items: fresh, index: 0 });
    }

    fn set_auto_report(&mut self, on: bool, ctx: &mut Ctx) {
        self.auto_report = on;
        let _ = ctx.storage.save(AUTO_REPORT_KEY, if on { b"1" } else { b"0" });
    }

    fn draw_toast(&mut self, ui: &mut Ui, x0: f32, w: f32) {
        let now = ui.input.time;
        if let Some((msg, until)) = &self.toast {
            if now > *until {
                self.toast = None;
                return;
            }
            let t = ui.theme;
            let s = ui.screen();
            // 長いメッセージは折り返す
            let max_w = w - 48.0;
            let lines = ui.text.wrap(msg, t.font_body, FontWeight::Bold, max_w - 40.0);
            let lh = ui.text.line_height(t.font_body);
            let tw = lines.iter().map(|l| ui.measure(l, t.font_body, FontWeight::Bold)).fold(0.0f32, f32::max) + 40.0;
            let th = lh * lines.len() as f32 + 18.0;
            let r = Rect::new(x0 + (w - tw) * 0.5, s.bottom() - TAB_H - 26.0 - th, tw, th);
            ui.dl.rect_border(r, t.bar, 22.0_f32.min(th * 0.5), 1.5, t.accent);
            for (i, line) in lines.iter().enumerate() {
                let lr = Rect::new(r.x, r.y + 9.0 + lh * i as f32, r.w, lh);
                ui.text_in(lr, line, t.font_body, FontWeight::Bold, t.text, Align::Center);
            }
            ui.mem.request_repaint();
        }
    }
}

fn draw_tab_bar(ui: &mut Ui, r: Rect, current: Tab) -> Option<Tab> {
    let t = ui.theme;
    ui.dl.rect(r, t.bar, 0.0);
    ui.dl.rect(Rect::new(r.x, r.y, r.w, 1.0), t.card_border, 0.0);
    let n = Tab::ALL.len() as f32;
    let cw = r.w / n;
    let mut out = None;
    for (i, tab) in Tab::ALL.iter().enumerate() {
        let cell = Rect::new(r.x + cw * i as f32, r.y, cw, r.h);
        let active = *tab == current;
        if active {
            ui.dl.rect(cell.shrink(6.0), t.card, 12.0);
        }
        let color = if active { t.accent } else { t.text_dim };
        let icon = Rect::new(cell.x, cell.y + 8.0, cell.w, 30.0);
        art::tab_icon(ui, *tab, icon, color);
        let label = Rect::new(cell.x, cell.y + 38.0, cell.w, 22.0);
        ui.text_in(label, tab.label(), t.font_small, FontWeight::Bold, color, Align::Center);
        if ui.clicked(cell) && !active {
            out = Some(*tab);
        }
    }
    out
}

/// 前週比の文字列と色。`good_when_up` が false なら増加を悪い変化として色付けする。
pub(crate) fn delta(ui: &Ui, cur: i64, prev: i64, good_when_up: bool) -> (String, Color) {
    let d = cur - prev;
    let t = ui.theme;
    let color = if d == 0 {
        t.text_dim
    } else if (d > 0) == good_when_up {
        t.positive
    } else {
        t.negative
    };
    (juku_sim::format_signed(d), color)
}

/// 確認ダイアログの文言：(タイトル, 本文, 実行ボタン)。
fn confirm_text(world: &World, c: Confirm) -> (String, String, String) {
    use fmt::format_yen;
    match c {
        Confirm::NewGame => (
            "新しいゲーム".into(),
            "今の進行状況は失われます。最初からやり直しますか？".into(),
            "最初から始める".into(),
        ),
        Confirm::Reload => (
            "キャッシュを削除して再読み込み".into(),
            "ブラウザに残っている古いファイルを削除して、最新版を読み込み直します。セーブデータは消えません（今の状態を保存してから再読み込みします）。".into(),
            "削除して再読み込み".into(),
        ),
        Confirm::BuyProperty(ci) => {
            let c = &world.classrooms[ci];
            let price = world.property_price(ci);
            (
                format!("{}の物件を購入", c.name),
                format!(
                    "{} で物件を買い取ります。家賃（{}/週）はなくなり、維持費がかかるようになります。",
                    format_yen(price),
                    format_yen(world.areas[c.area].rent)
                ),
                "購入する".into(),
            )
        }
        Confirm::SellProperty(ci) => {
            let c = &world.classrooms[ci];
            let v = c.property_value * 95 / 100;
            (
                format!("{}の物件を売却", c.name),
                format!("約 {} で売却します（手数料 5%）。以後は家賃がかかります。", format_yen(v)),
                "売却する".into(),
            )
        }
        Confirm::Acquire(comp) => {
            let target = &world.competitors[comp];
            let students = target.total_students();
            (
                format!("{}を買収", target.name),
                format!(
                    "{} で買収します。在籍 {students} 人のうち 1〜2 割は経営交代で辞めてしまいます。講師も引き継ぎます。",
                    format_yen(world.acquisition_price(comp))
                ),
                "買収する".into(),
            )
        }
        Confirm::StartIpo => (
            "上場準備を始める".into(),
            format!(
                "監査法人・証券会社と {} 週かけて準備します。費用は毎週 {}（総額 {}）かかります。準備が終わったときに条件を満たしていなければ、上場できず費用は戻りません。",
                juku_sim::listing::IPO_PREP_WEEKS,
                format_yen(juku_sim::listing::IPO_PREP_WEEKLY),
                format_yen(juku_sim::listing::IPO_PREP_WEEKLY * juku_sim::listing::IPO_PREP_WEEKS as i64)
            ),
            "準備を始める".into(),
        ),
        Confirm::CancelIpo => (
            "上場準備をやめる".into(),
            "ここまでの準備費用は戻りません。もう一度上場するには、最初から準備し直す必要があります。".into(),
            "やめる".into(),
        ),
        Confirm::StopSales => (
            "教材の外販をやめる".into(),
            format!("{} 塾との契約がすべて終わります。もう一度始めるには、営業体制をつくり直す費用がかかります。", world.materials.sales.contracts),
            "外販をやめる".into(),
        ),
        Confirm::Repay(id) => {
            let bal = world.loans.iter().find(|l| l.id == id).map(|l| l.balance).unwrap_or(0);
            ("繰り上げ返済".into(), format!("残高 {} を一括で返済します。", format_yen(bal)), "返済する".into())
        }
    }
}

/// 報告の行き先：(タブ, 本部のセクション)。
fn topic_target(topic: juku_sim::advisor::Topic) -> (Tab, Option<usize>) {
    use juku_sim::advisor::Topic;
    match topic {
        Topic::Classroom => (Tab::Classroom, None),
        Topic::Courses => (Tab::Courses, None),
        Topic::Marketing => (Tab::Marketing, None),
        Topic::Accounting => (Tab::Accounting, None),
        Topic::Expansion => (Tab::Hq, Some(0)),
        Topic::Finance => (Tab::Hq, Some(1)),
        Topic::Materials => (Tab::Hq, Some(2)),
        Topic::Exams => (Tab::Hq, Some(3)),
    }
}
