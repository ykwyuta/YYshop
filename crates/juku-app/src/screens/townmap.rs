//! タウンマップ：首都圏の実在の街を、緯度・経度どおりの地図の上で攻略していく。
//!
//! 海岸線・川・都県境・鉄道はおおまかな実際の形を緯度経度で持ち、画面の大きさに合わせて投影する。
//! 地図の背景画像（`images::MAP_FILE`）は `MAP_BOUNDS` の範囲を描いたもの。読み込めるまでは図形で描く。

use juku_render::{Color, FontWeight, Rect};
use juku_sim::World;
use juku_ui::{Align, ButtonStyle, ImageState, Ui};

use crate::fmt::format_yen;
use crate::images;
use crate::screens::hq::HqAction;

/// 地図に映す範囲の中心（経度・緯度）と、幅・高さ（km）。
const CENTER: (f64, f64) = (139.725, 35.685);
const SPAN_KM: (f64, f64) = (58.0, 55.0);
/// 地図の背景画像が描いている範囲（西端・東端の経度, 南端・北端の緯度）。
const MAP_BOUNDS: ((f64, f64), (f64, f64)) = ((139.3672, 140.0828), (35.43, 35.94));
/// 経度・緯度 1 度あたりの km（北緯 35.7 度）。
const KM_PER_LON: f64 = 90.4;
const KM_PER_LAT: f64 = 111.0;

/// 東京湾（経度, 緯度）。地図の外まで伸ばしてある。
const TOKYO_BAY: &[(f64, f64)] = &[
    (139.66, 35.15),
    (139.67, 35.28),
    (139.64, 35.34),
    (139.63, 35.40),
    (139.67, 35.42),
    (139.64, 35.465),
    (139.68, 35.49),
    (139.74, 35.52),
    (139.80, 35.545),
    (139.79, 35.565),
    (139.75, 35.59),
    (139.76, 35.62),
    (139.78, 35.635),
    (139.83, 35.63),
    (139.86, 35.635),
    (139.88, 35.625),
    (139.91, 35.64),
    (139.95, 35.66),
    (139.98, 35.685),
    (140.02, 35.665),
    (140.05, 35.64),
    (140.09, 35.605),
    (140.12, 35.57),
    (140.10, 35.52),
    (140.00, 35.45),
    (139.92, 35.38),
    (139.82, 35.30),
    (139.80, 35.15),
];

/// 川（名前, 河口から上流へ）。
const RIVERS: &[(&str, &[(f64, f64)])] = &[
    (
        "多摩川",
        &[
            (139.77, 35.54),
            (139.70, 35.565),
            (139.67, 35.58),
            (139.63, 35.607),
            (139.57, 35.625),
            (139.53, 35.645),
            (139.47, 35.657),
            (139.42, 35.68),
            (139.36, 35.70),
            (139.30, 35.73),
        ],
    ),
    (
        "荒川",
        &[
            (139.84, 35.645),
            (139.84, 35.70),
            (139.81, 35.75),
            (139.73, 35.785),
            (139.65, 35.825),
            (139.58, 35.88),
            (139.53, 35.94),
            (139.48, 36.00),
        ],
    ),
    (
        "江戸川",
        &[
            (139.90, 35.64),
            (139.89, 35.70),
            (139.88, 35.76),
            (139.88, 35.80),
            (139.90, 35.86),
            (139.88, 35.94),
            (139.83, 36.02),
        ],
    ),
];

/// 都県境（多摩川・江戸川と重なるところは川で代える）。
const BORDERS: &[&[(f64, f64)]] = &[
    // 東京・埼玉
    &[
        (139.20, 35.86),
        (139.35, 35.80),
        (139.45, 35.775),
        (139.55, 35.78),
        (139.63, 35.795),
        (139.72, 35.80),
        (139.80, 35.79),
        (139.88, 35.78),
    ],
    // 東京（町田）・神奈川
    &[
        (139.53, 35.64),
        (139.50, 35.61),
        (139.48, 35.575),
        (139.45, 35.555),
        (139.41, 35.55),
        (139.34, 35.60),
        (139.26, 35.62),
    ],
];

