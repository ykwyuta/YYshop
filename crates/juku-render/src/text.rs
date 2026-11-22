//! フォントのラスタライズ・グリフアトラス・行組み（禁則処理つき）。

use std::collections::HashMap;

use crate::draw::{Color, DrawList, Rect};

pub const ATLAS_SIZE: u32 = 2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Regular,
    Bold,
}

#[derive(Clone, Copy, Debug)]
struct Glyph {
    /// アトラス上の位置（px）。幅 0 のグリフ（空白）は描かない。
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    xmin: f32,
    ymin: f32,
    advance: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    weight: FontWeight,
    ch: char,
    px: u16,
}

/// 棚詰め方式のアトラス。
struct Atlas {
    pixels: Vec<u8>,
    shelf_y: u32,
    shelf_h: u32,
    cursor_x: u32,
    dirty: bool,
}

impl Atlas {
    fn new() -> Self {
        Atlas { pixels: vec![0; (ATLAS_SIZE * ATLAS_SIZE) as usize], shelf_y: 1, shelf_h: 0, cursor_x: 1, dirty: true }
    }

    fn reset(&mut self) {
        self.pixels.fill(0);
        self.shelf_y = 1;
        self.shelf_h = 0;
        self.cursor_x = 1;
        self.dirty = true;
    }

    fn alloc(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.cursor_x + w + 1 > ATLAS_SIZE {
            self.shelf_y += self.shelf_h + 1;
            self.shelf_h = 0;
            self.cursor_x = 1;
        }
        if self.shelf_y + h + 1 > ATLAS_SIZE {
            return None;
        }
        let pos = (self.cursor_x, self.shelf_y);
        self.cursor_x += w + 1;
        self.shelf_h = self.shelf_h.max(h);
        Some(pos)
    }

    fn blit(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[u8]) {
        for row in 0..h {
            let dst = ((y + row) * ATLAS_SIZE + x) as usize;
            let src = (row * w) as usize;
            self.pixels[dst..dst + w as usize].copy_from_slice(&data[src..src + w as usize]);
        }
        self.dirty = true;
    }
}

pub struct TextSystem {
    regular: fontdue::Font,
    bold: fontdue::Font,
    cache: HashMap<GlyphKey, Glyph>,
    advance_cache: HashMap<GlyphKey, f32>,
    atlas: Atlas,
    scale: f32,
    /// アトラスが溢れてリセットされた（このフレームは描き直しが必要）
    overflowed: bool,
}

/// 行頭に来てはいけない文字。
const NO_LINE_START: &str = "、。，．・：；？！ー）」』】〕〉》｝）]}>,.!?％ぁぃぅぇぉっゃゅょァィゥェォッャュョ…";
/// 行末に来てはいけない文字。
const NO_LINE_END: &str = "（「『【〔〈《｛([{<¥＄";

impl TextSystem {
    pub fn new(regular: &[u8], bold: &[u8]) -> Result<Self, String> {
        let settings = fontdue::FontSettings { scale: 40.0, ..Default::default() };
        let regular = fontdue::Font::from_bytes(regular, settings)?;
        let bold = fontdue::Font::from_bytes(bold, settings)?;
        Ok(TextSystem {
            regular,
            bold,
            cache: HashMap::new(),
            advance_cache: HashMap::new(),
            atlas: Atlas::new(),
            scale: 1.0,
            overflowed: false,
        })
    }

