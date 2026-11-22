//! 図形だけで描くイラスト（背景・アイコン・人物）。画像アセットは使わない。

use juku_render::{Color, Rect};
use juku_ui::Ui;

use crate::Tab;

/// 黒板のような背景（細い縦縞）。
pub fn chalkboard(ui: &mut Ui, r: Rect) {
    let t = ui.theme;
    ui.dl.rect(r, t.bg, 0.0);
    let stripe = 22.0;
    let n = (r.w / stripe) as i32 + 1;
    for i in 0..n {
        if i % 2 == 0 {
            ui.dl.rect(Rect::new(r.x + i as f32 * stripe, r.y, stripe, r.h), Color::WHITE.with_alpha(0.012), 0.0);
        }
    }
}

pub fn tab_icon(ui: &mut Ui, tab: Tab, r: Rect, c: Color) {
    let (cx, cy) = r.center();
    let dl = &mut *ui.dl;
    match tab {
        Tab::Classroom => {
            // 校舎
            dl.rect(Rect::new(cx - 13.0, cy - 3.0, 26.0, 16.0), c, 2.0);
            dl.line(cx - 16.0, cy - 2.0, cx, cy - 13.0, 3.0, c);
            dl.line(cx + 16.0, cy - 2.0, cx, cy - 13.0, 3.0, c);
            dl.rect(Rect::new(cx - 3.0, cy + 4.0, 6.0, 9.0), ui.theme.bar, 1.0);
        }
        Tab::Courses => {
            // 開いた本
            dl.rect(Rect::new(cx - 15.0, cy - 10.0, 14.0, 20.0), c, 3.0);
            dl.rect(Rect::new(cx + 1.0, cy - 10.0, 14.0, 20.0), c, 3.0);
            for k in 0..3 {
                let y = cy - 5.0 + k as f32 * 5.0;
                dl.line(cx - 12.0, y, cx - 4.0, y, 1.5, ui.theme.bar);
                dl.line(cx + 4.0, y, cx + 12.0, y, 1.5, ui.theme.bar);
            }
        }
        Tab::Marketing => {
            // メガホン
            dl.line(cx - 12.0, cy, cx + 10.0, cy - 9.0, 4.0, c);
            dl.line(cx - 12.0, cy, cx + 10.0, cy + 9.0, 4.0, c);
            dl.line(cx + 10.0, cy - 9.0, cx + 10.0, cy + 9.0, 4.0, c);
            dl.rect(Rect::new(cx - 15.0, cy - 4.0, 6.0, 8.0), c, 2.0);
            dl.line(cx - 6.0, cy + 4.0, cx - 3.0, cy + 12.0, 3.0, c);
        }
        Tab::Accounting => {
            for (k, h) in [10.0f32, 18.0, 14.0].iter().enumerate() {
                let x = cx - 13.0 + k as f32 * 10.0;
                dl.rect(Rect::new(x, cy + 11.0 - h, 7.0, *h), c, 2.0);
            }
            dl.rect(Rect::new(cx - 16.0, cy + 12.0, 32.0, 2.0), c, 1.0);
        }
        Tab::Year => {
            dl.rect_border(Rect::new(cx - 13.0, cy - 10.0, 26.0, 22.0), Color::TRANSPARENT, 4.0, 2.5, c);
            dl.rect(Rect::new(cx - 13.0, cy - 10.0, 26.0, 7.0), c, 3.0);
            for k in 0..3 {
                dl.circle(cx - 7.0 + k as f32 * 7.0, cy + 4.0, 2.0, c);
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Look {
    pub skin: Color,
    pub hair: Color,
    pub shirt: Color,
    pub accent: Color,
    pub long_hair: bool,
}

impl Look {
    /// 名前から見た目を決める（同じ名前なら同じ見た目）。
    pub fn from_name(name: &str) -> Look {
        let h = juku_ui::Id::new(name).0;
        let skins = [0xF1C9A5, 0xE0AC85, 0xC68863, 0xF5D5B8];
        let hairs = [0x2B1D14, 0x4A2E1C, 0x6B4423, 0x1C1C1C, 0x8A6A4A];
        let shirts = [0xF4F1E8, 0x3E5C8A, 0x6C8E6A, 0x8A4D5C, 0xD9D2C0];
        Look {
            skin: Color::hex(skins[(h % 4) as usize]),
            hair: Color::hex(hairs[((h >> 8) % 5) as usize]),
            shirt: Color::hex(shirts[((h >> 16) % 5) as usize]),
            accent: Color::hex(0xE39B3A),
            long_hair: (h >> 24).is_multiple_of(2),
        }
    }
}

/// 上半身の人物。
pub fn person(ui: &mut Ui, r: Rect, look: Look, tie: bool) {
    let dl = &mut *ui.dl;
    let cx = r.x + r.w * 0.5;
    let head_r = r.w.min(r.h) * 0.2;
    let head_cy = r.y + r.h * 0.36;
    // 後ろ髪
    if look.long_hair {
        dl.rect(Rect::new(cx - head_r * 1.1, head_cy - head_r * 0.6, head_r * 2.2, head_r * 2.2), look.hair, head_r);
    }
    // 体
    let body = Rect::new(cx - r.w * 0.36, r.y + r.h * 0.62, r.w * 0.72, r.h * 0.42);
    dl.rect(body, look.shirt, r.w * 0.18);
    // 首
    dl.rect(Rect::new(cx - head_r * 0.35, head_cy + head_r * 0.6, head_r * 0.7, head_r * 0.9), look.skin, 3.0);
    // 襟
    dl.line(cx - head_r * 0.5, r.y + r.h * 0.63, cx, r.y + r.h * 0.72, 3.0, Color::WHITE);
    dl.line(cx + head_r * 0.5, r.y + r.h * 0.63, cx, r.y + r.h * 0.72, 3.0, Color::WHITE);
    if tie {
        dl.line(cx, r.y + r.h * 0.7, cx, r.y + r.h * 0.9, 5.0, look.accent);
    }
    // 頭
    dl.circle(cx, head_cy, head_r, look.skin);
    // 前髪
    dl.arc(cx, head_cy, head_r * 1.08, head_r * 0.55, 4.6, std::f32::consts::TAU, look.hair);
    dl.arc(cx, head_cy, head_r * 1.08, head_r * 0.7, 0.0, 1.7, look.hair);
    // 目
    dl.circle(cx - head_r * 0.35, head_cy + head_r * 0.1, head_r * 0.09, Color::hex(0x2A2A2A));
    dl.circle(cx + head_r * 0.35, head_cy + head_r * 0.1, head_r * 0.09, Color::hex(0x2A2A2A));
    // 口
    dl.line(
        cx - head_r * 0.2,
        head_cy + head_r * 0.5,
        cx + head_r * 0.2,
        head_cy + head_r * 0.5,
        2.0,
        Color::hex(0xA0525A),
    );
}

/// 合格発表の桜。
pub fn sakura(ui: &mut Ui, cx: f32, cy: f32, r: f32) {
    let pink = Color::hex(0xF7B6C8);
    for i in 0..5 {
        let a = std::f32::consts::TAU * i as f32 / 5.0;
        ui.dl.circle(cx + r * 0.55 * a.sin(), cy - r * 0.55 * a.cos(), r * 0.48, pink);
    }
    ui.dl.circle(cx, cy, r * 0.25, Color::hex(0xF5D76E));
}
