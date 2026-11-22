//! 画像（PNG）の読み込みと縮小。GPU に送る前の CPU 側の処理。

/// RGBA8 の画像。
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// PNG を RGBA8 に展開する。
pub fn decode_png(bytes: &[u8]) -> Result<RgbaImage, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| format!("png: {e}"))?;
    let mut pixels = vec![0; reader.output_buffer_size().ok_or("png too large")?];
    let info = reader.next_frame(&mut pixels).map_err(|e| format!("png: {e}"))?;
    pixels.truncate(info.buffer_size());
    let pixels = match info.color_type {
        png::ColorType::Rgba => pixels,
        png::ColorType::Rgb => pixels.chunks_exact(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => pixels.chunks_exact(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("png: palette not expanded".into()),
    };
    Ok(RgbaImage { width: info.width, height: info.height, pixels })
}

impl RgbaImage {
    /// 縦横比を保ったまま、`max_w`×`max_h` に収まるよう縮める（拡大はしない）。面積平均で縮める。
    pub fn fit_within(self, max_w: u32, max_h: u32) -> RgbaImage {
        let s = (max_w as f64 / self.width as f64).min(max_h as f64 / self.height as f64);
        if s >= 1.0 {
            return self;
        }
        let nw = ((self.width as f64 * s).round() as u32).max(1);
        let nh = ((self.height as f64 * s).round() as u32).max(1);
        self.resize(nw, nh)
    }

    /// 面積平均（ボックスフィルタ）で縮小する。
    pub fn resize(&self, nw: u32, nh: u32) -> RgbaImage {
        let (w, h) = (self.width as usize, self.height as usize);
        let (nw_u, nh_u) = (nw as usize, nh as usize);
        let mut out = vec![0u8; nw_u * nh_u * 4];
        for y in 0..nh_u {
            let y0 = y * h / nh_u;
            let y1 = ((y + 1) * h / nh_u).max(y0 + 1).min(h);
            for x in 0..nw_u {
                let x0 = x * w / nw_u;
                let x1 = ((x + 1) * w / nw_u).max(x0 + 1).min(w);
                let mut acc = [0u32; 4];
                for sy in y0..y1 {
                    let row = &self.pixels[(sy * w + x0) * 4..(sy * w + x1) * 4];
                    for px in row.chunks_exact(4) {
                        for k in 0..4 {
                            acc[k] += px[k] as u32;
                        }
                    }
                }
                let n = ((y1 - y0) * (x1 - x0)) as u32;
                let o = (y * nw_u + x) * 4;
                for k in 0..4 {
                    out[o + k] = ((acc[k] + n / 2) / n) as u8;
                }
            }
        }
        RgbaImage { width: nw, height: nh, pixels: out }
    }

    /// ミップマップ（半分ずつ縮めた画像の列。最初は元の画像）。
    pub fn mip_chain(&self) -> Vec<RgbaImage> {
        let mut levels = vec![RgbaImage { width: self.width, height: self.height, pixels: self.pixels.clone() }];
        loop {
            let last = levels.last().expect("level");
            if last.width == 1 && last.height == 1 {
                break;
            }
            let next = last.resize((last.width / 2).max(1), (last.height / 2).max(1));
            levels.push(next);
        }
        levels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_keeps_aspect_and_averages() {
        let img = RgbaImage { width: 4, height: 2, pixels: [[255u8, 0, 0, 255], [0, 0, 255, 255]].repeat(4).concat() };
        let small = img.fit_within(2, 2);
        assert_eq!((small.width, small.height), (2, 1));
        assert_eq!(&small.pixels[0..4], &[128, 0, 128, 255]);
        assert_eq!(small.mip_chain().len(), 2);
    }
}