/// 都県・海の名前（経度, 緯度, 名前）。
const PLACE_LABELS: &[(f64, f64, &str)] = &[
    (139.52, 35.74, "東京都"),
    (139.50, 35.90, "埼玉県"),
    (139.50, 35.50, "神奈川県"),
    (140.00, 35.80, "千葉県"),
    (139.90, 35.52, "東京湾"),
];

/// 鉄道（名前, 色, 駅の経度緯度）。
const RAILWAYS: &[(&str, u32, &[(f64, f64)])] = &[
    (
        "山手線",
        0x9ACD32,
        &[
            (139.767, 35.681),
            (139.777, 35.714),
            (139.761, 35.738),
            (139.711, 35.730),
            (139.700, 35.690),
            (139.702, 35.658),
            (139.739, 35.628),
            (139.757, 35.655),
            (139.767, 35.681),
        ],
    ),
    (
        "中央線",
        0xF15A22,
        &[
            (139.767, 35.681),
            (139.700, 35.690),
            (139.620, 35.704),
            (139.580, 35.703),
            (139.480, 35.700),
            (139.414, 35.698),
            (139.339, 35.656),
        ],
    ),
    (
        "京王線",
        0xDD0077,
        &[
            (139.700, 35.690),
            (139.650, 35.668),
            (139.541, 35.652),
            (139.480, 35.672),
            (139.447, 35.650),
            (139.343, 35.657),
        ],
    ),
    (
        "小田急線",
        0x0D6AB8,
        &[
            (139.700, 35.690),
            (139.668, 35.661),
            (139.599, 35.640),
            (139.570, 35.620),
            (139.508, 35.604),
            (139.446, 35.542),
            (139.438, 35.532),
        ],
    ),
    (
        "東急田園都市線",
        0x20A288,
        &[
            (139.702, 35.658),
            (139.670, 35.644),
            (139.627, 35.611),
            (139.611, 35.600),
            (139.558, 35.578),
            (139.495, 35.531),
        ],
    ),
    (
        "東急東横線",
        0xDA0442,
        &[(139.702, 35.658), (139.669, 35.607), (139.660, 35.576), (139.631, 35.509), (139.622, 35.466)],
    ),
    (
        "京浜東北線",
        0x00B2E5,
        &[
            (139.622, 35.466),
            (139.697, 35.531),
            (139.739, 35.628),
            (139.767, 35.681),
            (139.777, 35.714),
            (139.721, 35.778),
            (139.657, 35.859),
            (139.624, 35.906),
        ],
    ),
    ("丸ノ内線", 0xF62E36, &[(139.767, 35.681), (139.752, 35.701), (139.737, 35.717), (139.711, 35.730)]),
    ("有楽町線", 0xC1A470, &[(139.711, 35.730), (139.763, 35.675), (139.796, 35.655), (139.827, 35.646)]),
    ("西武池袋線", 0xF5A300, &[(139.711, 35.730), (139.654, 35.738), (139.584, 35.751), (139.473, 35.786)]),
    (
        "総武線",
        0xFFD400,
        &[(139.767, 35.681), (139.814, 35.697), (139.907, 35.728), (139.985, 35.702), (140.113, 35.613)],
    ),
    ("常磐線", 0x00B261, &[(139.777, 35.714), (139.805, 35.749), (139.900, 35.784), (139.971, 35.862)]),
    (
        "南武線",
        0xE8B800,
        &[(139.697, 35.531), (139.660, 35.576), (139.570, 35.620), (139.477, 35.666), (139.414, 35.698)],
    ),
];

/// 遊べない主要駅（目印）。
const LANDMARKS: &[(f64, f64, &str)] = &[
    (139.700, 35.690, "新宿"),
    (139.711, 35.730, "池袋"),
    (139.767, 35.681, "東京"),
    (139.777, 35.714, "上野"),
    (139.739, 35.628, "品川"),
    (139.697, 35.531, "川崎"),
    (140.113, 35.613, "千葉"),
    (139.414, 35.698, "立川"),
    (139.446, 35.542, "町田"),
];

