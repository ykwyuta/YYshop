//! 秘書のウインドウ：左に秘書、右にメッセージ。経営の報告と、チュートリアルの両方で使う。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::advisor::{Advice, Priority};
use juku_ui::{Align, ButtonStyle, Id, Ui};

use crate::portraits::{Expression, Portrait};

pub const NAME: &str = "藤崎 さくら";

/// ウインドウに表示するボタン。
pub struct Button<'a> {
    pub label: &'a str,
    pub style: ButtonStyle,
}

/// 秘書のウインドウを描く。押されたボタンの添字を返す。
/// `area` は画面の中で使ってよい範囲（タブバーの上まで）。
pub fn window(
    ui: &mut Ui,
    area: Rect,
    portrait: Portrait,
    badge: Option<(&str, Color)>,
    title: &str,
    body: &str,
    page: Option<(usize, usize)>,
    buttons: &[Button],
) -> Option<usize> {
    let t = ui.theme;
    let w = area.w - 16.0;
    let portrait_w = (w * 0.28).clamp(92.0, 130.0);
    let bubble_w = w - portrait_w - 10.0;
    // 吹き出しは bubble_w - 10、左右の余白 16 ずつ
    let text_w = bubble_w - 10.0 - 32.0;

    // 本文の高さから、ウインドウの高さを決める
    // 句読点のぶら下げが本文の枠からはみ出さないよう、1 文字ぶん狭く折り返す
    let lines = ui.text.wrap(body, t.font_body, FontWeight::Regular, text_w - t.font_body);
    let lh = ui.text.line_height(t.font_body);
    let title_lines =
        ui.text.wrap(title, t.font_heading, FontWeight::Bold, text_w - if badge.is_some() { 48.0 } else { 0.0 });
    let tlh = ui.text.line_height(t.font_heading);
    let body_h = lh * lines.len() as f32;
    // 余白（外 10＋内 14）＋見出し＋本文＋ボタン列（52）＋下の余白（10）
    let content_h = 24.0 + tlh * title_lines.len() as f32 + 6.0 + body_h + 8.0 + 52.0 + 10.0;
    let h = content_h.clamp(190.0, area.h * 0.7);
    let r = Rect::new(area.x + 8.0, area.bottom() - h - 8.0, w, h);

    // 影と枠
    ui.dl.rect(r.translate(0.0, 4.0), Color::BLACK.with_alpha(0.35), 18.0);
    ui.dl.rect_border(r, t.bar, 18.0, 2.0, t.accent);

    // 左：秘書
    let pw = portrait_w - 10.0;
    let ph = (pw + 26.0).min(h - 20.0);
    let p = Rect::new(r.x + 10.0, r.y + (h - ph) * 0.5, pw, ph);
    ui.dl.rect_gradient(p, Color::hex(0x2F5A45), Color::hex(0x14201B), 12.0);
    portrait.draw(ui, Rect::new(p.x + 4.0, p.y + 4.0, p.w - 8.0, p.h - 34.0));
    let plate = Rect::new(p.x, p.bottom() - 26.0, p.w, 26.0);
    ui.dl.rect(plate, Color::BLACK.with_alpha(0.55), 8.0);
    ui.text_in(
        plate,
        &format!("秘書 {}", NAME.split(' ').next().unwrap_or(NAME)),
        t.font_small,
        FontWeight::Bold,
        t.text,
        Align::Center,
    );

    // 右：吹き出し
    let b = Rect::new(r.x + portrait_w + 10.0, r.y + 10.0, bubble_w - 10.0, h - 20.0);
    ui.dl.rect(b, Color::hex(0xF7F4EA), 14.0);
    // しっぽ
    let ty = b.y + 36.0;
    for k in 0..6 {
        let k = k as f32;
        ui.dl.rect(Rect::new(b.x - 8.0 + k, ty - 8.0 + k, 2.0, 16.0 - k * 2.0), Color::hex(0xF7F4EA), 0.0);
    }
    let ink = Color::hex(0x1F2A24);
    let ink_dim = Color::hex(0x5B6B62);
    let mut y = b.y + 14.0;
    let mut x = b.x + 16.0;
    if let Some((label, c)) = badge {
        let bw = ui.badge_at(x, y + 2.0, label, c);
        x += bw + 8.0;
    }
    for (i, line) in title_lines.iter().enumerate() {
        let lx = if i == 0 { x } else { b.x + 16.0 };
        ui.text_in(Rect::new(lx, y, text_w, tlh), line, t.font_heading, FontWeight::Bold, ink, Align::Left);
        y += tlh;
    }
    y += 6.0;

    // 本文（長ければスクロール）
    let footer_h = 44.0;
    let body_r = Rect::new(b.x + 16.0, y, text_w, (b.bottom() - footer_h - 8.0 - y).max(lh));
    ui.scroll_area(Id::new("secretary-body").with(title.len() as u64), body_r, 0.0, |ui| {
        for line in &lines {
            let lr = ui.alloc(lh);
            ui.text_in(lr, line, t.font_body, FontWeight::Regular, ink, Align::Left);
        }
    });

    // 下：ページとボタン
    let fy = b.bottom() - footer_h;
    let mut bx = b.right() - 12.0;
    let mut clicked = None;
    for (i, btn) in buttons.iter().enumerate().rev() {
        let bw = ui.measure(btn.label, t.font_body, FontWeight::Bold) + 28.0;
        let br = Rect::new(bx - bw, fy + 4.0, bw, 36.0);
        let fill = match btn.style {
            ButtonStyle::Primary => t.accent,
            _ => Color::TRANSPARENT,
        };
        let pressed = ui.pressed(br);
        let fill = if pressed { fill.lerp(Color::BLACK, 0.2) } else { fill };
        let border = if btn.style == ButtonStyle::Primary { t.accent } else { ink_dim };
        ui.dl.rect_border(br, fill, 18.0, 1.5, border);
        let tc = if btn.style == ButtonStyle::Primary { Color::hex(0x1A1208) } else { ink };
        ui.text_in(br, btn.label, t.font_body, FontWeight::Bold, tc, Align::Center);
        if ui.clicked(br) {
            clicked = Some(i);
        }
        bx -= bw + 8.0;
    }
    if let Some((cur, total)) = page {
        ui.text_in(
            Rect::new(b.x + 16.0, fy + 4.0, 80.0, 36.0),
            &format!("{} / {}", cur + 1, total),
            t.font_small,
            FontWeight::Bold,
            ink_dim,
            Align::Left,
        );
    }
    clicked
}

