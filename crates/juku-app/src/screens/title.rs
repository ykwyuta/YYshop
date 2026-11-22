//! スタート画面。夜明けの街並みに桜が舞う。保存データがあれば、その会社の様子を見せて「続きから」を勧める。

use juku_render::{Color, FontWeight, Rect};
use juku_ui::{Align, Ui};

use crate::art;

pub enum TitleAction {
    Continue,
    NewGame,
}

/// 保存データの要約（「続きから」のカードに出す）。
#[derive(Clone, Debug)]
pub struct SaveSummary {
    pub company: String,
    pub date: String,
    pub year_week: String,
    pub classrooms: usize,
    pub students: u32,
    pub cash: i64,
    pub listed: bool,
    pub bankrupt: bool,
}

impl SaveSummary {
    pub fn from_world(w: &juku_sim::World) -> Self {
        SaveSummary {
            company: w.company_name.clone(),
            date: w.calendar.date().label(),
            year_week: format!("{}年度 第{}週", w.calendar.school_year, w.calendar.week + 1),
            classrooms: w.classrooms.len(),
            students: w.total_students(),
            cash: w.cash,
            listed: w.listing.is_listed(),
            bankrupt: w.bankrupt,
        }
    }
}

/// 0..1 の擬似乱数（整数から決まる。毎フレーム同じ街並みを描くため）。
fn hash01(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t)
}