/// 地図の投影：経度緯度 → 画面座標。
#[derive(Clone, Copy)]
struct Projection {
    m: Rect,
    px_per_km: f64,
}

impl Projection {
    fn new(m: Rect) -> Self {
        let px_per_km = (m.w as f64 / SPAN_KM.0).min(m.h as f64 / SPAN_KM.1);
        Self { m, px_per_km }
    }

    fn pt(&self, (lon, lat): (f64, f64)) -> (f32, f32) {
        let (cx, cy) = self.m.center();
        let x = (lon - CENTER.0) * KM_PER_LON * self.px_per_km;
        let y = (CENTER.1 - lat) * KM_PER_LAT * self.px_per_km;
        (cx + x as f32, cy + y as f32)
    }

    fn km(&self, km: f64) -> f32 {
        (km * self.px_per_km) as f32
    }
}

/// 多角形を走査線で塗る（描画ライブラリに多角形が無いので、細い横長の矩形を並べる）。
fn fill_polygon(ui: &mut Ui, proj: &Projection, poly: &[(f64, f64)], color: Color) {
    let m = proj.m;
    let pts: Vec<(f32, f32)> = poly.iter().map(|p| proj.pt(*p)).collect();
    let step = 2.0;
    let mut y = m.y;
    while y < m.bottom() {
        let sy = y + step * 0.5;
        let mut xs: Vec<f32> = Vec::new();
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            if (a.1 <= sy && b.1 > sy) || (b.1 <= sy && a.1 > sy) {
                xs.push(a.0 + (sy - a.1) / (b.1 - a.1) * (b.0 - a.0));
            }
        }
        xs.sort_by(f32::total_cmp);
        for pair in xs.chunks(2) {
            if let [x0, x1] = pair {
                let (x0, x1) = (x0.max(m.x), x1.min(m.right()));
                if x1 > x0 {
                    ui.dl.rect(Rect::new(x0, y, x1 - x0, step + 0.5), color, 0.0);
                }
            }
        }
        y += step;
    }
}

fn polyline(ui: &mut Ui, proj: &Projection, pts: &[(f64, f64)], width: f32, color: Color) {
    for w in pts.windows(2) {
        let (p, q) = (proj.pt(w[0]), proj.pt(w[1]));
        ui.dl.line(p.0, p.1, q.0, q.1, width, color);
    }
}

fn dashed(ui: &mut Ui, proj: &Projection, pts: &[(f64, f64)], width: f32, color: Color) {
    for w in pts.windows(2) {
        let (p, q) = (proj.pt(w[0]), proj.pt(w[1]));
        let len = ((q.0 - p.0).powi(2) + (q.1 - p.1).powi(2)).sqrt();
        let n = (len / 8.0).max(1.0) as usize;
        for k in (0..n).step_by(2) {
            let (t0, t1) = (k as f32 / n as f32, ((k + 1) as f32 / n as f32).min(1.0));
            ui.dl.line(
                p.0 + (q.0 - p.0) * t0,
                p.1 + (q.1 - p.1) * t0,
                p.0 + (q.0 - p.0) * t1,
                p.1 + (q.1 - p.1) * t1,
                width,
                color,
            );
        }
    }
}

/// 街の地図上の位置（経度, 緯度）。古い街の定義で緯度経度が無いときは、範囲の中に円く並べる。
fn town_lonlat(world: &World, a: usize) -> (f64, f64) {
    let area = &world.areas[a];
    if area.lat != 0.0 {
        return (area.lon, area.lat);
    }
    let n = world.areas.len().max(1);
    let ang = a as f64 / n as f64 * std::f64::consts::TAU;
    (CENTER.0 + 0.25 * ang.sin(), CENTER.1 + 0.2 * ang.cos())
}

/// エリアの状態。
#[derive(Clone, Copy, PartialEq)]
enum State {
    /// 自塾の教室がある（教室の添字）
    Ours(usize),
    /// 開校できる
    Open,
    /// まだ出店できない
    Locked,
}

