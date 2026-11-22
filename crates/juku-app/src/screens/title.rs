//! タイトル画面。

use juku_render::{Color, FontWeight, Rect};
use juku_ui::{Align, ButtonStyle, Ui};

use crate::art;

pub enum TitleAction {
    Continue,
    NewGame,
}

pub fn draw(ui: &mut Ui, r: Rect, has_save: bool) -> Option<TitleAction> {
    let t = ui.theme;
    let cx = r.x + r.w * 0.5;
    let top = r.y + r.h * 0.12;

    // 黒板の枠
    let board = Rect::new(r.x + 24.0, top, r.w - 48.0, 210.0);
    ui.dl.rect_border(board, Color::hex(0x23402F), 10.0, 8.0, Color::hex(0x8A6A4A));
    ui.text_in(
        Rect::new(board.x, board.y + 34.0, board.w, 30.0),
        "中学受験塾 経営シミュレーション",
        t.font_body,
        FontWeight::Regular,
        t.text_dim,
        Align::Center,
    );
    ui.text_in(
        Rect::new(board.x, board.y + 70.0, board.w, 60.0),
        "合格の街",
        46.0,
        FontWeight::Bold,
        t.text,
        Align::Center,
    );
    ui.text_in(
        Rect::new(board.x, board.y + 134.0, board.w, 30.0),
        "〜 塾長シミュレーター 〜",
        t.font_heading,
        FontWeight::Regular,
        t.star,
        Align::Center,
    );
    // チョーク
    ui.dl.rect(Rect::new(board.right() - 70.0, board.bottom() - 12.0, 30.0, 6.0), Color::WHITE, 3.0);
    ui.dl.rect(Rect::new(board.right() - 110.0, board.bottom() - 12.0, 22.0, 6.0), Color::hex(0xF7B6C8), 3.0);

    // 人物
    let figs = ["塾長", "生徒A", "生徒B"];
    for (i, name) in figs.iter().enumerate() {
        let w = if i == 0 { 120.0 } else { 90.0 };
        let x = cx
            + match i {
                0 => -60.0,
                1 => -170.0,
                _ => 80.0,
            };
        let y = board.bottom() + if i == 0 { 30.0 } else { 70.0 };
        art::person(ui, Rect::new(x, y, w, w * 1.2), art::Look::from_name(name), i == 0);
    }
    art::sakura(ui, r.x + 40.0, board.bottom() + 40.0, 14.0);
    art::sakura(ui, r.right() - 50.0, board.bottom() + 90.0, 10.0);

    let by = r.bottom() - 220.0;
    ui.begin(r.x + 40.0, by, r.w - 80.0, 14.0);
    let mut out = None;
    if has_save && ui.button("続きから", ButtonStyle::Primary) {
        out = Some(TitleAction::Continue);
    }
    let style = if has_save { ButtonStyle::Secondary } else { ButtonStyle::Primary };
    if ui.button("はじめから", style) {
        out = Some(TitleAction::NewGame);
    }
    ui.space(6.0);
    ui.label_ex(
        "あなたは小さな進学塾の塾長。生徒を育て、合格実績を積み上げ、地域一番の塾を目指そう。",
        t.font_small,
        FontWeight::Regular,
        t.text_dim,
        Align::Center,
    );
    ui.end();
    out
}
