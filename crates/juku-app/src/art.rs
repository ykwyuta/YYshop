//! 図形で描く背景・アイコン・装飾。

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
        Tab::Hq => {
            // 本部ビル
            dl.rect(Rect::new(cx - 9.0, cy - 12.0, 18.0, 25.0), c, 2.0);
            dl.rect(Rect::new(cx - 16.0, cy - 2.0, 7.0, 15.0), c, 1.5);
            dl.rect(Rect::new(cx + 9.0, cy - 2.0, 7.0, 15.0), c, 1.5);
            for k in 0..3 {
                let y = cy - 8.0 + k as f32 * 6.0;
                dl.rect(Rect::new(cx - 5.0, y, 3.5, 3.5), ui.theme.bar, 0.5);
                dl.rect(Rect::new(cx + 1.5, y, 3.5, 3.5), ui.theme.bar, 0.5);
            }
            dl.line(cx, cy - 12.0, cx, cy - 18.0, 1.5, c);
            dl.rect(Rect::new(cx, cy - 18.0, 6.0, 4.0), c, 0.5);
        }
    }
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