fn state(world: &World, a: usize) -> State {
    if let Some(ci) = world.classrooms.iter().position(|c| c.area == a) {
        State::Ours(ci)
    } else if world.area_unlocked(a) {
        State::Open
    } else {
        State::Locked
    }
}

/// 攻略状況、地図、選んだ街の詳細を描く。
pub fn draw(ui: &mut Ui, world: &World, selected: &mut Option<usize>, action: &mut Option<HqAction>) {
    let t = ui.theme;
    let n = world.areas.len();
    if n == 0 {
        return;
    }
    // 最初は開校できる街のうち市場の大きいところ、なければ創業の地を選んでおく
    let sel = selected
        .filter(|a| *a < n)
        .or_else(|| world.open_areas().into_iter().max_by(|a, b| world.areas[*a].pool.total_cmp(&world.areas[*b].pool)))
        .unwrap_or(world.classrooms[0].area);
    *selected = Some(sel);

    ui.heading("首都圏攻略マップ");
    progress(ui, world);
    ui.caption("街をタップすると、市場の大きさ・家賃・競合が見られます。地域で在籍数 1 位になり、シェア 35% と在籍 30 人を超えるとその街を「制覇」です。");

    let mut clicked = None;
    ui.card(10.0, |ui| {
        let width = ui.width();
        let m = ui.alloc((width * 0.95).clamp(320.0, 680.0));
        let proj = Projection::new(m);

        // ---- 陸・海・川・都県境 ----
        ui.dl.rect_gradient(m, Color::hex(0x31503F), Color::hex(0x263F32), 10.0);
        ui.dl.push_clip(m);
        if let ImageState::Ready(info) = ui.image(images::MAP_FILE) {
            // 背景画像を緯度経度の範囲どおりに置く（はみ出す分は地図の枠で切る）
            let ((lon0, lon1), (lat0, lat1)) = MAP_BOUNDS;
            let (x0, y0) = proj.pt((lon0, lat1));
            let (x1, y1) = proj.pt((lon1, lat0));
            ui.dl.image(Rect::new(x0, y0, x1 - x0, y1 - y0), info.id, [0.0, 0.0, 1.0, 1.0], 0.0, 1.0);
        } else {
            // 23 区のあたりは街が詰まっている
            let (kx, ky) = proj.pt((139.72, 35.69));
            ui.dl.circle(kx, ky, proj.km(13.0), Color::WHITE.with_alpha(0.035));
            ui.dl.circle(kx, ky, proj.km(8.0), Color::WHITE.with_alpha(0.03));
            fill_polygon(ui, &proj, TOKYO_BAY, Color::hex(0x1D3C55));
            polyline(ui, &proj, TOKYO_BAY, 1.5, Color::hex(0x5B88A8).with_alpha(0.6));
            for (_, pts) in RIVERS {
                polyline(ui, &proj, pts, (proj.km(0.35)).clamp(2.0, 4.0), Color::hex(0x3B6E8C));
            }
            for pts in BORDERS {
                dashed(ui, &proj, pts, 1.2, Color::WHITE.with_alpha(0.22));
            }
        }
        for (lon, lat, name) in PLACE_LABELS {
            let (x, y) = proj.pt((*lon, *lat));
            ui.text_in(
                Rect::new(x - 50.0, y - 9.0, 100.0, 18.0),
                name,
                t.font_small,
                FontWeight::Bold,
                Color::WHITE.with_alpha(0.22),
                Align::Center,
            );
        }
        for (name, pts) in RIVERS {
            let (x, y) = proj.pt(pts[pts.len() / 2]);
            ui.text_in(
                Rect::new(x + 4.0, y - 14.0, 60.0, 14.0),
                name,
                t.font_small - 3.0,
                FontWeight::Regular,
                Color::hex(0x7FB2D0).with_alpha(0.7),
                Align::Left,
            );
        }

        // ---- 鉄道 ----
        for (_, color, pts) in RAILWAYS {
            polyline(ui, &proj, pts, 3.5, Color::hex(0x101A15).with_alpha(0.6));
            polyline(ui, &proj, pts, 1.6, Color::hex(*color).with_alpha(0.55));
        }
        for (lon, lat, name) in LANDMARKS {
            let (x, y) = proj.pt((*lon, *lat));
            ui.dl.rect(Rect::new(x - 3.0, y - 3.0, 6.0, 6.0), Color::hex(0xDDD8C4).with_alpha(0.7), 1.0);
            ui.text_in(
                Rect::new(x + 5.0, y - 7.0, 40.0, 14.0),
                name,
                t.font_small - 3.0,
                FontWeight::Regular,
                Color::WHITE.with_alpha(0.45),
                Align::Left,
            );
        }

        // ---- 街 ----
        let time = ui.input.time as f32;
        let pos: Vec<(f32, f32)> = (0..n).map(|a| proj.pt(town_lonlat(world, a))).collect();
        for a in 0..n {
            let area = &world.areas[a];
            let (cx, cy) = pos[a];
            let st = state(world, a);
            let r = 8.0 + (area.pool as f32 - 0.6).clamp(0.0, 1.4) * 5.0;
            let is_sel = a == sel;

            if is_sel {
                let pulse = 0.5 + 0.5 * (time * 3.0).sin();
                ui.dl.circle(cx, cy, r + 8.0 + pulse * 2.0, t.star.with_alpha(0.25));
                ui.mem.request_repaint();
            }
            match st {
                State::Ours(_) => {
                    let conquered = world.area_conquered(a);
                    if conquered {
                        // 制覇した街は金の縁と旗
                        ui.dl.circle(cx, cy, r + 4.0, t.star);
                    }
                    ui.dl.circle(cx, cy, r + 2.0, Color::hex(0x101A15));
                    ui.dl.circle(cx, cy, r, t.accent);
                    ui.text_in(
                        Rect::new(cx - r, cy - r, r * 2.0, r * 2.0),
                        "塾",
                        (r * 1.1).min(16.0),
                        FontWeight::Bold,
                        Color::hex(0x1A1208),
                        Align::Center,
                    );
                    if conquered {
                        flag(ui, cx + r * 0.6, cy - r - 2.0, t.star);
                    }
                }
                State::Open => {
                    ui.dl.circle(cx, cy, r + 2.0, Color::WHITE.with_alpha(0.9));
                    ui.dl.circle(cx, cy, r - 1.0, Color::hex(0x1F3329));
                    ui.text_in(
                        Rect::new(cx - r, cy - r, r * 2.0, r * 2.0),
                        "＋",
                        (r * 1.0).min(14.0),
                        FontWeight::Bold,
                        Color::WHITE.with_alpha(0.9),
                        Align::Center,
                    );
                }
                State::Locked => {
                    ui.dl.circle(cx, cy, r, Color::hex(0x3A4540).with_alpha(0.85));
                    padlock(ui, cx, cy, r * 0.5, Color::WHITE.with_alpha(0.5));
                }
            }

            // 競合の教室（小さな点。売りに出ている塾は赤い縁）
            let rivals = world.competitors.iter().filter(|c| !c.acquired && c.branch(a).is_some());
            for (k, comp) in rivals.enumerate() {
                let ang = -0.9 + k as f32 * 0.5;
                let (dx, dy) = ((r + 6.0) * ang.cos(), (r + 6.0) * ang.sin());
                let c = Color::rgb(comp.color[0], comp.color[1], comp.color[2]);
                if comp.for_sale {
                    ui.dl.circle(cx + dx, cy + dy, 5.0, t.negative);
                }
                ui.dl.circle(cx + dx, cy + dy, 3.5, c);
            }

            // 名前
            let name_c = if st == State::Locked { Color::WHITE.with_alpha(0.5) } else { Color::WHITE };
            let tw = ui.measure(&area.name, 10.5, FontWeight::Bold) + 10.0;
            let label_r = match label_side(&area.name) {
                LabelSide::Right => Rect::new(cx + r + 4.0, cy - 7.5, tw, 15.0),
                LabelSide::Above => Rect::new(cx - tw * 0.5, cy - r - 18.0, tw, 15.0),
                LabelSide::Below => Rect::new(cx - tw * 0.5, cy + r + 3.0, tw, 15.0),
            };
            ui.dl.rect(label_r, Color::BLACK.with_alpha(0.5), 7.5);
            ui.text_in(label_r, &area.name, 10.5, FontWeight::Bold, name_c, Align::Center);

            // タップの当たり判定（名前まで含める）
            let hit = Rect::new(cx - r - 8.0, cy - r - 8.0, (r + 8.0) * 2.0, (r + 8.0) * 2.0 + 10.0);
            if ui.clicked(hit) {
                clicked = Some(a);
            }
        }

        // 縮尺（10 km）と方位
        let bar = proj.km(10.0);
        let (bx, by) = (m.x + 12.0, m.bottom() - 16.0);
        ui.dl.line(bx, by, bx + bar, by, 2.0, Color::WHITE.with_alpha(0.6));
        ui.dl.line(bx, by - 4.0, bx, by + 1.0, 2.0, Color::WHITE.with_alpha(0.6));
        ui.dl.line(bx + bar, by - 4.0, bx + bar, by + 1.0, 2.0, Color::WHITE.with_alpha(0.6));
        ui.text_in(
            Rect::new(bx, by - 18.0, bar, 14.0),
            "10 km",
            t.font_small - 3.0,
            FontWeight::Regular,
            Color::WHITE.with_alpha(0.6),
            Align::Center,
        );
        let (nx, ny) = (m.right() - 22.0, m.y + 26.0);
        ui.dl.line(nx, ny + 8.0, nx, ny - 8.0, 2.0, Color::WHITE.with_alpha(0.6));
        ui.dl.line(nx - 5.0, ny - 3.0, nx, ny - 9.0, 2.0, Color::WHITE.with_alpha(0.6));
        ui.dl.line(nx + 5.0, ny - 3.0, nx, ny - 9.0, 2.0, Color::WHITE.with_alpha(0.6));
        ui.text_in(
            Rect::new(nx - 10.0, ny + 8.0, 20.0, 14.0),
            "N",
            t.font_small - 3.0,
            FontWeight::Bold,
            Color::WHITE.with_alpha(0.6),
            Align::Center,
        );
        ui.dl.pop_clip();

        // 凡例
        let r = ui.alloc(20.0);
        let items: [(Color, &str); 5] = [
            (t.star, "制覇"),
            (t.accent, "自塾"),
            (Color::WHITE, "開校できる"),
            (Color::hex(0x5A6560), "条件あり"),
            (t.negative, "売却中の塾"),
        ];
        let mut x = r.x;
        for (c, label) in items {
            ui.dl.circle(x + 6.0, r.y + 10.0, 5.0, c);
            ui.text_in(
                Rect::new(x + 15.0, r.y, 100.0, 20.0),
                label,
                t.font_small - 1.0,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
            x += 22.0 + ui.measure(label, t.font_small - 1.0, FontWeight::Regular) + 8.0;
        }
    });
    if let Some(a) = clicked {
        *selected = Some(a);
    }
    let sel = selected.unwrap_or(sel);
    detail(ui, world, sel, action);
}

/// 攻略の進み具合（全体と地方ごと）。
fn progress(ui: &mut Ui, world: &World) {
    let t = ui.theme;
    let (won, total) = world.conquest_progress();
    ui.card(12.0, |ui| {
        ui.set_spacing(6.0);
        let r = ui.alloc(24.0);
        ui.text_in(r, "首都圏の制覇", t.font_body, FontWeight::Bold, t.text, Align::Left);
        ui.text_in(r, &format!("{won} / {total} 街"), t.font_heading, FontWeight::Bold, t.star, Align::Right);
        let bar = ui.alloc(8.0);
        ui.meter_at(bar, won as f32 / total.max(1) as f32, t.star);
        let regions = world.region_progress();
        let r = ui.alloc(22.0);
        let mut x = r.x;
        for (name, won, total) in regions {
            let label = format!("{name} {won}/{total}");
            let color = if won == total {
                t.star
            } else if won > 0 {
                t.accent
            } else {
                t.text_dim
            };
            let w = ui.measure(&label, t.font_small - 1.0, FontWeight::Bold) + 16.0;
            if x + w > r.right() {
                break;
            }
            x += ui.badge_at(x, r.y + 2.0, &label, color) + 6.0;
        }
    });
}

enum LabelSide {
    Below,
    Right,
    Above,
}

/// 街が詰まっているところは、名前の位置をずらして重ならないようにする。
fn label_side(name: &str) -> LabelSide {
    match name {
        "成城学園前" => LabelSide::Above,
        "二子玉川" => LabelSide::Right,
        _ => LabelSide::Below,
    }
}

/// 制覇の旗。
fn flag(ui: &mut Ui, x: f32, y: f32, c: Color) {
    ui.dl.line(x, y, x, y - 12.0, 1.5, Color::WHITE.with_alpha(0.9));
    ui.dl.rect(Rect::new(x, y - 12.0, 8.0, 5.5), c, 1.0);
}

/// 錠前の形（鍵のかかったエリア）。
fn padlock(ui: &mut Ui, cx: f32, cy: f32, s: f32, c: Color) {
    ui.dl.arc(cx, cy - s * 0.25, s * 0.62, s * 0.38, -std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2, c);
    ui.dl.rect(Rect::new(cx - s * 0.75, cy - s * 0.25, s * 1.5, s * 1.15), c, s * 0.2);
}

/// 選んだエリアの詳細カード。
fn detail(ui: &mut Ui, world: &World, a: usize, action: &mut Option<HqAction>) {
    let t = ui.theme;
    let area = &world.areas[a];
    let st = state(world, a);
    let shares = world.area_shares(a);
    let total: u32 = shares.iter().map(|s| s.1).sum();
    ui.card(16.0, |ui| {
        ui.set_spacing(6.0);
        // 街の風景（未生成）
        let w = ui.width();
        let img = ui.alloc((w * 9.0 / 16.0).min(200.0));
        images::picture(ui, img, &format!("{}の街並み", area.name), &images::town_file(&area.name), t.series[4]);
        let r = ui.alloc(26.0);
        let title = area.name.clone();
        ui.text_in(r, &title, t.font_heading, FontWeight::Bold, t.text, Align::Left);
        let w = ui.measure(&title, t.font_heading, FontWeight::Bold);
        let (badge, color) = match st {
            State::Ours(_) => ("自塾あり", t.accent),
            State::Open => ("開校できる", t.positive),
            State::Locked => ("条件あり", t.text_dim),
        };
        let bw = ui.badge_at(r.x + w + 8.0, r.y + 2.0, badge, color);
        let rw = ui.badge_at(r.x + w + 8.0 + bw + 6.0, r.y + 2.0, &area.region, t.series[4]);
        if world.area_conquered(a) {
            ui.badge_at(r.x + w + 8.0 + bw + 6.0 + rw + 6.0, r.y + 2.0, "制覇", t.star);
        }
        if !area.kana.is_empty() {
            ui.label_ex(&area.kana, t.font_small - 1.0, FontWeight::Regular, t.text_dim, Align::Left);
        }
        ui.label_ex(&area.description, t.font_small, FontWeight::Regular, t.text_dim, Align::Left);
        if !area.lines.is_empty() {
            ui.kv_row("路線", &area.lines, None);
        }
        ui.kv_row("所在地", &area.address, None);
        if let Some((share, rank)) = world.area_standing(a) {
            let c = if world.area_conquered(a) { t.star } else { t.text };
            ui.kv_row("自塾の順位", &format!("{rank} 位（シェア {:.0}%）", share * 100.0), Some(c));
        }

        let growth = if area.pool_growth > 0.0 {
            format!("（毎年 +{:.0}%）", area.pool_growth * 100.0)
        } else if area.pool_growth < 0.0 {
            format!("（毎年 {:.0}%）", area.pool_growth * 100.0)
        } else {
            String::new()
        };
        ui.kv_row("市場規模", &format!("×{:.2}{growth}", area.pool), Some(t.star));
        ui.kv_row("家賃", &format!("{}/週", format_yen(area.rent)), None);
        ui.kv_row("開校費用", &format_yen(area.open_cost), None);
        ui.kv_row("エリアの通塾生", &format!("{total}人"), None);

        // 塾ごとのシェア（横棒）
        if total > 0 {
            let mut sorted = shares.clone();
            sorted.sort_by_key(|s| std::cmp::Reverse(s.1));
            for (name, students, rgb, ours) in sorted.iter().take(6) {
                let r = ui.alloc(20.0);
                let c = Color::rgb(rgb[0], rgb[1], rgb[2]);
                let label_w = (r.w * 0.42).min(150.0);
                let weight = if *ours { FontWeight::Bold } else { FontWeight::Regular };
                let label = ui.fit_text(name, t.font_small, weight, label_w - 4.0);
                ui.text_in(Rect::new(r.x, r.y, label_w, r.h), &label, t.font_small, weight, t.text, Align::Left);
                let bar_w = r.w - label_w - 52.0;
                let frac = *students as f32 / total as f32;
                ui.dl.rect(Rect::new(r.x + label_w, r.y + 5.0, bar_w, 10.0), Color::WHITE.with_alpha(0.06), 5.0);
                ui.dl.rect(Rect::new(r.x + label_w, r.y + 5.0, (bar_w * frac).max(4.0), 10.0), c, 5.0);
                ui.text_in(r, &format!("{students}人"), t.font_small, FontWeight::Regular, t.text_dim, Align::Right);
            }
        } else {
            ui.label_ex(
                "このエリアにはまだ塾がありません。",
                t.font_small,
                FontWeight::Regular,
                t.text_dim,
                Align::Left,
            );
        }
        for comp in world.competitors.iter().filter(|c| c.for_sale && !c.acquired && c.branch(a).is_some()) {
            ui.label_ex(
                &format!("{}が売りに出ています（下の「塾の買収」から買えます）", comp.name),
                t.font_small,
                FontWeight::Bold,
                t.negative,
                Align::Left,
            );
        }

        match st {
            State::Ours(ci) => {
                let c = &world.classrooms[ci];
                let w = ui.width();
                let img = ui.alloc((w * 9.0 / 16.0).min(180.0));
                images::picture(ui, img, &format!("{}の校舎", c.name), &images::school_file(&area.name), t.accent);
                ui.label_ex(
                    &format!("{}：在籍 {} 人・講師 {} 人", c.name, c.students.len(), c.teachers.len()),
                    t.font_small,
                    FontWeight::Bold,
                    t.accent,
                    Align::Left,
                );
                if ui.button("この教室を見る", ButtonStyle::Secondary) {
                    *action = Some(HqAction::Select(ci));
                }
            }
            State::Open => {
                ui.caption(
                    "2 教室目からは本部の運営費（週 ¥60,000／教室）がかかります。新しい教室は生徒ゼロから始まります。",
                );
                let affordable = world.cash >= area.open_cost;
                let label = format!("ここに開校する（{}）", format_yen(area.open_cost));
                let style = if affordable { ButtonStyle::Primary } else { ButtonStyle::Ghost };
                if ui.button(&label, style) {
                    *action = Some(if affordable {
                        HqAction::Open(a)
                    } else {
                        HqAction::Toast(format!("資金が足りません（あと {}）", format_yen(area.open_cost - world.cash)))
                    });
                }
            }
            State::Locked => {
                ui.label_ex(
                    &format!("出店の条件：{}", area.unlock.label()),
                    t.font_body,
                    FontWeight::Bold,
                    t.accent,
                    Align::Left,
                );
            }
        }
    });
}
