//! 描画命令の蓄積（DrawList）と基本型。座標はすべて論理ピクセル。

use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
    }

    /// 0xRRGGBB
    pub const fn hex(v: u32) -> Color {
        Color::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    pub const fn with_alpha(self, a: f32) -> Color {
        Color { a, ..self }
    }

    pub fn lerp(self, o: Color, t: f32) -> Color {
        Color {
            r: self.r + (o.r - self.r) * t,
            g: self.g + (o.g - self.g) * t,
            b: self.b + (o.b - self.b) * t,
            a: self.a + (o.a - self.a) * t,
        }
    }

    fn arr(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }

    pub fn shrink(&self, d: f32) -> Rect {
        Rect::new(self.x + d, self.y + d, (self.w - 2.0 * d).max(0.0), (self.h - 2.0 * d).max(0.0))
    }

    pub fn intersect(&self, o: &Rect) -> Rect {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = self.right().min(o.right());
        let y1 = self.bottom().min(o.bottom());
        Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }

    pub fn translate(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Instance {
    pub rect: [f32; 4],
    pub color: [f32; 4],
    pub clip: [f32; 4],
    pub uv: [f32; 4],
    pub params: [f32; 4],
    pub params2: [f32; 4],
    pub color2: [f32; 4],
}

const KIND_RECT: f32 = 0.0;
pub(crate) const KIND_GLYPH: f32 = 1.0;
const KIND_ARC: f32 = 2.0;
const KIND_LINE: f32 = 3.0;
const KIND_STAR: f32 = 4.0;

const NO_CLIP: [f32; 4] = [-1.0e9, -1.0e9, 1.0e9, 1.0e9];

/// 1 フレーム分の描画命令。
#[derive(Default)]
pub struct DrawList {
    pub(crate) instances: Vec<Instance>,
    clip_stack: Vec<Rect>,
}

impl DrawList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.instances.clear();
        self.clip_stack.clear();
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub fn push_clip(&mut self, r: Rect) {
        let r = match self.clip_stack.last() {
            Some(top) => top.intersect(&r),
            None => r,
        };
        self.clip_stack.push(r);
    }

    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    pub fn current_clip(&self) -> Option<Rect> {
        self.clip_stack.last().copied()
    }

    fn clip(&self) -> [f32; 4] {
        match self.clip_stack.last() {
            Some(r) => [r.x, r.y, r.right(), r.bottom()],
            None => NO_CLIP,
        }
    }

    pub(crate) fn push(&mut self, mut inst: Instance) {
        inst.clip = self.clip();
        self.instances.push(inst);
    }

    /// 後から中身を差し替えるための場所取り（カードの背景など）。
    pub fn placeholder(&mut self) -> usize {
        self.push(Instance::zeroed());
        let i = self.instances.len() - 1;
        self.instances[i].color = [0.0; 4];
        i
    }

    /// `placeholder` で確保した位置に角丸矩形を入れる。
    pub fn fill_placeholder(
        &mut self,
        index: usize,
        r: Rect,
        fill: Color,
        radius: f32,
        border: f32,
        border_color: Color,
    ) {
        let clip = self.instances[index].clip;
        self.instances[index] = rect_instance(r, fill, radius, border, border_color, false);
        self.instances[index].clip = clip;
    }

    pub fn rect(&mut self, r: Rect, color: Color, radius: f32) {
        self.push(rect_instance(r, color, radius, 0.0, Color::TRANSPARENT, false));
    }

    pub fn rect_border(&mut self, r: Rect, fill: Color, radius: f32, border: f32, border_color: Color) {
        self.push(rect_instance(r, fill, radius, border, border_color, false));
    }

    /// 縦グラデーションの角丸矩形。
    pub fn rect_gradient(&mut self, r: Rect, top: Color, bottom: Color, radius: f32) {
        self.push(rect_instance(r, top, radius, 0.0, bottom, true));
    }

    pub fn circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        self.rect(Rect::new(cx - radius, cy - radius, radius * 2.0, radius * 2.0), color, radius);
    }

    /// 円弧（リング）。角度はラジアンで、0 = 真上、時計回り。
    pub fn arc(&mut self, cx: f32, cy: f32, outer: f32, inner: f32, start: f32, end: f32, color: Color) {
        if end <= start {
            return;
        }
        self.push(Instance {
            rect: [cx - outer, cy - outer, outer * 2.0, outer * 2.0],
            color: color.arr(),
            clip: NO_CLIP,
            uv: [0.0; 4],
            params: [KIND_ARC, 0.0, 0.0, 0.0],
            params2: [inner, start, end, 0.0],
            color2: [0.0; 4],
        });
    }

    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, color: Color) {
        let pad = width * 0.5 + 1.0;
        let minx = x0.min(x1) - pad;
        let miny = y0.min(y1) - pad;
        let maxx = x0.max(x1) + pad;
        let maxy = y0.max(y1) + pad;
        self.push(Instance {
            rect: [minx, miny, maxx - minx, maxy - miny],
            color: color.arr(),
            clip: NO_CLIP,
            uv: [0.0; 4],
            params: [KIND_LINE, width, 0.0, 0.0],
            params2: [x0, y0, x1, y1],
            color2: [0.0; 4],
        });
    }

    /// 五芒星。`r` は外接円の半径。
    pub fn star(&mut self, cx: f32, cy: f32, r: f32, color: Color) {
        self.push(Instance {
            rect: [cx - r, cy - r, r * 2.0, r * 2.0],
            color: color.arr(),
            clip: NO_CLIP,
            uv: [0.0; 4],
            params: [KIND_STAR, 0.0, 0.0, 0.0],
            params2: [0.45, 0.0, 0.0, 0.0],
            color2: [0.0; 4],
        });
    }

    pub(crate) fn glyph(&mut self, r: Rect, uv: [f32; 4], color: Color) {
        self.push(Instance {
            rect: [r.x, r.y, r.w, r.h],
            color: color.arr(),
            clip: NO_CLIP,
            uv,
            params: [KIND_GLYPH, 0.0, 0.0, 0.0],
            params2: [0.0; 4],
            color2: [0.0; 4],
        });
    }
}

fn rect_instance(r: Rect, fill: Color, radius: f32, border: f32, color2: Color, gradient: bool) -> Instance {
    Instance {
        rect: [r.x, r.y, r.w, r.h],
        color: fill.arr(),
        clip: NO_CLIP,
        uv: [0.0; 4],
        params: [KIND_RECT, radius, border, if gradient { 1.0 } else { 0.0 }],
        params2: [0.0; 4],
        color2: color2.arr(),
    }
}