pub fn priority_badge(ui: &Ui, p: Priority) -> (&'static str, Color) {
    let t = ui.theme;
    match p {
        Priority::Urgent => (p.label(), t.negative),
        Priority::Important => (p.label(), Color::hex(0xD9822B)),
        Priority::Opportunity => (p.label(), Color::hex(0x3F8F5B)),
    }
}

pub enum Action {
    Close,
    Prev,
    Next,
    /// 関係する画面へ
    Goto,
}

/// 報告の一覧を 1 件ずつ表示する。
pub fn reports(ui: &mut Ui, area: Rect, items: &[Advice], index: usize) -> Option<Action> {
    if items.is_empty() {
        let clicked = window(
            ui,
            area,
            Portrait::secretary(Expression::Happy),
            None,
            "いまは特に問題はありません",
            "各教室も競合の動きも落ち着いています。気になることがあれば、すぐにお知らせしますね。",
            None,
            &[Button { label: "閉じる", style: ButtonStyle::Primary }],
        );
        return clicked.map(|_| Action::Close);
    }
    let i = index.min(items.len() - 1);
    let a = &items[i];
    let badge = priority_badge(ui, a.priority);
    let expression = match a.priority {
        Priority::Urgent => Expression::Worried,
        Priority::Important => Expression::Focused,
        Priority::Opportunity => Expression::Surprised,
    };
    let portrait = Portrait::secretary(expression);
    let has_prev = i > 0;
    let has_next = i + 1 < items.len();
    let mut buttons = Vec::new();
    let mut actions = Vec::new();
    if has_prev {
        buttons.push(Button { label: "前へ", style: ButtonStyle::Ghost });
        actions.push(Action::Prev);
    }
    buttons.push(Button { label: "見に行く", style: ButtonStyle::Secondary });
    actions.push(Action::Goto);
    if has_next {
        buttons.push(Button { label: "次へ", style: ButtonStyle::Primary });
        actions.push(Action::Next);
    } else {
        buttons.push(Button { label: "閉じる", style: ButtonStyle::Primary });
        actions.push(Action::Close);
    }
    let clicked = window(ui, area, portrait, Some(badge), &a.title, &a.body, Some((i, items.len())), &buttons);
    clicked.map(|k| std::mem::replace(&mut actions[k], Action::Close))
}