pub fn draw(ui: &mut Ui, r: Rect, save: Option<&SaveSummary>, first_time: bool) -> Option<TitleAction> {
    let t = ui.theme;
    let time = ui.input.time as f32;
    // 最初に開いた時刻からの経過（入場の演出用）。速さ 0 の animate は最初の値を覚えておくだけになる
    let start = ui.animate(juku_ui::Id::new("title-start"), time, 0.0);
    let since = (time - start).max(0.0);
    ui.mem.request_repaint();

    // ---- 空 ----
    let horizon = r.y + r.h * 0.62;
    ui.dl.rect_gradient(Rect::new(r.x, r.y, r.w, horizon - r.y), Color::hex(0x0E1A2B), Color::hex(0x2F5A6B), 0.0);
    ui.dl.rect_gradient(
        Rect::new(r.x, horizon - 120.0, r.w, 120.0),
        Color::hex(0x2F5A6B).with_alpha(0.0),
        Color::hex(0xE8A86B).with_alpha(0.55),
        0.0,
    );
    // 星
    for i in 0..40u32 {
        let x = r.x + hash01(i * 3) * r.w;
        let y = r.y + hash01(i * 3 + 1) * (horizon - r.y - 140.0);
        let tw = 0.4 + 0.6 * ((time * (1.0 + hash01(i) * 2.0) + i as f32).sin() * 0.5 + 0.5);
        ui.dl.circle(x, y, 1.0 + hash01(i * 7) * 1.2, Color::WHITE.with_alpha(0.25 + 0.55 * tw));
    }
    // 月
    ui.dl.circle(r.right() - 70.0, r.y + 110.0, 26.0, Color::hex(0xFFF4D6).with_alpha(0.9));
    ui.dl.circle(r.right() - 60.0, r.y + 104.0, 24.0, Color::hex(0x14243A).with_alpha(0.85));

    // ---- 街並み（奥と手前の 2 層）----
    for (layer, (base, color, win_alpha)) in
        [(horizon + 10.0, Color::hex(0x1F3A44), 0.35), (horizon + 70.0, Color::hex(0x132530), 0.9)].iter().enumerate()
    {
        let mut cursor = r.x - 10.0;
        let mut i = layer as u32 * 100;
        while cursor < r.right() {
            let full_w = 26.0 + hash01(i) * 38.0;
            let bh = 50.0 + hash01(i + 1) * if layer == 0 { 140.0 } else { 110.0 };
            // 画面（r）の外へはみ出さないよう切り詰める
            let x = cursor.max(r.x);
            let bw = (cursor + full_w).min(r.right()) - x;
            let b = Rect::new(x, base - bh, bw, r.bottom() - (base - bh));
            ui.dl.rect(b, *color, 2.0);
            // 窓の灯り
            let cols = ((bw - 8.0) / 9.0) as i32;
            let rows = ((bh - 10.0) / 12.0) as i32;
            for cy in 0..rows {
                for cx in 0..cols {
                    let k = i * 977 + (cy * 31 + cx) as u32;
                    if hash01(k) < 0.32 {
                        let flicker = if hash01(k + 5) < 0.04 {
                            ((time * 2.0 + k as f32).sin() * 0.5 + 0.5).round()
                        } else {
                            1.0
                        };
                        let wx = x + 5.0 + cx as f32 * 9.0;
                        let wy = base - bh + 8.0 + cy as f32 * 12.0;
                        ui.dl.rect(
                            Rect::new(wx, wy, 4.0, 6.0),
                            Color::hex(0xFFD98A).with_alpha(win_alpha * flicker),
                            1.0,
                        );
                    }
                }
            }
            cursor += full_w + 2.0 + hash01(i + 2) * 6.0;
            i += 3;
        }
    }
    // 地面
    ui.dl.rect_gradient(
        Rect::new(r.x, horizon + 60.0, r.w, r.bottom() - horizon - 60.0),
        Color::hex(0x132530),
        Color::hex(0x0B151B),
        0.0,
    );

    // ---- 桜の花びら ----
    for i in 0..26u32 {
        let speed = 18.0 + hash01(i + 50) * 22.0;
        let phase = hash01(i + 80) * 1000.0;
        let y = r.y + ((time * speed + phase) % (r.h + 40.0)) - 20.0;
        let sway = ((time * 0.8 + i as f32) * (0.6 + hash01(i + 90))).sin() * 26.0;
        let x = r.x + (hash01(i + 60) * r.w + sway).rem_euclid(r.w);
        let size = 3.0 + hash01(i + 70) * 3.0;
        let c = Color::hex(0xF7B6C8).with_alpha(0.55 + 0.35 * hash01(i + 40));
        ui.dl.circle(x, y, size, c);
        ui.dl.circle(x + size * 0.6, y + size * 0.4, size * 0.75, c);
    }

    // ---- ロゴ ----
    let logo_in = ease(since / 1.2);
    let lift = (1.0 - logo_in) * 24.0;
    let cx = r.x + r.w * 0.5;
    let card_h = if save.is_some() { 128.0 } else { 0.0 };
    let panel_h = card_h + 56.0 + 12.0 + 48.0 + 40.0;
    // 横向きのスマホなど低い画面では、余白を詰め、紋章→副題の順に省いて下のメニューと重ならないようにする
    let full_h = 224.0;
    let compact = r.h < full_h + panel_h + 64.0 + 24.0;
    let menu_top = r.bottom() - panel_h - if compact { 28.0 } else { 64.0 };
    let avail = menu_top - r.y - 8.0;
    let show_emblem = avail >= full_h + 24.0;
    let show_sub = avail >= full_h - 84.0;
    let logo_h = full_h - if show_emblem { 0.0 } else { 84.0 } - if show_sub { 0.0 } else { 20.0 };
    let top = (r.y + r.h * 0.12).min(menu_top - logo_h).max(r.y + 8.0) + lift;
    // 題字の位置を、紋章（92）と副題（20）がある前提の座標に合わせる
    let ly = top - if show_emblem { 0.0 } else { 84.0 } - if show_sub { 0.0 } else { 20.0 };
    if show_emblem {
        emblem(ui, cx, ly + 40.0, 40.0, logo_in);
    }
    if show_sub {
        ui.text_in(
            Rect::new(r.x, ly + 92.0, r.w, 22.0),
            "中学受験塾 経営シミュレーション",
            t.font_small + 1.0,
            FontWeight::Regular,
            Color::hex(0xCFE3DA).with_alpha(logo_in),
            Align::Center,
        );
    }
    // 影をつけた題字
    let title_r = Rect::new(r.x, ly + 112.0, r.w, 70.0);
    ui.text_in(
        title_r.translate(0.0, 3.0),
        "合格の街",
        54.0,
        FontWeight::Bold,
        Color::BLACK.with_alpha(0.45 * logo_in),
        Align::Center,
    );
    ui.text_in(title_r, "合格の街", 54.0, FontWeight::Bold, Color::hex(0xFFF8EC).with_alpha(logo_in), Align::Center);
    ui.text_in(
        Rect::new(r.x, ly + 182.0, r.w, 26.0),
        "〜 塾長シミュレーター 〜",
        t.font_heading,
        FontWeight::Bold,
        t.star.with_alpha(logo_in),
        Align::Center,
    );

    // ---- メニュー ----
    let menu_in = ease((since - 0.5) / 1.0);
    let panel_w = (r.w - 48.0).min(380.0);
    let px = cx - panel_w * 0.5;
    let py = menu_top + (1.0 - menu_in) * 30.0;
    let mut out = None;

    let mut y = py;
    if let Some(s) = save {
        let card = Rect::new(px, y, panel_w, card_h + 56.0);
        ui.dl.rect(card.translate(0.0, 4.0), Color::BLACK.with_alpha(0.3 * menu_in), 18.0);
        ui.dl.rect_border(
            card,
            Color::hex(0x0E1A22).with_alpha(0.86 * menu_in),
            18.0,
            1.5,
            t.accent.with_alpha(0.8 * menu_in),
        );
        let tx = card.x + 18.0;
        let tw = card.w - 36.0;
        ui.text_in(
            Rect::new(tx, y + 12.0, tw, 18.0),
            "前回のつづき",
            t.font_small,
            FontWeight::Regular,
            t.text_dim.with_alpha(menu_in),
            Align::Left,
        );
        ui.text_in(
            Rect::new(tx, y + 30.0, tw, 26.0),
            &s.company,
            t.font_heading + 2.0,
            FontWeight::Bold,
            t.text.with_alpha(menu_in),
            Align::Left,
        );
        if s.listed {
            let w = ui.measure(&s.company, t.font_heading + 2.0, FontWeight::Bold);
            ui.badge_at(tx + w + 8.0, y + 33.0, "上場", t.star);
        }
        ui.text_in(
            Rect::new(tx, y + 30.0, tw, 26.0),
            &s.date,
            t.font_small,
            FontWeight::Regular,
            t.text_dim.with_alpha(menu_in),
            Align::Right,
        );
        let stats = [
            ("教室", format!("{}", s.classrooms)),
            ("生徒", format!("{}人", s.students)),
            ("資金", crate::fmt::man(s.cash)),
        ];
        let cw = tw / 3.0;
        for (k, (label, value)) in stats.iter().enumerate() {
            let sx = tx + cw * k as f32;
            ui.text_in(
                Rect::new(sx, y + 62.0, cw, 18.0),
                label,
                t.font_small,
                FontWeight::Regular,
                t.text_dim.with_alpha(menu_in),
                Align::Left,
            );
            let c = if *label == "資金" && s.cash < 0 { t.negative } else { t.text };
            ui.text_in(
                Rect::new(sx, y + 80.0, cw, 28.0),
                value,
                t.font_heading + 2.0,
                FontWeight::Bold,
                c.with_alpha(menu_in),
                Align::Left,
            );
        }
        let label = if s.bankrupt { "倒産した会社を見る" } else { "続きから" };
        let b = Rect::new(tx, y + card_h - 6.0, tw, 48.0);
        if big_button(ui, b, label, true, menu_in) {
            out = Some(TitleAction::Continue);
        }
        ui.text_in(
            Rect::new(tx, y + card_h - 26.0, tw, 18.0),
            &s.year_week,
            t.font_small,
            FontWeight::Regular,
            t.text_dim.with_alpha(menu_in),
            Align::Right,
        );
        y += card_h + 56.0 + 12.0;
    }
    let b = Rect::new(px, y, panel_w, 48.0);
    if big_button(ui, b, "はじめから", save.is_none(), menu_in) {
        out = Some(TitleAction::NewGame);
    }
    let note = if first_time {
        "はじめての方には、秘書が画面をご案内します"
    } else {
        "小さな進学塾の塾長として、地域一番の塾を目指そう"
    };
    ui.text_in(
        Rect::new(px, y + 54.0, panel_w, 20.0),
        note,
        t.font_small,
        FontWeight::Regular,
        Color::hex(0xCFE3DA).with_alpha(menu_in),
        Align::Center,
    );

    // ---- 足もと ----
    ui.text_in(
        Rect::new(r.x, r.bottom() - 30.0, r.w, 20.0),
        &format!("ver {}  ・  © YYshop  ・  作中の塾名・学校名・人名はすべて架空です", env!("CARGO_PKG_VERSION")),
        10.0,
        FontWeight::Regular,
        Color::hex(0x8FA9A0).with_alpha(0.8 * menu_in),
        Align::Center,
    );
    out
}

