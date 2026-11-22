//! 画面とゲーム進行。

mod art;
mod fmt;
mod screens;

use juku_platform::{App, Ctx};
use juku_render::{Color, FontWeight, Rect};
use juku_sim::{Choice, World};
use juku_ui::{Align, Id, Ui};

pub const SAVE_KEY: &str = "autosave";
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

pub struct GameApp {
    world: Option<World>,
    has_save: bool,
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
    confirm_new_game: bool,
    confirm_reload: bool,
    seed_counter: u64,
}

impl GameApp {
    pub fn new(storage: &mut juku_platform::Storage) -> Self {
        let has_save = storage
            .load(SAVE_KEY)
            .and_then(|b| postcard::from_bytes::<World>(&b).ok())
            .map(|w| w.save_version == juku_sim::SAVE_VERSION)
            .unwrap_or(false);
        GameApp {
            world: None,
            has_save,
            tab: Tab::Classroom,
            selected: 0,
            course_segment: 0,
            accounting_segment: 0,
            opening_area: None,
            exam_view: None,
            toast: None,
            confirm_new_game: false,
            confirm_reload: false,
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
    }

    fn show_toast(&mut self, msg: impl Into<String>, now: f64) {
        self.toast = Some((msg.into(), now + 2.5));
    }
}

impl App for GameApp {
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
            match screens::title::draw(ui, Rect::new(x0, s.y, w, s.h), self.has_save) {
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
                || world.unseen_exam_result
                || self.exam_view.is_some()
                || self.confirm_new_game
                || self.opening_area.is_some()
                || self.confirm_reload
        };
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
            let mut hq_action = None;
            let is_web = ctx.is_web;
            ui.scroll_area(id, viewport, 0.0, |ui| {
                ui.padded(16.0, 18.0, |ui| {
                    ui.set_spacing(16.0);
                    match tab {
                        Tab::Classroom => screens::classroom::draw(ui, world, ci),
                        Tab::Courses => screens::courses::draw(ui, world, ci, seg),
                        Tab::Marketing => screens::marketing::draw(ui, world, ci),
                        Tab::Accounting => screens::accounting::draw(ui, world, ci, acc_seg),
                        Tab::Hq => hq_action = screens::hq::draw(ui, world, is_web),
                    }
                });
            });
            let now = ui.input.time;
            match hq_action {
                Some(screens::hq::HqAction::ShowExam(i)) => self.exam_view = Some(i),
                Some(screens::hq::HqAction::NewGame) => self.confirm_new_game = true,
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
                Some(screens::hq::HqAction::ForceReload) => self.confirm_reload = true,
                None => {}
            }
        }

        // ヘッダー
        let header = Rect::new(x0, s.y, w, HEADER_H);
        let (stepped, switch) = {
            let world = self.world.as_mut().unwrap();
            screens::header::draw(ui, header, world, ci)
        };
        if stepped {
            self.save(ctx);
        }
        if let Some(next) = switch {
            self.selected = next;
        }

        // タブバー
        let bar = Rect::new(x0, s.bottom() - TAB_H, w, TAB_H);
        if let Some(t) = draw_tab_bar(ui, bar, self.tab) {
            self.tab = t;
        }

        // モーダル
        ui.input_enabled = true;
        let world = self.world.as_mut().unwrap();
        if let Some(ev) = world.pending_event.clone() {
            let cname = ev.classroom.and_then(|i| world.classrooms.get(i)).map(|c| c.name.clone());
            if let Some(choice) = screens::modals::event(ui, &ev, cname.as_deref()) {
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
        } else if self.confirm_reload {
            match screens::modals::confirm(
                ui,
                "キャッシュを削除して再読み込み",
                "ブラウザに残っている古いファイルを削除して、最新版を読み込み直します。セーブデータは消えません（今の状態を保存してから再読み込みします）。",
                "削除して再読み込み",
            ) {
                Some(true) => {
                    self.confirm_reload = false;
                    self.save(ctx);
                    ctx.force_reload();
                    let now = ui.input.time;
                    self.show_toast("再読み込みしています…", now);
                }
                Some(false) => self.confirm_reload = false,
                None => {}
            }
        } else if self.confirm_new_game {
            match screens::modals::confirm(
                ui,
                "新しいゲーム",
                "今の進行状況は失われます。最初からやり直しますか？",
                "最初から始める",
            ) {
                Some(true) => {
                    self.confirm_new_game = false;
                    let now = ui.input.time;
                    self.new_game(now);
                    self.save(ctx);
                }
                Some(false) => self.confirm_new_game = false,
                None => {}
            }
        }
        self.draw_toast(ui, x0, w);
    }
}

impl GameApp {
    fn draw_toast(&mut self, ui: &mut Ui, x0: f32, w: f32) {
        let now = ui.input.time;
        if let Some((msg, until)) = &self.toast {
            if now > *until {
                self.toast = None;
                return;
            }
            let t = ui.theme;
            let s = ui.screen();
            let tw = ui.measure(msg, t.font_body, FontWeight::Bold) + 40.0;
            let r = Rect::new(x0 + (w - tw) * 0.5, s.bottom() - TAB_H - 70.0, tw, 44.0);
            ui.dl.rect_border(r, t.bar, 22.0, 1.5, t.accent);
            ui.text_in(r, msg, t.font_body, FontWeight::Bold, t.text, Align::Center);
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