    pub fn set_scale(&mut self, scale: f32) {
        if (scale - self.scale).abs() > f32::EPSILON {
            self.scale = scale;
            self.cache.clear();
            self.advance_cache.clear();
            self.atlas.reset();
        }
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// アトラスが溢れたか（溢れた場合、次のフレームを描き直す必要がある）。
    pub fn take_overflowed(&mut self) -> bool {
        std::mem::take(&mut self.overflowed)
    }

    pub(crate) fn atlas_pixels(&mut self) -> Option<&[u8]> {
        if self.atlas.dirty {
            self.atlas.dirty = false;
            Some(&self.atlas.pixels)
        } else {
            None
        }
    }

    fn font(&self, w: FontWeight) -> &fontdue::Font {
        match w {
            FontWeight::Regular => &self.regular,
            FontWeight::Bold => &self.bold,
        }
    }

    fn px(&self, size: f32) -> u16 {
        (size * self.scale).round().clamp(4.0, 400.0) as u16
    }

    /// 1 文字の送り幅（論理 px）。
    pub fn advance(&mut self, ch: char, size: f32, weight: FontWeight) -> f32 {
        let key = GlyphKey { weight, ch, px: self.px(size) };
        if let Some(a) = self.advance_cache.get(&key) {
            return *a;
        }
        let m = self.font(weight).metrics(ch, key.px as f32);
        let a = m.advance_width / self.scale;
        self.advance_cache.insert(key, a);
        a
    }

    pub fn measure(&mut self, text: &str, size: f32, weight: FontWeight) -> f32 {
        text.chars().map(|c| self.advance(c, size, weight)).sum()
    }

    pub fn line_height(&self, size: f32) -> f32 {
        size * 1.45
    }

    fn glyph(&mut self, ch: char, px: u16, weight: FontWeight) -> Glyph {
        let key = GlyphKey { weight, ch, px };
        if let Some(g) = self.cache.get(&key) {
            return *g;
        }
        let (m, bitmap) = self.font(weight).rasterize(ch, px as f32);
        let mut g = Glyph {
            x: 0,
            y: 0,
            w: m.width as u16,
            h: m.height as u16,
            xmin: m.xmin as f32,
            ymin: m.ymin as f32,
            advance: m.advance_width,
        };
        if m.width > 0 && m.height > 0 {
            match self.atlas.alloc(m.width as u32, m.height as u32) {
                Some((x, y)) => {
                    self.atlas.blit(x, y, m.width as u32, m.height as u32, &bitmap);
                    g.x = x as u16;
                    g.y = y as u16;
                }
                None => {
                    // 溢れたら全部捨てて作り直す（次フレームで描き直す）
                    self.atlas.reset();
                    self.cache.clear();
                    self.overflowed = true;
                    if let Some((x, y)) = self.atlas.alloc(m.width as u32, m.height as u32) {
                        self.atlas.blit(x, y, m.width as u32, m.height as u32, &bitmap);
                        g.x = x as u16;
                        g.y = y as u16;
                    }
                }
            }
        }
        self.cache.insert(key, g);
        g
    }

    /// 1 行を描く。`(x, y)` は行の左上。戻り値は描いた幅。
    pub fn draw(
        &mut self,
        dl: &mut DrawList,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        weight: FontWeight,
        color: Color,
    ) -> f32 {
        let px = self.px(size);
        let s = self.scale;
        let line_h = self.line_height(size);
        // 和文の仮想ボディ（1em）を行の中央に置き、ベースラインはその下端から 0.12em 上
        let baseline = y + (line_h - size) * 0.5 + size * 0.88;
        let baseline_px = (baseline * s).round();
        let mut pen = (x * s).round();
        let inv = 1.0 / ATLAS_SIZE as f32;
        for ch in text.chars() {
            let g = self.glyph(ch, px, weight);
            if g.w > 0 && g.h > 0 {
                let gx = pen + g.xmin;
                let gy = baseline_px - g.ymin - g.h as f32;
                let r = Rect::new(gx / s, gy / s, g.w as f32 / s, g.h as f32 / s);
                let uv = [g.x as f32 * inv, g.y as f32 * inv, (g.x + g.w) as f32 * inv, (g.y + g.h) as f32 * inv];
                dl.glyph(r, uv, color);
            }
            pen += g.advance;
        }
        pen / s - x
    }

    /// 幅 `max_width` で折り返す（禁則処理つき）。
    pub fn wrap(&mut self, text: &str, size: f32, weight: FontWeight, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for para in text.split('\n') {
            let chars: Vec<char> = para.chars().collect();
            let mut start = 0;
            while start < chars.len() {
                let mut w = 0.0;
                let mut end = start;
                while end < chars.len() {
                    let a = self.advance(chars[end], size, weight);
                    if w + a > max_width && end > start {
                        break;
                    }
                    w += a;
                    end += 1;
                }
                if end < chars.len() {
                    // 英単語の途中では折り返さない
                    let mut e = end;
                    while e > start + 1 && chars[e - 1].is_ascii_alphanumeric() && chars[e].is_ascii_alphanumeric() {
                        e -= 1;
                    }
                    if e > start + 1 {
                        end = e;
                    }
                    // 行頭禁則：次の行の先頭が禁則文字なら前の行に含める（ぶら下げ）
                    while end < chars.len() && NO_LINE_START.contains(chars[end]) {
                        end += 1;
                    }
                    // 行末禁則：行末が開き括弧なら次の行へ送る
                    while end > start + 1 && end < chars.len() && NO_LINE_END.contains(chars[end - 1]) {
                        end -= 1;
                    }
                }
                lines.push(chars[start..end].iter().collect::<String>().trim_start_matches(' ').to_string());
                start = end;
                while start < chars.len() && chars[start] == ' ' {
                    start += 1;
                }
            }
            if chars.is_empty() {
                lines.push(String::new());
            }
        }
        lines
    }
}