/// 大きなボタン（光沢のある塗り、または縁取り）。
fn big_button(ui: &mut Ui, r: Rect, label: &str, primary: bool, alpha: f32) -> bool {
    let t = ui.theme;
    let pressed = ui.pressed(r);
    let hovered = ui.hovered(r);
    if primary {
        let top = if pressed { t.accent_dark } else { Color::hex(0xF2B04F) };
        let bottom = if pressed { t.accent_dark } else { Color::hex(0xD9822B) };
        ui.dl.rect(r.translate(0.0, 3.0), Color::BLACK.with_alpha(0.35 * alpha), 24.0);
        ui.dl.rect_gradient(r, top.with_alpha(alpha), bottom.with_alpha(alpha), 24.0);
        ui.text_in(r, label, t.font_heading, FontWeight::Bold, Color::hex(0x1A1208).with_alpha(alpha), Align::Center);
    } else {
        let fill = if pressed || hovered {
            Color::WHITE.with_alpha(0.12 * alpha)
        } else {
            Color::hex(0x0E1A22).with_alpha(0.6 * alpha)
        };
        ui.dl.rect_border(r, fill, 24.0, 1.5, Color::hex(0xF2B04F).with_alpha(alpha));
        ui.text_in(r, label, t.font_heading, FontWeight::Bold, Color::hex(0xFFE3B0).with_alpha(alpha), Align::Center);
    }
    ui.clicked(r)
}

