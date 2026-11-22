//! グラフ（ドーナツ・棒・折れ線・レーダー）。

use std::f32::consts::TAU;

use juku_render::{Color, FontWeight, Rect};

use crate::{Align, Ui};

pub struct Series<'s> {
    pub values: &'s [f32],
    pub color: Color,
}

impl<'a> Ui<'a> {
    /// ドーナツグラフ。`segments` は (値, 色)。割合ラベルを描く。
    pub fn donut(&mut self, cx: f32, cy: f32, outer: f32, inner: f32, segments: &[(f32, Color)], labels: bool) {
        let t = self.theme;
        let total: f32 = segments.iter().map(|s| s.0).sum();
        if total <= 0.0 {
            self.dl.arc(cx, cy, outer, inner, 0.0, TAU, t.track);
            return;
        }
        let mut a = 0.0;
        for (v, c) in segments {
            let span = v / total * TAU;
            self.dl.arc(cx, cy, outer, inner, a, a + span, *c);
            if labels && *v / total >= 0.06 {
                let mid = a + span * 0.5;
                let rr = (outer + inner) * 0.5;
                let lx = cx + rr * mid.sin();
                let ly = cy - rr * mid.cos();
                let pct = format!("{:.0}%", v / total * 100.0);
                self.text_in(
                    Rect::new(lx - 30.0, ly - 10.0, 60.0, 20.0),
                    &pct,
                    t.font_small,
                    FontWeight::Bold,
                    Color::WHITE,
                    Align::Center,
                );
            }
            a += span;
        }
    }

    /// 2 系列の棒グラフ（売上・利益の週次推移など）。
    pub fn bar_chart(&mut self, r: Rect, labels: &[String], a: &[f32], b: &[f32], color_a: Color, color_b: Color) {
        let t = self.theme;
        let n = labels.len().max(1);
        let max = a.iter().chain(b.iter()).fold(1.0f32, |m, v| m.max(v.abs()));
        let label_h = 22.0;
        let plot_h = r.h - label_h;
        let slot = r.w / n as f32;
        let bar_w = (slot * 0.22).min(22.0);
        for i in 0..n {
            let x = r.x + slot * i as f32 + slot * 0.5;
            for (k, (vals, c)) in [(a, color_a), (b, color_b)].iter().enumerate() {
                let v = vals.get(i).copied().unwrap_or(0.0).max(0.0);
                let h = (v / max * plot_h).max(2.0);
                let bx = x - bar_w - 2.0 + k as f32 * (bar_w + 4.0);
                self.dl.rect(Rect::new(bx, r.y + plot_h - h, bar_w, h), *c, bar_w * 0.5);
            }
            let lr = Rect::new(x - slot * 0.5, r.y + plot_h + 2.0, slot, label_h);
            self.text_in(lr, &labels[i], t.font_small, FontWeight::Regular, t.text_dim, Align::Center);
        }
    }

    /// 折れ線グラフ。系列ごとに最大・最小を共有する。
    pub fn line_chart(&mut self, r: Rect, labels: &[String], series: &[Series], min_zero: bool) {
        let t = self.theme;
        let label_h = 22.0;
        let plot = Rect::new(r.x + 12.0, r.y + 10.0, r.w - 24.0, r.h - label_h - 16.0);
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for s in series {
            for v in s.values {
                lo = lo.min(*v);
                hi = hi.max(*v);
            }
        }
        if min_zero {
            lo = lo.min(0.0);
        }
        if lo == f32::MAX {
            return;
        }
        if (hi - lo).abs() < 1e-3 {
            hi = lo + 1.0;
        }
        let pad = (hi - lo) * 0.1;
        let (lo, hi) = (lo - if min_zero { 0.0 } else { pad }, hi + pad);
        for k in 0..=3 {
            let y = plot.y + plot.h * k as f32 / 3.0;
            self.dl.rect(Rect::new(plot.x, y, plot.w, 1.0), t.card_border.with_alpha(0.4), 0.0);
        }
        for s in series {
            let n = s.values.len();
            if n == 0 {
                continue;
            }
            let pts: Vec<(f32, f32)> = s
                .values
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let x = if n == 1 { plot.x + plot.w * 0.5 } else { plot.x + plot.w * i as f32 / (n - 1) as f32 };
                    let y = plot.y + plot.h * (1.0 - (v - lo) / (hi - lo));
                    (x, y)
                })
                .collect();
            for w in pts.windows(2) {
                self.dl.line(w[0].0, w[0].1, w[1].0, w[1].1, 3.0, s.color);
            }
            if n <= 12 {
                for (i, (x, y)) in pts.iter().enumerate() {
                    if i == n - 1 {
                        self.dl.circle(*x, *y, 9.0, s.color.with_alpha(0.3));
                    }
                    self.dl.circle(*x, *y, 5.5, s.color);
                }
            }
        }
        if !labels.is_empty() {
            let n = labels.len();
            for (i, l) in labels.iter().enumerate() {
                let x = if n == 1 { plot.x + plot.w * 0.5 } else { plot.x + plot.w * i as f32 / (n - 1) as f32 };
                let lr = Rect::new(x - 40.0, r.bottom() - label_h, 80.0, label_h);
                self.text_in(lr, l, t.font_small, FontWeight::Regular, t.text_dim, Align::Center);
            }
        }
    }

    /// レーダーチャート（5 軸の教材評価など）。値は 0..1。
    pub fn radar(&mut self, cx: f32, cy: f32, r: f32, values: &[f32], labels: &[&str], color: Color) {
        let t = self.theme;
        let n = values.len();
        if n < 3 {
            return;
        }
        let point = |i: usize, k: f32| {
            let a = TAU * i as f32 / n as f32;
            (cx + r * k * a.sin(), cy - r * k * a.cos())
        };
        for ring in 1..=4 {
            let k = ring as f32 / 4.0;
            for i in 0..n {
                let (x0, y0) = point(i, k);
                let (x1, y1) = point((i + 1) % n, k);
                self.dl.line(x0, y0, x1, y1, 1.0, t.card_border);
            }
        }
        // 塗り（中心からの太線で近似）
        for i in 0..n {
            let (x0, y0) = point(i, values[i].clamp(0.0, 1.0));
            let (x1, y1) = point((i + 1) % n, values[(i + 1) % n].clamp(0.0, 1.0));
            for s in 0..=8 {
                let k = s as f32 / 8.0;
                let px = x0 + (x1 - x0) * k;
                let py = y0 + (y1 - y0) * k;
                self.dl.line(cx, cy, px, py, 3.0, color.with_alpha(0.12));
            }
            self.dl.line(x0, y0, x1, y1, 2.5, color);
        }
        for i in 0..n {
            let (x, y) = point(i, values[i].clamp(0.0, 1.0));
            self.dl.circle(x, y, 4.0, color);
            let (lx, ly) = point(i, 1.28);
            let lr = Rect::new(lx - 45.0, ly - 11.0, 90.0, 22.0);
            self.text_in(
                lr,
                labels.get(i).copied().unwrap_or(""),
                t.font_small,
                FontWeight::Regular,
                t.text,
                Align::Center,
            );
        }
    }
}
