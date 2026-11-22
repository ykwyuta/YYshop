//! 基本ウィジェット。

use juku_render::{Color, FontWeight, Rect};

use crate::{Align, Id, Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonStyle {
    /// 塗りつぶし（主要な操作）
    Primary,
    /// 枠線のみ
    Secondary,
    /// 控えめ
    Ghost,
}

impl<'a> Ui<'a> {
    /// カード。中身を描いた後に背景を埋める。
    pub fn card<R>(&mut self, pad: f32, f: impl FnOnce(&mut Ui) -> R) -> R {
        let (x, y) = self.cursor();
        let w = self.width();
        let bg = self.dl.placeholder();
        let spacing = self.layout_spacing();
        self.begin(x + pad, y + pad, w - pad * 2.0, spacing);
        let r = f(self);
        let end = self.end();
        let h = end - y + pad;
        let t = self.theme;
        self.dl.fill_placeholder(bg, Rect::new(x, y, w, h), t.card, t.radius, 1.5, t.card_border);
        self.alloc(h);
        r
    }

    /// 全幅の帯（セクション見出し）。
    pub fn band(&mut self, title: &str, value: Option<&str>) {
        let t = self.theme;
        let r = self.alloc(48.0);
        self.dl.rect(r, t.band, 0.0);
        let inner = Rect::new(r.x + 18.0, r.y, r.w - 36.0, r.h);
        self.text_in(inner, title, t.font_heading, FontWeight::Bold, t.text, Align::Left);
        if let Some(v) = value {
            self.text_in(inner, v, t.font_heading, FontWeight::Bold, t.text, Align::Right);
        }
    }

    fn layout_spacing(&self) -> f32 {
        self.layouts.last().map(|l| l.spacing).unwrap_or(0.0)
    }

    pub fn separator(&mut self) {
        let t = self.theme;
        let r = self.alloc(1.0);
        self.dl.rect(r, t.card_border, 0.0);
    }

    /// 見出しラベルと値を左右に並べた 1 行。
    pub fn kv_row(&mut self, key: &str, value: &str, value_color: Option<Color>) {
        let t = self.theme;
        let lh = self.text.line_height(t.font_body);
        let r = self.alloc(lh + 4.0);
        self.text_in(r, key, t.font_body, FontWeight::Regular, t.text_dim, Align::Left);
        self.text_in(r, value, t.font_body, FontWeight::Bold, value_color.unwrap_or(t.text), Align::Right);
    }

    pub fn button(&mut self, label: &str, style: ButtonStyle) -> bool {
        let r = self.alloc(44.0);
        self.button_at(r, label, style)
    }

    pub fn button_at(&mut self, r: Rect, label: &str, style: ButtonStyle) -> bool {
        let t = self.theme;
        let pressed = self.pressed(r);
        let clicked = self.clicked(r);
        let (fill, border, text) = match style {
            ButtonStyle::Primary => (t.accent, t.accent, Color::hex(0x1A1208)),
            ButtonStyle::Secondary => (Color::TRANSPARENT, t.accent, t.accent),
            ButtonStyle::Ghost => (t.card, t.card_border, t.text),
        };
        let fill = if pressed { fill.lerp(Color::BLACK, 0.25) } else { fill };
        let fill = if pressed && fill.a == 0.0 { t.accent.with_alpha(0.15) } else { fill };
        self.dl.rect_border(r, fill, r.h.min(22.0), 1.5, border);
        self.text_in(r, label, t.font_body, FontWeight::Bold, text, Align::Center);
        clicked
    }

    /// トグルスイッチ。押されたら新しい値を返す。
    pub fn toggle_at(&mut self, id: Id, r: Rect, on: bool) -> Option<bool> {
        let t = self.theme;
        let clicked = self.clicked(r);
        let k = self.animate(id, if on { 1.0 } else { 0.0 }, 18.0);
        let track = Rect::new(r.x, r.y + (r.h - 30.0) * 0.5, 52.0, 30.0);
        let fill = t.track.lerp(t.accent_dark, k);
        let border = t.card_border.lerp(t.accent, k);
        self.dl.rect_border(track, fill, 15.0, 2.0, border);
        let knob_x = track.x + 4.0 + k * (track.w - 30.0);
        self.dl.rect(Rect::new(knob_x, track.y + 4.0, 22.0, 22.0), Color::WHITE.lerp(t.text_dim, 1.0 - k), 7.0);
        if clicked {
            Some(!on)
        } else {
            None
        }
    }

    /// ラベル付きトグル（1 行）。
    pub fn toggle_row(&mut self, id: Id, label: &str, on: bool) -> Option<bool> {
        let t = self.theme;
        let r = self.alloc(46.0);
        let res = self.toggle_at(id, r, on);
        let lr = Rect::new(r.x + 66.0, r.y, r.w - 66.0, r.h);
        self.text_in(lr, label, t.font_body, FontWeight::Regular, t.text, Align::Left);
        if res.is_none() && self.clicked(lr) {
            return Some(!on);
        }
        res
    }

    /// 「− 値 ＋」のステッパー。-1 / 0 / +1 を返す。長押しで連続。
    pub fn stepper_at(&mut self, id: Id, r: Rect, value: &str, sub: Option<&str>) -> i32 {
        let t = self.theme;
        let bw = r.h.min(56.0);
        self.dl.rect_border(r, t.accent_dark, 12.0, 2.0, t.accent);
        let mid = Rect::new(r.x + bw, r.y + 2.0, r.w - bw * 2.0, r.h - 4.0);
        self.dl.rect(mid, t.track, 0.0);
        let minus = Rect::new(r.x, r.y, bw, r.h);
        let plus = Rect::new(r.right() - bw, r.y, bw, r.h);
        let mut delta = 0;
        if self.repeat_clicked(id.with(1), minus) {
            delta = -1;
        }
        if self.repeat_clicked(id.with(2), plus) {
            delta = 1;
        }
        for (rr, sign) in [(minus, false), (plus, true)] {
            let (cx, cy) = rr.center();
            let c = if self.pressed(rr) { Color::WHITE } else { t.text };
            self.dl.line(cx - 9.0, cy, cx + 9.0, cy, 3.0, c);
            if sign {
                self.dl.line(cx, cy - 9.0, cx, cy + 9.0, 3.0, c);
            }
        }
        match sub {
            Some(s) => {
                let top = Rect::new(mid.x, mid.y + 2.0, mid.w, mid.h * 0.62);
                let bottom = Rect::new(mid.x, mid.y + mid.h * 0.58, mid.w, mid.h * 0.38);
                self.text_in(top, value, t.font_heading + 2.0, FontWeight::Bold, t.text, Align::Center);
                self.text_in(bottom, s, t.font_small - 1.0, FontWeight::Regular, t.text_dim, Align::Center);
            }
            None => self.text_in(mid, value, t.font_heading + 2.0, FontWeight::Bold, t.text, Align::Center),
        }
        delta
    }

    /// セグメントタブ（横並びの切り替え）。押されたら新しい添字。
    pub fn segmented(&mut self, labels: &[&str], selected: usize) -> Option<usize> {
        let t = self.theme;
        let r = self.alloc(48.0);
        self.dl.rect_border(r, t.accent_dark, 14.0, 2.0, t.accent);
        let w = r.w / labels.len() as f32;
        let mut out = None;
        for (i, label) in labels.iter().enumerate() {
            let cell = Rect::new(r.x + w * i as f32, r.y, w, r.h);
            if i == selected {
                self.dl.rect(cell.shrink(4.0), t.bar, 10.0);
            }
            let color = if i == selected { t.text } else { t.text.with_alpha(0.75) };
            self.text_in(cell, label, t.font_body, FontWeight::Bold, color, Align::Center);
            if self.clicked(cell) && i != selected {
                out = Some(i);
            }
        }
        out
    }

    /// 星評価（小数対応）。
    pub fn stars_at(&mut self, x: f32, y: f32, size: f32, value: f64) {
        let t = self.theme;
        for i in 0..5 {
            let cx = x + size * 0.5 + i as f32 * size * 1.1;
            let cy = y + size * 0.5;
            let fill = (value as f32 - i as f32).clamp(0.0, 1.0);
            star(self, cx, cy, size * 0.5, t.track.lerp(t.text_dim, 0.4), 1.0);
            if fill > 0.0 {
                star(self, cx, cy, size * 0.5, t.star, fill);
            }
        }
    }

    /// 目盛り付きの横棒（能力値）。
    pub fn meter_at(&mut self, r: Rect, value: f32, color: Color) {
        let t = self.theme;
        self.dl.rect(r, t.track, r.h * 0.5);
        let fw = (r.w - 4.0) * value.clamp(0.0, 1.0);
        if fw > 0.0 {
            self.dl.rect(Rect::new(r.x + 2.0, r.y + 2.0, fw, r.h - 4.0), color, (r.h - 4.0) * 0.5);
        }
        for i in 1..5 {
            let x = r.x + r.w * i as f32 / 5.0;
            self.dl.rect(Rect::new(x, r.y + 2.0, 1.0, r.h - 4.0), t.track.with_alpha(0.6), 0.0);
        }
    }

    /// ページ送りの点。
    pub fn page_dots(&mut self, count: usize, current: usize) {
        let t = self.theme;
        let r = self.alloc(16.0);
        let gap = 14.0;
        let total = gap * (count.saturating_sub(1)) as f32;
        let x0 = r.x + (r.w - total) * 0.5;
        for i in 0..count {
            let c = if i == current { t.text } else { t.text_dim.with_alpha(0.5) };
            self.dl.circle(x0 + gap * i as f32, r.y + 8.0, 4.0, c);
        }
    }

    /// 小さなバッジ（タグ）。幅を返す。
    pub fn badge_at(&mut self, x: f32, y: f32, label: &str, color: Color) -> f32 {
        let t = self.theme;
        let w = self.text.measure(label, t.font_small, FontWeight::Bold) + 14.0;
        let r = Rect::new(x, y, w, 22.0);
        self.dl.rect(r, color.with_alpha(0.2), 11.0);
        self.text_in(r, label, t.font_small, FontWeight::Bold, color, Align::Center);
        w
    }

    /// 画面全体を暗くして中央にパネルを出す。中身は `f` で描く。
    pub fn modal<R>(&mut self, width: f32, f: impl FnOnce(&mut Ui) -> R) -> R {
        let t = self.theme;
        let s = self.screen();
        self.dl.rect(s, Color::BLACK.with_alpha(0.6), 0.0);
        let w = width.min(s.w - 24.0);
        let x = s.x + (s.w - w) * 0.5;
        let y = s.y + s.h * 0.14;
        let bg = self.dl.placeholder();
        self.begin(x + 20.0, y + 20.0, w - 40.0, 10.0);
        let r = f(self);
        let end = self.end();
        let h = end - y + 20.0;
        self.dl.fill_placeholder(bg, Rect::new(x, y, w, h), t.card, 18.0, 2.0, t.accent);
        r
    }
}

/// 星。`fill` は左からの塗り割合。
fn star(ui: &mut Ui, cx: f32, cy: f32, r: f32, color: Color, fill: f32) {
    if fill >= 1.0 {
        ui.dl.star(cx, cy, r, color);
        return;
    }
    ui.dl.push_clip(Rect::new(cx - r, cy - r, r * 2.0 * fill, r * 2.0));
    ui.dl.star(cx, cy, r, color);
    ui.dl.pop_clip();
}