/// アプリアイコンと同じモチーフの紋章（上り坂のビルと角帽、桜）。
fn emblem(ui: &mut Ui, cx: f32, cy: f32, size: f32, alpha: f32) {
    let s = size / 40.0;
    let bg = Rect::new(cx - size, cy - size, size * 2.0, size * 2.0);
    ui.dl.rect(bg.translate(0.0, 3.0), Color::BLACK.with_alpha(0.35 * alpha), size * 0.45);
    ui.dl.rect_gradient(
        bg,
        Color::hex(0x2F5A45).with_alpha(alpha),
        Color::hex(0x0C1511).with_alpha(alpha),
        size * 0.45,
    );
    let amber = Color::hex(0xF2B04F).with_alpha(alpha);
    for (k, h) in [16.0f32, 26.0, 38.0].iter().enumerate() {
        let x = cx - 26.0 * s + k as f32 * 18.0 * s;
        ui.dl.rect(Rect::new(x, cy + 24.0 * s - h * s, 14.0 * s, h * s), amber, 3.0 * s);
    }
    ui.dl.rect(Rect::new(cx - 30.0 * s, cy + 24.0 * s, 60.0 * s, 3.0 * s), Color::hex(0xF3F1E7).with_alpha(alpha), 1.5);
    // 角帽
    let capx = cx + 18.0 * s;
    let capy = cy - 20.0 * s;
    ui.dl.line(capx - 16.0 * s, capy, capx + 16.0 * s, capy - 4.0 * s, 7.0 * s, Color::hex(0xFFFFFF).with_alpha(alpha));
    ui.dl.rect(Rect::new(capx - 8.0 * s, capy, 14.0 * s, 6.0 * s), Color::hex(0xD9D4C3).with_alpha(alpha), 2.0);
    ui.dl.line(capx + 14.0 * s, capy - 3.0 * s, capx + 15.0 * s, capy + 10.0 * s, 1.5 * s, amber);
    art::sakura(ui, cx - 22.0 * s, cy - 20.0 * s, 8.0 * s);
}
