#!/usr/bin/env python3
"""地図・街・校舎の画像を描く（ゲームの正式な絵）。

    python3 tools/gen_images.py                 # すべて描き直す
    python3 tools/gen_images.py kichijoji map   # 名前に一致するものだけ描き直す

フラットなイラスト調。2 倍の大きさで描いてから縮め、輪郭をなめらかにする。
街ごとに時間帯・季節・景色の型を決めてあり、乱数は街の名前で固定するので、何度描いても同じ絵になる。
必要: Pillow・NumPy。一覧は docs/images/README.md。
"""

import math
import random
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONT_BOLD = ROOT / "assets/fonts/NotoSansJP-Bold.subset.otf"
FONT_REG = ROOT / "assets/fonts/NotoSansJP-Regular.subset.otf"

W, H = 1280, 720
SS = 2  # 何倍で描いてから縮めるか
COMPANY = "あおぞら進学塾"
ORANGE = (0xE3, 0x9B, 0x3A)

# 地方の色（駅名標の帯）
REGION_COLOR = {
    "東京西部": (0xF1, 0x5A, 0x22),
    "埼玉": (0x00, 0xB2, 0xE5),
    "都心": (0x9A, 0xCD, 0x32),
    "神奈川": (0x0D, 0x6A, 0xB8),
    "湾岸・千葉": (0xFF, 0xC8, 0x00),
}

# 時間帯ごとの空（上から下への色）・遠景の色・窓の明かりの割合
TIMES = {
    "day": dict(sky=[(0x4F, 0x93, 0xD6), (0x8C, 0xC0, 0xE8), (0xD8, 0xEC, 0xF4)], haze=(0xC8, 0xDE, 0xEA), light=0.15, sun=(0xFF, 0xF6, 0xD8)),
    "afternoon": dict(sky=[(0x5F, 0x95, 0xC8), (0xA8, 0xC4, 0xD8), (0xF6, 0xD9, 0xAA)], haze=(0xEE, 0xD8, 0xB8), light=0.25, sun=(0xFF, 0xE2, 0xA0)),
    "dusk": dict(sky=[(0x2A, 0x2C, 0x5C), (0x8E, 0x4E, 0x7E), (0xF0, 0x8E, 0x5A), (0xF8, 0xC0, 0x78)], haze=(0xC8, 0x86, 0x7A), light=0.6, sun=(0xFF, 0xB0, 0x60)),
    "night": dict(sky=[(0x0B, 0x12, 0x2C), (0x1C, 0x2A, 0x55), (0x3A, 0x48, 0x78)], haze=(0x3A, 0x46, 0x70), light=0.8, sun=None),
}

# 季節ごとの木の色（影・中・光）
FOLIAGE = {
    "summer": [(0x2E, 0x6B, 0x3A), (0x3F, 0x8A, 0x4A), (0x6C, 0xB0, 0x5E)],
    "spring": [(0xD8, 0x8F, 0xA8), (0xF0, 0xB4, 0xC6), (0xFB, 0xDA, 0xE2)],
    "autumn": [(0xC0, 0x8A, 0x1E), (0xE2, 0xB0, 0x2E), (0xF6, 0xD8, 0x6A)],
    "fall_red": [(0xA0, 0x3A, 0x28), (0xCC, 0x5A, 0x30), (0xE8, 0x92, 0x50)],
}

# (slug, 名前, 読み, 地方, 景色, 時間帯, 季節, 校舎の型)
TOWNS = [
    ("kichijoji", "吉祥寺", "きちじょうじ", "東京西部", "park", "day", "spring", "zakkyo"),
    ("seijogakuenmae", "成城学園前", "せいじょうがくえんまえ", "東京西部", "residential", "afternoon", "autumn", "white"),
    ("chofu", "調布", "ちょうふ", "東京西部", "plaza", "day", "summer", "office"),
    ("fuchu", "府中", "ふちゅう", "東京西部", "avenue", "afternoon", "summer", "wood"),
    ("futakotamagawa", "二子玉川", "ふたこたまがわ", "東京西部", "river", "dusk", "summer", "glass"),
    ("urawa", "浦和", "うらわ", "埼玉", "schoolzone", "day", "summer", "brick"),
    ("omiya", "大宮", "おおみや", "埼玉", "railway", "day", "summer", "office"),
    ("tokorozawa", "所沢", "ところざわ", "埼玉", "lawn", "day", "summer", "suburb"),
    ("shibuya", "渋谷", "しぶや", "都心", "scramble", "dusk", "summer", "tower"),
    ("myogadani", "茗荷谷", "みょうがだに", "都心", "slope", "day", "spring", "stone"),
    ("yokohama", "横浜", "よこはま", "神奈川", "port", "night", "summer", "office"),
    ("tamaplaza", "たまプラーザ", "たまぷらーざ", "神奈川", "hillside", "afternoon", "fall_red", "white"),
    ("musashikosugi", "武蔵小杉", "むさしこすぎ", "神奈川", "towers", "day", "summer", "podium"),
    ("toyosu", "豊洲", "とよす", "湾岸・千葉", "canal", "dusk", "summer", "glass"),
    ("funabashi", "船橋", "ふなばし", "湾岸・千葉", "arcade", "afternoon", "summer", "zakkyo"),
    ("kashiwa", "柏", "かしわ", "湾岸・千葉", "deck", "dusk", "summer", "office"),
]


def soft_blur(im, radius):
    """透明な部分の黒がにじまないよう、乗算済みアルファでぼかす。"""
    arr = np.asarray(im, np.float32)
    a = arr[..., 3:4] / 255.0
    pre = np.concatenate([arr[..., :3] * a, arr[..., 3:4]], axis=2)
    out = []
    for k in range(4):
        ch = Image.fromarray(np.clip(pre[..., k], 0, 255).astype(np.uint8), "L").filter(ImageFilter.GaussianBlur(radius))
        out.append(np.asarray(ch, np.float32))
    pre = np.stack(out, axis=2)
    alpha = pre[..., 3:4]
    rgb = np.where(alpha > 0.5, pre[..., :3] / np.maximum(alpha / 255.0, 1e-6), 0)
    res = np.concatenate([np.clip(rgb, 0, 255), np.clip(alpha, 0, 255)], axis=2).astype(np.uint8)
    return Image.fromarray(res, "RGBA")


def font(size, bold=True):
    return ImageFont.truetype(str(FONT_BOLD if bold else FONT_REG), int(size))


def lerp(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


def shade(c, k):
    return tuple(max(0, min(255, int(v * k))) for v in c[:3])


def mix(c, other, t):
    return lerp(c, other, t)


def rgba(c, a):
    return tuple(c[:3]) + (int(a),)


class Art:
    """論理座標（W×H）で描き、内部では SS 倍で描くキャンバス。"""

    def __init__(self, w, h, img=None):
        self.w, self.h = w, h
        self.img = img if img is not None else Image.new("RGBA", (w * SS, h * SS), (0, 0, 0, 0))
        self.d = ImageDraw.Draw(self.img, "RGBA")

    def _s(self, v):
        return v * SS

    def rect(self, x0, y0, x1, y1, fill, r=0):
        box = [x0 * SS, y0 * SS, x1 * SS, y1 * SS]
        if box[2] <= box[0] or box[3] <= box[1]:
            return
        if r > 0:
            self.d.rounded_rectangle(box, radius=r * SS, fill=fill)
        else:
            self.d.rectangle(box, fill=fill)

    def poly(self, pts, fill):
        self.d.polygon([(x * SS, y * SS) for x, y in pts], fill=fill)

    def ellipse(self, x0, y0, x1, y1, fill):
        self.d.ellipse([x0 * SS, y0 * SS, x1 * SS, y1 * SS], fill=fill)

    def circle(self, x, y, r, fill):
        self.ellipse(x - r, y - r, x + r, y + r, fill)

    def line(self, pts, fill, width):
        self.d.line([(x * SS, y * SS) for x, y in pts], fill=fill, width=max(1, int(width * SS)), joint="curve")

    def text(self, x, y, s, size, fill, bold=True, anchor="mm"):
        self.d.text((x * SS, y * SS), s, font=font(size * SS, bold), fill=fill, anchor=anchor)

    def text_w(self, s, size, bold=True):
        return self.d.textlength(s, font=font(size * SS, bold)) / SS

    def vgrad(self, y0, y1, stops, x0=0, x1=None):
        x1 = self.w if x1 is None else x1
        n = int((y1 - y0) * SS)
        for i in range(n):
            t = i / max(1, n - 1)
            k = t * (len(stops) - 1)
            j = min(int(k), len(stops) - 2)
            c = lerp(stops[j], stops[j + 1], k - j)
            self.d.line([(x0 * SS, y0 * SS + i), (x1 * SS, y0 * SS + i)], fill=c)

    def layer(self):
        return Art(self.w, self.h)

    def paste(self, layer, blur=0, opacity=1.0):
        im = layer.img
        if blur:
            im = soft_blur(im, blur * SS)
        if opacity < 1.0:
            a = im.getchannel("A").point(lambda v: int(v * opacity))
            im.putalpha(a)
        self.img.alpha_composite(im)

    def finish(self):
        return self.img.resize((self.w, self.h), Image.LANCZOS).convert("RGB")


# ---------------------------------------------------------------------------
# 部品
# ---------------------------------------------------------------------------


def sky(a, time, horizon, rng):
    t = TIMES[time]
    a.vgrad(0, horizon + 40, t["sky"])
    if time == "night":
        for _ in range(160):
            x, y = rng.uniform(0, a.w), rng.uniform(0, horizon * 0.8)
            r = rng.choice([0.6, 0.8, 1.0, 1.4])
            a.circle(x, y, r, rgba((255, 255, 240), rng.randint(120, 230)))
        moon = a.layer()
        moon.circle(a.w * 0.82, horizon * 0.22, 60, rgba((0xF4, 0xEC, 0xC8), 90))
        a.paste(moon, blur=30)
        a.circle(a.w * 0.82, horizon * 0.22, 24, (0xF6, 0xF0, 0xD6))
        a.circle(a.w * 0.82 + 10, horizon * 0.22 - 6, 21, t["sky"][0])
    elif t["sun"]:
        sx = a.w * rng.uniform(0.62, 0.86)
        sy = horizon * (0.62 if time == "dusk" else 0.2)
        glow = a.layer()
        glow.circle(sx, sy, 170, rgba(t["sun"], 120))
        glow.circle(sx, sy, 80, rgba(t["sun"], 160))
        a.paste(glow, blur=50)
        a.circle(sx, sy, 34 if time == "dusk" else 28, rgba(t["sun"], 255))
    # 雲
    cl = a.layer()
    ccol = {"day": (255, 255, 255), "afternoon": (0xFF, 0xF2, 0xE0), "dusk": (0xF6, 0xB0, 0x90), "night": (0x50, 0x5C, 0x88)}[time]
    for _ in range(rng.randint(4, 7)):
        cx, cy = rng.uniform(-50, a.w + 50), rng.uniform(40, horizon * 0.55)
        s = rng.uniform(0.6, 1.4)
        for _ in range(7):
            dx, dy = rng.uniform(-90, 90) * s, rng.uniform(-14, 10) * s
            r = rng.uniform(22, 44) * s
            cl.ellipse(cx + dx - r * 1.6, cy + dy - r, cx + dx + r * 1.6, cy + dy + r * 0.7, rgba(ccol, 200))
        cl.rect(cx - 110 * s, cy + 4 * s, cx + 110 * s, cy + 14 * s, rgba(ccol, 200), r=6)
    a.paste(cl, blur=3, opacity=0.85)


def mountains(a, horizon, time, rng, fuji=True):
    """西の山並み（丹沢と富士山）。"""
    t = TIMES[time]
    col = mix(t["haze"], (0x50, 0x66, 0x88), 0.35)
    pts = [(0, horizon)]
    x = 0
    while x <= a.w:
        pts.append((x, horizon - 40 - 30 * math.sin(x / 90 + rng.random()) - rng.uniform(0, 18)))
        x += 40
    pts.append((a.w, horizon))
    a.poly(pts, rgba(col, 200))
    if fuji:
        fx = a.w * 0.2
        peak = horizon - 150
        a.poly([(fx - 260, horizon - 20), (fx - 28, peak), (fx + 28, peak), (fx + 260, horizon - 20)], rgba(mix(col, (0x40, 0x55, 0x80), 0.3), 220))
        snow = (0xF4, 0xF6, 0xFA) if time != "night" else (0x9A, 0xA6, 0xC8)
        a.poly([(fx - 70, peak + 40), (fx - 28, peak), (fx + 28, peak), (fx + 70, peak + 40), (fx + 40, peak + 34), (fx + 14, peak + 46),
                (fx - 12, peak + 36), (fx - 40, peak + 48)], rgba(snow, 230))


def skyline(a, base, time, rng, hmin, hmax, wmin=40, wmax=110, depth=0.0, windows=False):
    """遠景・中景のビル群。depth が大きいほどかすむ。"""
    t = TIMES[time]
    body = {"day": (0x6E, 0x80, 0x98), "afternoon": (0x86, 0x7E, 0x84), "dusk": (0x4A, 0x3C, 0x5E), "night": (0x1A, 0x22, 0x40)}[time]
    col = mix(body, t["haze"], depth)
    x = rng.uniform(-60, 0)
    while x < a.w:
        w = rng.uniform(wmin, wmax)
        h = rng.uniform(hmin, hmax)
        a.rect(x, base - h, x + w, base + 2, col)
        if rng.random() < 0.3:  # 屋上の設備
            a.rect(x + w * 0.2, base - h - 8, x + w * 0.45, base - h, col)
        if windows:
            lit = (0xFF, 0xD8, 0x8A)
            for wy in np.arange(base - h + 8, base - 6, 11):
                for wx in np.arange(x + 5, x + w - 6, 9):
                    if rng.random() < t["light"] * 0.7:
                        a.rect(wx, wy, wx + 4, wy + 5, rgba(lit, 200))
        x += w + rng.uniform(-6, 10)


def haze(a, y, height, time, alpha=150):
    t = TIMES[time]
    hz = a.layer()
    hz.rect(-20, y - height / 2, a.w + 20, y + height / 2, rgba(t["haze"], alpha))
    a.paste(hz, blur=height / 3)


def window_grid(a, x0, y0, x1, y1, cols, rows, frame, glass, lit, p_lit, rng, gap=0.28):
    cw = (x1 - x0) / cols
    rh = (y1 - y0) / rows
    for r in range(rows):
        for c in range(cols):
            wx0 = x0 + c * cw + cw * gap / 2
            wy0 = y0 + r * rh + rh * gap / 2
            wx1, wy1 = wx0 + cw * (1 - gap), wy0 + rh * (1 - gap)
            a.rect(wx0 - 1.5, wy0 - 1.5, wx1 + 1.5, wy1 + 1.5, frame)
            if rng.random() < p_lit:
                a.rect(wx0, wy0, wx1, wy1, lit)
                a.rect(wx0, wy0, wx1, wy0 + (wy1 - wy0) * 0.25, mix(lit, (255, 255, 255), 0.3))
            else:
                a.rect(wx0, wy0, wx1, wy1, glass)
                a.poly([(wx0, wy1), (wx0, wy0 + (wy1 - wy0) * 0.6), (wx0 + (wx1 - wx0) * 0.5, wy0)], rgba((255, 255, 255), 40))
                a.rect(wx0, wy0, wx1, wy1, rgba(glass, 0))


def building(a, x, base, w, h, body, time, rng, cols=None, rows=None, roof=True, side=True):
    t = TIMES[time]
    lit = (0xFF, 0xD6, 0x82)
    glass = mix(shade(body, 0.55), (0x9C, 0xC4, 0xE0) if time in ("day", "afternoon") else (0x30, 0x3A, 0x60), 0.5)
    a.rect(x, base - h, x + w, base, body)
    if side:
        a.rect(x + w * 0.86, base - h, x + w, base, shade(body, 0.82))
    a.rect(x - 3, base - h - 6, x + w + 3, base - h, shade(body, 1.12))
    cols = cols or max(2, int(w / 26))
    rows = rows or max(2, int(h / 32))
    window_grid(a, x + 8, base - h + 12, x + w * 0.86 - 4, base - 14, cols, rows, shade(body, 0.75), glass, lit, t["light"], rng)
    if roof and rng.random() < 0.7:
        rx = x + rng.uniform(0.1, 0.5) * w
        a.rect(rx, base - h - 20, rx + 24, base - h - 6, shade(body, 0.9))
        a.rect(rx + 30, base - h - 14, rx + 46, base - h - 6, shade(body, 0.8))
        a.line([(x + 4, base - h - 12), (x + w - 4, base - h - 12)], shade(body, 0.7), 1.2)


def tree(a, x, base, h, season, rng, kind="round"):
    f = FOLIAGE[season]
    trunk = (0x5A, 0x40, 0x30)
    if kind == "ginkgo" or kind == "cone":
        a.rect(x - h * 0.04, base - h * 0.35, x + h * 0.04, base, trunk)
        a.poly([(x - h * 0.32, base - h * 0.25), (x, base - h), (x + h * 0.32, base - h * 0.25)], f[0])
        a.poly([(x - h * 0.25, base - h * 0.3), (x - h * 0.02, base - h * 0.95), (x + h * 0.12, base - h * 0.35)], f[1])
        a.poly([(x - h * 0.12, base - h * 0.45), (x - h * 0.04, base - h * 0.85), (x + h * 0.02, base - h * 0.5)], f[2])
        return
    a.rect(x - h * 0.045, base - h * 0.45, x + h * 0.045, base, trunk)
    spread = 0.55 if kind == "zelkova" else 0.38
    for k, col in enumerate(f):
        for _ in range(4 - k):
            dx = rng.uniform(-spread, spread) * h * (1 - 0.25 * k)
            dy = rng.uniform(-0.15, 0.1) * h
            r = rng.uniform(0.18, 0.26) * h * (1 - 0.2 * k)
            cx, cy = x + dx - k * h * 0.04, base - h * 0.68 + dy - k * h * 0.05
            a.ellipse(cx - r, cy - r, cx + r, cy + r * 0.9, col)


def house(a, x, base, w, body, roof, time, rng):
    h = w * 0.62
    a.rect(x, base - h, x + w, base, body)
    a.rect(x + w * 0.82, base - h, x + w, base, shade(body, 0.88))
    a.poly([(x - 10, base - h + 2), (x + w * 0.5, base - h - w * 0.36), (x + w + 10, base - h + 2)], roof)
    a.poly([(x + w * 0.5, base - h - w * 0.36), (x + w + 10, base - h + 2), (x + w * 0.5, base - h + 2)], shade(roof, 0.85))
    lit = TIMES[time]["light"] > 0.3
    wc = (0xFF, 0xD8, 0x90) if lit else (0x9C, 0xBC, 0xD4)
    a.rect(x + w * 0.12, base - h * 0.75, x + w * 0.4, base - h * 0.42, shade(body, 0.7))
    a.rect(x + w * 0.14, base - h * 0.73, x + w * 0.38, base - h * 0.44, wc)
    a.rect(x + w * 0.56, base - h * 0.62, x + w * 0.74, base, shade(roof, 0.7))
    # 生け垣
    a.rect(x - 14, base - 16, x + w + 14, base, (0x3A, 0x6E, 0x40), r=6)


def pole_and_wires(a, xs, base, time, sag=26):
    col = (0x3A, 0x36, 0x34) if time != "night" else (0x10, 0x14, 0x24)
    tops = []
    for x in xs:
        a.rect(x - 3, base - 170, x + 3, base, col)
        a.rect(x - 22, base - 160, x + 22, base - 156, col)
        a.rect(x - 16, base - 140, x + 16, base - 137, col)
        tops.append(x)
    for y_off in (158, 138, 124):
        for x0, x1 in zip(tops, tops[1:]):
            pts = []
            for k in range(21):
                tt = k / 20
                pts.append((x0 + (x1 - x0) * tt, base - y_off + sag * 4 * tt * (1 - tt)))
            a.line(pts, rgba(col, 220), 1.1)


def lamp(a, x, base, time):
    col = (0x44, 0x4A, 0x50)
    a.rect(x - 2, base - 90, x + 2, base, col)
    a.line([(x, base - 90), (x + 14, base - 96)], col, 3)
    lit = time in ("dusk", "night")
    c = (0xFF, 0xE8, 0xB0) if lit else (0xE0, 0xE4, 0xE8)
    a.rect(x + 8, base - 98, x + 24, base - 92, c, r=2)
    if lit:
        g = a.layer()
        g.circle(x + 16, base - 92, 26, rgba((0xFF, 0xDC, 0x90), 140))
        a.paste(g, blur=10)


def people(a, x0, x1, base, n, rng, scale=1.0):
    cols = [(0x3A, 0x4A, 0x6A), (0xC8, 0x5A, 0x4A), (0xE8, 0xE0, 0xD0), (0x4A, 0x7A, 0x5A), (0x2A, 0x2A, 0x30), (0xD8, 0xA0, 0x3A)]
    for _ in range(n):
        x = rng.uniform(x0, x1)
        y = base + rng.uniform(-6, 6) * scale
        s = rng.uniform(0.85, 1.1) * scale
        c = rng.choice(cols)
        a.rect(x - 4 * s, y - 22 * s, x + 4 * s, y - 6 * s, c, r=3 * s)
        a.rect(x - 3 * s, y - 7 * s, x - 0.5 * s, y, (0x30, 0x30, 0x38))
        a.rect(x + 0.5 * s, y - 7 * s, x + 3 * s, y, (0x30, 0x30, 0x38))
        a.circle(x, y - 26 * s, 4 * s, (0x3A, 0x2A, 0x22))


def train(a, x, y, cars, stripe, time, length=150):
    body = (0xE6, 0xE8, 0xEA) if time != "night" else (0x9A, 0xA0, 0xB0)
    win = (0x5A, 0x78, 0x98) if time in ("day", "afternoon") else (0xFF, 0xE0, 0x9A)
    for k in range(cars):
        cx = x + k * (length + 6)
        a.rect(cx, y - 38, cx + length, y, body, r=6)
        a.rect(cx, y - 14, cx + length, y - 8, stripe)
        for wx in np.arange(cx + 10, cx + length - 16, 22):
            a.rect(wx, y - 32, wx + 14, y - 18, win, r=2)
        a.rect(cx + length * 0.45, y - 34, cx + length * 0.55, y - 2, shade(body, 0.85))
        a.circle(cx + 22, y + 3, 5, (0x30, 0x30, 0x34))
        a.circle(cx + length - 22, y + 3, 5, (0x30, 0x30, 0x34))
    a.line([(x - 40, y - 52), (x + cars * (length + 6) + 40, y - 52)], (0x40, 0x40, 0x48), 1.2)


def torii(a, x, base, h, col=(0xD0, 0x3A, 0x2A)):
    w = h * 0.9
    a.rect(x - w * 0.36, base - h, x - w * 0.28, base, col)
    a.rect(x + w * 0.28, base - h, x + w * 0.36, base, col)
    a.rect(x - w * 0.5, base - h, x + w * 0.5, base - h * 0.9, col, r=4)
    a.rect(x - w * 0.54, base - h * 1.06, x + w * 0.54, base - h * 0.98, (0x2A, 0x26, 0x24), r=4)
    a.rect(x - w * 0.42, base - h * 0.76, x + w * 0.42, base - h * 0.7, col)


def water(a, y0, y1, time, rng, base_col=None):
    t = TIMES[time]
    top = mix(t["sky"][-1], (0x2E, 0x6A, 0x96), 0.5) if base_col is None else base_col
    a.vgrad(y0, y1, [top, shade(top, 0.7)])
    for _ in range(int((y1 - y0) * 0.9)):
        y = rng.uniform(y0 + 4, y1)
        x = rng.uniform(-20, a.w)
        L = rng.uniform(20, 90) * (0.4 + (y - y0) / (y1 - y0))
        a.line([(x, y), (x + L, y)], rgba((255, 255, 255), rng.randint(25, 70)), 1.4)


def reflect(a, y_axis, depth, blur=4, opacity=0.35):
    """y_axis より上を水面に映す。"""
    src = a.img.crop((0, int((y_axis - depth) * SS), a.w * SS, int(y_axis * SS))).transpose(Image.FLIP_TOP_BOTTOM)
    lay = Art(a.w, a.h)
    lay.img.paste(src, (0, int(y_axis * SS)))
    a.paste(lay, blur=blur, opacity=opacity)


def road(a, y0, y1, time, crosswalk=True):
    asphalt = (0x5A, 0x5C, 0x62) if time != "night" else (0x24, 0x28, 0x38)
    a.vgrad(y0, y1, [shade(asphalt, 1.1), asphalt])
    if crosswalk:
        cx = a.w * 0.5
        for k in range(-9, 10):
            xt = cx + k * 34
            xb = cx + k * 70
            a.poly([(xt - 9, y0 + 8), (xt + 9, y0 + 8), (xb + 18, y1), (xb - 18, y1)], rgba((0xF2, 0xF2, 0xEE), 215))


def sidewalk(a, y0, y1, col=(0xC8, 0xBE, 0xAE)):
    a.rect(0, y0, a.w, y1, col)
    a.rect(0, y0, a.w, y0 + 4, shade(col, 1.08))
    for x in range(0, a.w, 40):
        a.line([(x, y0 + 4), (x - 10, y1)], rgba(shade(col, 0.85), 120), 1)


def screen_board(a, x0, y0, x1, y1, rng):
    """ビルの大型ビジョン（文字は入れない）。"""
    cols = [(0x3A, 0x8E, 0xF0), (0xF0, 0x5A, 0x8E), (0x6A, 0xE0, 0xB0), (0xF8, 0xC8, 0x40)]
    c1, c2 = rng.sample(cols, 2)
    lay = Art(a.w, a.h)
    lay.rect(x0, y0, x1, y1, rgba(c1, 255))
    lay.poly([(x0, y1), (x1, y0), (x1, y1)], rgba(c2, 255))
    lay.circle((x0 + x1) / 2, (y0 + y1) / 2, (y1 - y0) * 0.25, rgba((255, 255, 255), 200))
    g = Art(a.w, a.h)
    g.rect(x0 - 10, y0 - 10, x1 + 10, y1 + 10, rgba(c1, 120))
    a.paste(g, blur=14)
    a.rect(x0 - 3, y0 - 3, x1 + 3, y1 + 3, (0x22, 0x22, 0x2A))
    a.paste(lay)


def station_sign(a, name, kana, region):
    """左下の駅名標ふうの名札。"""
    size = 46
    tw = max(a.text_w(name, size), a.text_w(kana, 18, False)) + 70
    x0, y1 = 28, a.h - 28
    y0 = y1 - 112
    sh = a.layer()
    sh.rect(x0 + 4, y0 + 6, x0 + tw + 4, y1 + 6, rgba((0, 0, 0), 110), r=10)
    a.paste(sh, blur=6)
    a.rect(x0, y0, x0 + tw, y1, (0xFA, 0xFA, 0xF6), r=10)
    a.rect(x0, y1 - 26, x0 + tw, y1 - 10, REGION_COLOR[region])
    a.text(x0 + tw / 2, y0 + 40, name, size, (0x22, 0x22, 0x28))
    a.text(x0 + tw / 2, y0 + 76, kana, 18, (0x55, 0x55, 0x5C), bold=False)


def vignette(a, strength=70):
    v = a.layer()
    for k in range(12):
        m = k * 6
        v.d.rectangle([m * SS, m * SS, (a.w - m) * SS, (a.h - m) * SS], outline=(0, 0, 0, int(strength * (1 - k / 12) ** 2)), width=6 * SS)
    a.paste(v, blur=10)


# ---------------------------------------------------------------------------
# 街の景色
# ---------------------------------------------------------------------------


def backdrop(a, time, rng, horizon, west=False, city=0.5):
    sky(a, time, horizon, rng)
    if west:
        mountains(a, horizon - 30, time, rng)
    skyline(a, horizon, time, rng, 40 + 60 * city, 90 + 160 * city, depth=0.6, windows=time in ("dusk", "night"))
    haze(a, horizon - 20, 70, time, alpha=110)
    skyline(a, horizon + 20, time, rng, 30 + 40 * city, 70 + 120 * city, 50, 130, depth=0.3, windows=time in ("dusk", "night"))
    a.rect(0, horizon + 18, a.w, a.h, (0x6E, 0x86, 0x5E))


def scene_park(a, s, rng):
    time, season = s["time"], s["season"]
    hz = 380
    backdrop(a, time, rng, hz, west=True, city=0.3)
    # アーケード街（奥）
    for k in range(4):
        building(a, 560 + k * 150, hz + 30, 130, rng.uniform(120, 200), rng.choice([(0xC8, 0xB4, 0x98), (0xB8, 0xC0, 0xC8), (0xD8, 0xC8, 0xB0)]), time, rng)
    a.rect(0, hz + 30, a.w, a.h, (0x5E, 0x8A, 0x52))
    # 池とボート
    a.ellipse(-120, hz + 120, a.w + 80, a.h + 260, (0x3E, 0x7A, 0x9A))
    water_l = a.layer()
    for _ in range(80):
        y = rng.uniform(hz + 150, a.h)
        x = rng.uniform(0, a.w)
        water_l.line([(x, y), (x + rng.uniform(20, 70), y)], rgba((255, 255, 255), 60), 1.4)
    a.paste(water_l)
    for bx, by, c in [(420, 560, (0xFF, 0xFF, 0xFF)), (700, 600, (0xF4, 0xC0, 0x40)), (900, 540, (0xFF, 0xFF, 0xFF))]:
        a.ellipse(bx - 34, by - 10, bx + 34, by + 12, c)
        a.circle(bx + 20, by - 18, 9, c)
        a.rect(bx + 16, by - 30, bx + 22, by - 16, c)
        a.circle(bx + 24, by - 30, 4, (0xF0, 0x90, 0x30))
    # 橋
    a.rect(100, hz + 108, 640, hz + 118, (0x9A, 0x6A, 0x4A))
    for x in range(110, 640, 40):
        a.rect(x, hz + 92, x + 4, hz + 118, (0x8A, 0x5A, 0x3A))
    a.line([(100, hz + 94), (640, hz + 94)], (0x8A, 0x5A, 0x3A), 3)
    people(a, 150, 600, hz + 106, 9, rng, 0.8)
    # 手前の木
    for x in (-30, 70, 180, 1050, 1160, 1270):
        tree(a, x, hz + rng.uniform(140, 200), rng.uniform(220, 300), season, rng)
    for x in range(260, 1000, 120):
        tree(a, x + rng.uniform(-20, 20), hz + 70, rng.uniform(110, 150), "summer" if rng.random() < 0.4 else season, rng)


def scene_residential(a, s, rng):
    time, season = s["time"], s["season"]
    hz = 400
    backdrop(a, time, rng, hz, west=True, city=0.15)
    a.rect(0, hz + 20, a.w, a.h, (0x6E, 0x8C, 0x5A))
    # 奥の家並み
    for k in range(9):
        house(a, k * 150 - 20, hz + 90, 110, rng.choice([(0xEE, 0xE6, 0xD8), (0xE0, 0xD4, 0xBE), (0xF4, 0xF0, 0xEA)]),
              rng.choice([(0x6A, 0x4A, 0x40), (0x4A, 0x5A, 0x6A), (0x8A, 0x5A, 0x3E)]), time, rng)
    # 並木道
    a.poly([(a.w * 0.44, hz + 100), (a.w * 0.56, hz + 100), (a.w * 0.95, a.h), (a.w * 0.05, a.h)], (0x8E, 0x8A, 0x86))
    a.poly([(a.w * 0.495, hz + 100), (a.w * 0.505, hz + 100), (a.w * 0.52, a.h), (a.w * 0.48, a.h)], rgba((0xF2, 0xEE, 0xE0), 160))
    for k in range(8):
        t = k / 7
        hgt = 90 + 260 * t
        y = hz + 110 + (a.h - hz - 110) * t
        for side in (-1, 1):
            tree(a, a.w * (0.5 + side * (0.08 + 0.42 * t)), y, hgt, season, rng, kind="ginkgo")
    # 落ち葉
    for _ in range(140):
        x, y = rng.uniform(0, a.w), rng.uniform(hz + 200, a.h)
        a.circle(x, y, rng.uniform(1.5, 3.5), rgba(FOLIAGE[season][2], 200))
    people(a, 560, 720, hz + 220, 3, rng, 1.2)


def scene_plaza(a, s, rng):
    time, season = s["time"], s["season"]
    hz = 400
    backdrop(a, time, rng, hz, west=True, city=0.45)
    for k, (w, h) in enumerate([(240, 250), (180, 190), (260, 300), (200, 210)]):
        building(a, 40 + k * 310, hz + 80, w, h, rng.choice([(0xD8, 0xD0, 0xC4), (0xB8, 0xC6, 0xD2), (0xE6, 0xDC, 0xCC)]), time, rng)
    sidewalk(a, hz + 80, a.h)
    # バスロータリー
    a.ellipse(250, hz + 150, 1030, a.h + 140, (0x66, 0x68, 0x6E))
    a.ellipse(480, hz + 230, 800, hz + 330, (0x6E, 0x9A, 0x58))
    tree(a, 640, hz + 290, 160, season, rng, kind="zelkova")
    for bx in (300, 860):
        a.rect(bx, hz + 175, bx + 150, hz + 225, (0xF2, 0xF2, 0xEE), r=8)
        a.rect(bx, hz + 200, bx + 150, hz + 208, (0x3A, 0x8A, 0x5A))
        for wx in range(bx + 10, bx + 140, 24):
            a.rect(wx, hz + 181, wx + 16, hz + 196, (0x5A, 0x78, 0x98), r=2)
    people(a, 30, 1250, hz + 140, 22, rng)
    lamp(a, 200, hz + 150, time)
    lamp(a, 1080, hz + 150, time)


def scene_avenue(a, s, rng):
    time, season = s["time"], s["season"]
    hz = 390
    backdrop(a, time, rng, hz, west=True, city=0.25)
    a.rect(0, hz + 10, a.w, a.h, (0x5E, 0x7A, 0x52))
    # 参道
    a.poly([(a.w * 0.46, hz + 60), (a.w * 0.54, hz + 60), (a.w * 0.9, a.h), (a.w * 0.1, a.h)], (0xC8, 0xB8, 0x9A))
    for k in range(10):
        t = k / 9
        y = hz + 70 + (a.h - hz - 70) * t
        a.line([(a.w * (0.46 - 0.36 * t), y), (a.w * (0.54 + 0.36 * t), y)], rgba((0xA8, 0x98, 0x7A), 120), 1.2)
    torii(a, a.w * 0.5, hz + 64, 70)
    torii(a, a.w * 0.5, hz + 200, 190)
    for k in range(9):
        t = k / 8
        hgt = 140 + 300 * t
        y = hz + 70 + (a.h - hz - 70) * t
        for side in (-1, 1):
            tree(a, a.w * (0.5 + side * (0.1 + 0.4 * t)), y, hgt, season, rng, kind="zelkova")
    people(a, 520, 760, hz + 260, 6, rng, 1.1)


def scene_river(a, s, rng):
    time = s["time"]
    hz = 380
    backdrop(a, time, rng, hz, west=True, city=0.6)
    # 川向こうの高層ビル
    for k, (x, w, h) in enumerate([(700, 120, 300), (840, 100, 250), (960, 140, 330)]):
        building(a, x, hz + 30, w, h, (0x6A, 0x7E, 0x9A), time, rng)
    a.rect(0, hz + 30, a.w, hz + 50, (0x6E, 0x8E, 0x5A))
    water(a, hz + 50, a.h - 120, time, rng)
    reflect(a, hz + 50, 160, blur=5, opacity=0.3)
    # 鉄橋と電車
    a.rect(-10, hz + 2, a.w + 10, hz + 14, (0x6A, 0x6A, 0x74))
    for x in range(0, a.w, 140):
        a.rect(x, hz + 14, x + 16, hz + 60, (0x5A, 0x5A, 0x64))
        a.line([(x, hz + 2), (x + 70, hz - 30), (x + 140, hz + 2)], (0x7A, 0x7A, 0x84), 4)
    a.line([(0, hz - 30), (a.w, hz - 30)], (0x7A, 0x7A, 0x84), 4)
    train(a, 160, hz + 2, 5, (0x20, 0xA2, 0x88), time)
    # 河川敷
    a.vgrad(a.h - 120, a.h, [(0x7E, 0x9C, 0x58), (0x5A, 0x7A, 0x42)])
    a.rect(0, a.h - 70, a.w, a.h - 58, (0xC8, 0xBA, 0x98))
    people(a, 100, 1100, a.h - 60, 7, rng)


def scene_schoolzone(a, s, rng):
    time, season = s["time"], s["season"]
    hz = 410
    backdrop(a, time, rng, hz, city=0.25)
    # 学校の校舎（奥）
    building(a, 620, hz + 40, 560, 150, (0xF0, 0xEA, 0xDC), time, rng, cols=14, rows=3)
    a.rect(880, hz - 150, 920, hz - 110, (0xF0, 0xEA, 0xDC))
    a.circle(900, hz - 130, 14, (0xF8, 0xF8, 0xF4))
    a.line([(900, hz - 130), (900, hz - 140)], (0x30, 0x30, 0x30), 2)
    a.rect(600, hz + 40, a.w, hz + 60, (0x3A, 0x6A, 0x40))
    for k in range(5):
        house(a, k * 120 - 10, hz + 80, 92, rng.choice([(0xEE, 0xE6, 0xD8), (0xE6, 0xDA, 0xC8)]), (0x6A, 0x4A, 0x40), time, rng)
    sidewalk(a, hz + 80, a.h, col=(0xB8, 0x7A, 0x5E))
    for y in range(hz + 100, a.h, 22):
        a.line([(0, y), (a.w, y)], rgba((0x9A, 0x60, 0x48), 150), 1)
    a.rect(0, a.h - 90, a.w, a.h, (0x6A, 0x6C, 0x72))
    a.rect(0, a.h - 92, a.w, a.h - 88, (0xF0, 0xF0, 0xEC))
    for x in (90, 380, 1080):
        tree(a, x, hz + 110, 230, season, rng)
    pole_and_wires(a, [240, 760, 1260], hz + 130, time)
    # 学校帰りの子ども（ランドセル）
    for x in (520, 560, 610, 860):
        people(a, x, x + 1, hz + 190, 1, rng, 0.9)
        a.rect(x - 7, hz + 160, x - 1, hz + 176, rng.choice([(0xC8, 0x3A, 0x3A), (0x2A, 0x2A, 0x40)]), r=2)


def scene_railway(a, s, rng):
    time, season = s["time"], s["season"]
    hz = 380
    backdrop(a, time, rng, hz, city=0.6)
    for k, (w, h) in enumerate([(220, 230), (300, 280), (200, 180), (260, 250)]):
        building(a, 30 + k * 320, hz + 40, w, h, rng.choice([(0xCC, 0xC6, 0xBE), (0xA8, 0xB8, 0xC8)]), time, rng)
    a.rect(0, hz + 40, a.w, a.h, (0x7A, 0x76, 0x70))
    # 何本もの線路
    for k in range(6):
        y = hz + 90 + k * 46
        a.rect(0, y - 4, a.w, y + 6, (0x8C, 0x84, 0x78))
        a.line([(0, y - 2), (a.w, y - 2)], (0xB8, 0xB8, 0xC0), 2)
        a.line([(0, y + 4), (a.w, y + 4)], (0xB8, 0xB8, 0xC0), 2)
    train(a, -60, hz + 134, 6, (0x00, 0xB2, 0xE5), time)
    train(a, 300, hz + 226, 5, (0x22, 0x8B, 0x3A), time)
    train(a, -200, hz + 318, 7, (0xF1, 0x5A, 0x22), time, length=170)
    # 架線柱
    for x in range(40, a.w, 260):
        a.rect(x, hz + 20, x + 6, a.h, (0x5A, 0x5A, 0x60))
        a.rect(x - 10, hz + 30, x + 200, hz + 36, (0x5A, 0x5A, 0x60))
    # 手前の参道の並木
    tree(a, 1180, a.h + 10, 300, season, rng, kind="zelkova")
    tree(a, 40, a.h + 10, 280, season, rng, kind="zelkova")


def scene_lawn(a, s, rng):
    time, season = s["time"], s["season"]
    hz = 430
    backdrop(a, time, rng, hz, west=True, city=0.1)
    a.vgrad(hz + 20, a.h, [(0x8C, 0xB8, 0x5E), (0x5E, 0x92, 0x48)])
    for _ in range(300):
        x, y = rng.uniform(0, a.w), rng.uniform(hz + 40, a.h)
        a.line([(x, y), (x + 2, y - 6)], rgba((0x4A, 0x7A, 0x3A), 140), 1.2)
    for x in range(-20, a.w + 40, 90):
        tree(a, x + rng.uniform(-20, 20), hz + 30, rng.uniform(90, 130), season, rng)
    # 飛行機
    px, py = 820, 150
    a.ellipse(px - 70, py - 9, px + 70, py + 9, (0xF4, 0xF4, 0xF8))
    a.poly([(px - 10, py), (px + 20, py), (px - 30, py + 44), (px - 46, py + 44)], (0xE0, 0xE2, 0xE8))
    a.poly([(px - 10, py), (px + 20, py), (px - 30, py - 44), (px - 46, py - 44)], (0xE0, 0xE2, 0xE8))
    a.poly([(px - 70, py), (px - 56, py), (px - 74, py - 28), (px - 82, py - 28)], (0xD0, 0xD4, 0xDC))
    a.line([(px - 300, py + 30), (px - 80, py + 4)], rgba((255, 255, 255), 140), 3)
    people(a, 200, 1100, hz + 180, 10, rng, 0.8)
    # 親子とボール
    a.circle(640, hz + 210, 9, (0xF0, 0x60, 0x50))


def scene_scramble(a, s, rng):
    time = s["time"]
    hz = 360
    sky(a, time, hz, rng)
    skyline(a, hz + 40, time, rng, 140, 260, depth=0.45, windows=True)
    skyline(a, hz + 120, time, rng, 120, 200, 60, 120, depth=0.2, windows=True)
    # 交差点を囲むビル
    specs = [(0, 250, 330), (250, 200, 270), (830, 200, 300), (1030, 260, 350)]
    for x, w, h in specs:
        building(a, x, hz + 120, w, h, rng.choice([(0x5E, 0x5A, 0x72), (0x6E, 0x66, 0x7A), (0x4E, 0x58, 0x74)]), time, rng)
    screen_board(a, 40, hz - 170, 210, hz - 70, rng)
    screen_board(a, 870, hz - 150, 1010, hz - 60, rng)
    screen_board(a, 1060, hz - 200, 1250, hz - 90, rng)
    road(a, hz + 120, a.h, time)
    a.rect(0, hz + 110, a.w, hz + 122, (0x9A, 0x96, 0x92))
    people(a, 120, 1160, hz + 220, 40, rng, 0.9)
    people(a, 60, 1220, hz + 320, 30, rng, 1.2)
    for x in (230, 1050):
        lamp(a, x, hz + 122, time)
    # 信号
    a.rect(820, hz + 20, 826, hz + 122, (0x3A, 0x3A, 0x40))
    a.rect(808, hz + 20, 838, hz + 48, (0x2A, 0x2A, 0x30), r=4)
    a.circle(823, hz + 34, 8, (0x40, 0xE0, 0x90))


def scene_slope(a, s, rng):
    time, season = s["time"], s["season"]
    hz = 360
    backdrop(a, time, rng, hz, city=0.35)
    # 古い学校の校舎（レンガと時計塔）
    building(a, 760, hz + 60, 420, 170, (0xB8, 0x6A, 0x52), time, rng, cols=9, rows=3)
    a.rect(930, hz - 200, 1010, hz - 110, (0xB8, 0x6A, 0x52))
    a.poly([(920, hz - 200), (970, hz - 250), (1020, hz - 200)], (0x4A, 0x5A, 0x5A))
    a.circle(970, hz - 165, 22, (0xF4, 0xF0, 0xE4))
    a.line([(970, hz - 165), (970, hz - 180)], (0x30, 0x30, 0x30), 2.5)
    a.line([(970, hz - 165), (982, hz - 160)], (0x30, 0x30, 0x30), 2.5)
    # 坂道
    a.poly([(0, a.h), (0, hz + 160), (a.w, hz + 40), (a.w, a.h)], (0x7E, 0x9A, 0x62))
    a.poly([(0, a.h), (380, hz + 120), (560, hz + 100), (520, a.h)], (0xA8, 0xA0, 0x96))
    for k in range(9):
        t = k / 8
        a.line([(380 - 380 * t, hz + 120 + (a.h - hz - 120) * t), (560 - 40 * t, hz + 100 + (a.h - hz - 100) * t)], rgba((0x8A, 0x84, 0x7A), 150), 1.4)
    # 桜並木
    for k in range(7):
        t = k / 6
        tree(a, 330 - 300 * t, hz + 130 + (a.h - hz - 120) * t, 120 + 230 * t, season, rng)
        tree(a, 610 + 80 * t, hz + 110 + (a.h - hz - 100) * t, 120 + 230 * t, season, rng)
    for _ in range(220):
        x, y = rng.uniform(0, a.w), rng.uniform(80, a.h)
        a.circle(x, y, rng.uniform(1.5, 3), rgba((0xFB, 0xD4, 0xDE), 210))
    people(a, 380, 520, hz + 300, 5, rng, 1.1)


def scene_port(a, s, rng):
    time = s["time"]
    hz = 400
    sky(a, time, hz, rng)
    skyline(a, hz, time, rng, 80, 200, depth=0.35, windows=True)
    # 帆の形のホテルと高層ビル
    a.poly([(560, hz), (600, hz - 300), (660, hz - 240), (680, hz)], (0x6A, 0x78, 0xA8))
    building(a, 430, hz, 110, 330, (0x3A, 0x48, 0x78), time, rng)
    building(a, 720, hz, 90, 220, (0x40, 0x4E, 0x80), time, rng)
    # 観覧車
    cx, cy, r = 980, hz - 150, 140
    wheel = a.layer()
    wheel.d.ellipse([(cx - r) * SS, (cy - r) * SS, (cx + r) * SS, (cy + r) * SS], outline=rgba((0x9A, 0xE0, 0xFF), 255), width=5 * SS)
    for k in range(24):
        ang = k * math.tau / 24
        col = [(0xFF, 0x6A, 0x9A), (0x6A, 0xE0, 0xFF), (0xFF, 0xD8, 0x5A)][k % 3]
        wheel.line([(cx, cy), (cx + r * math.cos(ang), cy + r * math.sin(ang))], rgba(col, 200), 1.5)
        wheel.circle(cx + r * math.cos(ang), cy + r * math.sin(ang), 7, rgba(col, 255))
    glow = Art(a.w, a.h)
    glow.img = wheel.img.copy()
    a.paste(glow, blur=8)
    a.paste(wheel)
    a.line([(cx, cy), (cx - 60, hz), (cx + 60, hz), (cx, cy)], (0x50, 0x5A, 0x80), 5)
    # レンガ倉庫
    for k in range(2):
        x = 60 + k * 230
        a.rect(x, hz - 90, x + 200, hz, (0x9A, 0x50, 0x3E))
        a.rect(x - 4, hz - 96, x + 204, hz - 88, (0x6A, 0x3A, 0x30))
        window_grid(a, x + 10, hz - 80, x + 190, hz - 10, 8, 2, (0x6A, 0x3A, 0x30), (0x3A, 0x2A, 0x30), (0xFF, 0xD0, 0x80), 0.7, rng)
    water(a, hz, a.h, time, rng, base_col=(0x1E, 0x2E, 0x58))
    reflect(a, hz, 300, blur=4, opacity=0.45)
    # 岸壁と手すり
    a.rect(0, a.h - 60, a.w, a.h, (0x3A, 0x3E, 0x52))
    for x in range(0, a.w, 30):
        a.rect(x, a.h - 84, x + 3, a.h - 60, (0x8A, 0x90, 0xA8))
    a.line([(0, a.h - 84), (a.w, a.h - 84)], (0x8A, 0x90, 0xA8), 3)
    for x in (300, 900):
        lamp(a, x, a.h - 60, time)


def scene_hillside(a, s, rng):
    time, season = s["time"], s["season"]
    hz = 380
    backdrop(a, time, rng, hz, west=True, city=0.1)
    # なだらかな丘の住宅地
    a.ellipse(-300, hz - 20, 900, hz + 300, (0x7A, 0x96, 0x5E))
    a.ellipse(500, hz, 1600, hz + 320, (0x6E, 0x8E, 0x56))
    a.rect(0, hz + 140, a.w, a.h, (0x6E, 0x8E, 0x56))
    for k in range(12):
        x = k * 110 - 20
        y = hz + 90 + 30 * math.sin(k * 0.8)
        house(a, x, y, 80, rng.choice([(0xF2, 0xEC, 0xE0), (0xE8, 0xDE, 0xCC), (0xF8, 0xF4, 0xEE)]),
              rng.choice([(0x5A, 0x4A, 0x44), (0x3A, 0x4E, 0x62), (0x8A, 0x5A, 0x3E)]), time, rng)
    # 曲がった並木道
    pts = [(1300, a.h), (900, hz + 300), (700, hz + 200), (640, hz + 150)]
    a.line(pts, (0x9A, 0x96, 0x92), 90)
    a.line(pts, (0xB0, 0xAC, 0xA6), 70)
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        for k in range(3):
            t = k / 3
            x, y = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
            sc = (y - hz) / (a.h - hz)
            tree(a, x - 90 * sc - 20, y, 80 + 220 * sc, season, rng)
            tree(a, x + 90 * sc + 20, y + 10, 80 + 220 * sc, season, rng)
    people(a, 900, 1000, hz + 290, 3, rng, 1.1)


def scene_towers(a, s, rng):
    time = s["time"]
    hz = 470
    sky(a, time, hz, rng)
    skyline(a, hz, time, rng, 80, 180, depth=0.55)
    # タワーマンション群
    for x, w, h in [(40, 150, 420), (230, 130, 360), (400, 170, 450), (800, 140, 400), (980, 170, 440), (1160, 130, 330)]:
        body = rng.choice([(0xD8, 0xDC, 0xE4), (0xC4, 0xCC, 0xD8), (0xE6, 0xE4, 0xDE)])
        building(a, x, hz, w, h, body, time, rng, cols=int(w / 18), rows=int(h / 16))
        for y in range(int(hz - h + 20), hz - 20, 16):
            a.line([(x, y), (x + w * 0.86, y)], rgba(shade(body, 0.8), 160), 1.2)
    a.rect(0, hz, a.w, hz + 30, (0x6E, 0x8E, 0x58))
    sidewalk(a, hz + 30, a.h, col=(0xD4, 0xCE, 0xC4))
    for x in range(60, a.w, 220):
        tree(a, x, hz + 70, 150, s["season"], rng)
    people(a, 80, 1200, hz + 170, 16, rng, 1.2)
    # ベビーカー
    for bx in (420, 860):
        a.rect(bx, hz + 140, bx + 34, hz + 162, (0x3A, 0x5A, 0x8A), r=8)
        a.circle(bx + 6, hz + 168, 6, (0x30, 0x30, 0x34))
        a.circle(bx + 28, hz + 168, 6, (0x30, 0x30, 0x34))
        a.line([(bx + 34, hz + 146), (bx + 46, hz + 128)], (0x30, 0x30, 0x34), 3)


def scene_canal(a, s, rng):
    time = s["time"]
    hz = 430
    sky(a, time, hz, rng)
    skyline(a, hz, time, rng, 100, 220, depth=0.4, windows=True)
    for x, w, h in [(60, 150, 380), (250, 120, 300), (430, 160, 420), (760, 140, 360), (930, 170, 400), (1130, 140, 330)]:
        building(a, x, hz, w, h, (0x7E, 0x9A, 0xBC), time, rng, cols=int(w / 20), rows=int(h / 18))
    a.rect(0, hz - 6, a.w, hz + 10, (0x8A, 0x9A, 0x7A))
    water(a, hz + 10, a.h, time, rng, base_col=(0x3A, 0x5A, 0x86))
    reflect(a, hz + 10, 320, blur=4, opacity=0.4)
    # 橋
    a.poly([(0, hz + 150), (a.w, hz + 110), (a.w, hz + 130), (0, hz + 172)], (0xD8, 0xD8, 0xDE))
    a.poly([(0, hz + 172), (a.w, hz + 130), (a.w, hz + 140), (0, hz + 184)], (0xA8, 0xA8, 0xB4))
    for x in range(0, a.w, 24):
        y = hz + 150 - 40 * x / a.w
        a.line([(x, y), (x, y - 18)], (0xC0, 0xC4, 0xCC), 1.4)
    a.line([(0, hz + 132), (a.w, hz + 92)], (0xC0, 0xC4, 0xCC), 2)
    people(a, 200, 1100, hz + 145, 10, rng, 0.8)
    for x in range(100, a.w, 300):
        lamp(a, x, hz + 150 - 40 * x / a.w, time)


def scene_arcade(a, s, rng):
    time = s["time"]
    hz = 300
    sky(a, time, hz + 100, rng)
    skyline(a, hz + 100, time, rng, 80, 180, depth=0.4)
    # アーケードの屋根と店
    a.rect(0, hz + 100, a.w, a.h, (0x8A, 0x80, 0x76))
    a.poly([(0, hz - 20), (a.w, hz - 20), (a.w * 0.62, hz + 160), (a.w * 0.38, hz + 160)], rgba((0xE8, 0xEC, 0xF0), 150))
    for k in range(10):
        t = k / 9
        y = hz - 20 + 180 * t
        a.line([(a.w * 0.38 * t, y), (a.w - a.w * 0.38 * t, y)], rgba((0x8A, 0x90, 0x9A), 200), 3)
    for side in (-1, 1):
        for k in range(6):
            t0, t1 = k / 6, (k + 1) / 6
            xo0 = a.w * (0.5 + side * (0.5 - 0.38 * t0))
            xo1 = a.w * (0.5 + side * (0.5 - 0.38 * t1))
            y0, y1 = hz - 20 + 180 * t0, hz - 20 + 180 * t1
            col = rng.choice([(0xC8, 0x5A, 0x4A), (0x4A, 0x7A, 0xB8), (0xE8, 0xB8, 0x4A), (0x5A, 0x9A, 0x6A), (0xF0, 0xE8, 0xDA)])
            a.poly([(xo0, y0 + 30), (xo1, y1 + 30), (xo1, a.h - (a.h - hz - 160) * t1 * 0.2 - 120 * (1 - t1)), (xo0, a.h - 120 * (1 - t0))], shade(col, 0.9))
            a.poly([(xo0, y0 + 30), (xo1, y1 + 30), (xo1, y1 + 60), (xo0, y0 + 70)], col)
    a.poly([(a.w * 0.38, hz + 160), (a.w * 0.62, hz + 160), (a.w, a.h), (0, a.h)], (0xC8, 0xC0, 0xB2))
    for k in range(12):
        t = k / 11
        y = hz + 160 + (a.h - hz - 160) * t
        a.line([(a.w * 0.38 * (1 - t), y), (a.w - a.w * 0.38 * (1 - t), y)], rgba((0xA8, 0xA0, 0x92), 150), 1.2)
    people(a, 480, 800, hz + 200, 12, rng, 0.8)
    people(a, 300, 1000, hz + 330, 10, rng, 1.3)
    glow = a.layer()
    for k in range(8):
        glow.circle(a.w * 0.5 + rng.uniform(-300, 300), hz + rng.uniform(80, 200), 40, rgba((0xFF, 0xE0, 0xA0), 90))
    a.paste(glow, blur=20)


def scene_deck(a, s, rng):
    time = s["time"]
    hz = 400
    backdrop(a, time, rng, hz, city=0.5)
    for k, (w, h) in enumerate([(260, 260), (220, 200), (280, 300), (240, 230)]):
        building(a, 20 + k * 320, hz + 60, w, h, rng.choice([(0xB8, 0xAE, 0xA4), (0x9A, 0xA8, 0xB8), (0xC8, 0xBE, 0xB0)]), time, rng)
    road(a, hz + 60, a.h, time, crosswalk=False)
    # 二層の歩行者デッキ
    deck_y = hz + 160
    a.rect(-10, deck_y, a.w + 10, deck_y + 24, (0xD8, 0xD2, 0xC6))
    a.rect(-10, deck_y + 24, a.w + 10, deck_y + 34, (0x9A, 0x94, 0x8A))
    for x in range(60, a.w, 200):
        a.rect(x, deck_y + 34, x + 26, a.h, (0xB8, 0xB2, 0xA6))
        a.rect(x + 20, deck_y + 34, x + 26, a.h, (0x9A, 0x94, 0x8A))
    a.rect(-10, deck_y - 30, a.w + 10, deck_y - 26, (0xA8, 0xB0, 0xB8))
    for x in range(0, a.w, 16):
        a.line([(x, deck_y - 26), (x, deck_y)], rgba((0xA8, 0xB0, 0xB8), 200), 1.2)
    people(a, 40, 1240, deck_y, 22, rng, 1.0)
    for x in (300, 900):
        lamp(a, x, deck_y - 28, time)
        tree(a, x + 150, deck_y - 2, 120, s["season"], rng)


SCENES = {
    "park": scene_park,
    "residential": scene_residential,
    "plaza": scene_plaza,
    "avenue": scene_avenue,
    "river": scene_river,
    "schoolzone": scene_schoolzone,
    "railway": scene_railway,
    "lawn": scene_lawn,
    "scramble": scene_scramble,
    "slope": scene_slope,
    "port": scene_port,
    "hillside": scene_hillside,
    "towers": scene_towers,
    "canal": scene_canal,
    "arcade": scene_arcade,
    "deck": scene_deck,
}


def town_spec(t):
    slug, name, kana, region, scene, time, season, school = t
    return dict(slug=slug, name=name, kana=kana, region=region, scene=scene, time=time, season=season, school=school)


def town_image(t):
    s = town_spec(t)
    rng = random.Random(s["slug"])
    a = Art(W, H)
    SCENES[s["scene"]](a, s, rng)
    vignette(a)
    station_sign(a, s["name"], s["kana"], s["region"])
    return a.finish()


# ---------------------------------------------------------------------------
# 校舎
# ---------------------------------------------------------------------------

SCHOOL_SPECS = {
    # 幅, 高さ, 外壁, 窓の列, 窓の段
    "zakkyo": (380, 420, (0xD8, 0xC8, 0xB0), 5, 5),
    "white": (560, 320, (0xF4, 0xF0, 0xE8), 8, 2),
    "office": (500, 420, (0xB4, 0xBE, 0xC8), 7, 5),
    "wood": (540, 320, (0xB0, 0x82, 0x5A), 7, 2),
    "glass": (480, 470, (0x8A, 0xB0, 0xD0), 6, 6),
    "brick": (520, 370, (0xB0, 0x60, 0x48), 7, 3),
    "suburb": (600, 310, (0xEE, 0xE6, 0xD4), 8, 2),
    "tower": (400, 520, (0x6A, 0x7E, 0x9E), 5, 7),
    "stone": (540, 370, (0xD4, 0xCC, 0xBC), 7, 3),
    "podium": (620, 320, (0xE2, 0xDE, 0xD6), 8, 2),
}


def school_logo(a, x, y, r):
    """塾のマーク（開いた本と星）。"""
    a.circle(x, y, r, (0xFF, 0xF8, 0xEC))
    a.poly([(x - r * 0.62, y - r * 0.2), (x, y + r * 0.05), (x, y + r * 0.55), (x - r * 0.62, y + r * 0.3)], ORANGE)
    a.poly([(x + r * 0.62, y - r * 0.2), (x, y + r * 0.05), (x, y + r * 0.55), (x + r * 0.62, y + r * 0.3)], shade(ORANGE, 0.85))
    pts = []
    for k in range(10):
        ang = -math.pi / 2 + k * math.pi / 5
        rr = r * (0.34 if k % 2 == 0 else 0.14)
        pts.append((x + rr * math.cos(ang), y - r * 0.38 + rr * math.sin(ang)))
    a.poly(pts, (0x4F, 0x8C, 0xF0))


def school_building(a, s, rng, base):
    time = s["time"]
    bw, bh, body, cols, rows = SCHOOL_SPECS[s["school"]]
    x0 = (a.w - bw) / 2 + 40
    lit = (0xFF, 0xD6, 0x82)
    glass = mix(shade(body, 0.5), (0x9C, 0xC4, 0xE0) if time in ("day", "afternoon") else (0x30, 0x3A, 0x60), 0.55)
    if s["school"] == "podium":  # タワーマンションの低層階
        building(a, x0 + 60, base - bh - 420, bw - 120, 430, (0xC8, 0xCE, 0xD8), time, rng, cols=12, rows=18)
    # 影と本体
    sh = a.layer()
    sh.poly([(x0 + bw, base), (x0 + bw + 50, base), (x0 + bw + 50, base - bh * 0.6), (x0 + bw, base - bh * 0.7)], rgba((0, 0, 0), 70))
    a.paste(sh, blur=10)
    a.rect(x0, base - bh, x0 + bw, base, body)
    a.rect(x0 + bw - 30, base - bh, x0 + bw, base, shade(body, 0.85))
    a.rect(x0 - 6, base - bh - 10, x0 + bw + 6, base - bh, shade(body, 1.1))
    if s["school"] == "wood":
        for x in np.arange(x0, x0 + bw - 30, 12):
            a.line([(x, base - bh), (x, base)], rgba(shade(body, 0.85), 150), 1)
    if s["school"] == "brick":
        for y in np.arange(base - bh + 8, base, 10):
            a.line([(x0, y), (x0 + bw - 30, y)], rgba(shade(body, 0.85), 160), 1)
    # 看板（会社名と校名）
    sign_y = base - bh + 18
    a.rect(x0 + 16, sign_y, x0 + bw - 46, sign_y + 62, ORANGE, r=6)
    a.rect(x0 + 16, sign_y + 52, x0 + bw - 46, sign_y + 62, shade(ORANGE, 0.85))
    school_logo(a, x0 + 54, sign_y + 30, 24)
    a.text(x0 + 90, sign_y + 17, COMPANY, 15, (0x3A, 0x26, 0x10), anchor="lm")
    name = f"{s['name']}校"
    a.text(x0 + 90, sign_y + 38, name, min(30, (bw - 160) / len(name)), (0x1E, 0x14, 0x08), anchor="lm")
    # 窓（教室の明かりと机）
    top = sign_y + 80
    bottom = base - 120
    cw = (bw - 60) / cols
    rh = (bottom - top) / rows
    p_lit = 0.85 if time in ("dusk", "night") else 0.55
    for r in range(rows):
        for c in range(cols):
            wx0, wy0 = x0 + 14 + c * cw + 5, top + r * rh + 5
            wx1, wy1 = wx0 + cw - 10, wy0 + rh - 12
            a.rect(wx0 - 2, wy0 - 2, wx1 + 2, wy1 + 2, shade(body, 0.7))
            if rng.random() < p_lit:
                a.rect(wx0, wy0, wx1, wy1, lit)
                # 机に向かう生徒の影
                for k in range(2):
                    px = wx0 + (wx1 - wx0) * (0.3 + 0.4 * k)
                    a.circle(px, wy1 - (wy1 - wy0) * 0.45, (wy1 - wy0) * 0.11, rgba((0x6A, 0x50, 0x3A), 170))
                    a.rect(px - (wy1 - wy0) * 0.14, wy1 - (wy1 - wy0) * 0.34, px + (wy1 - wy0) * 0.14, wy1, rgba((0x6A, 0x50, 0x3A), 170))
                a.rect(wx0, wy1 - (wy1 - wy0) * 0.12, wx1, wy1, rgba((0x9A, 0x6A, 0x3A), 200))
            else:
                a.rect(wx0, wy0, wx1, wy1, glass)
                a.poly([(wx0, wy1), (wx0, wy0 + (wy1 - wy0) * 0.5), (wx0 + (wx1 - wx0) * 0.6, wy0)], rgba((255, 255, 255), 45))
    # 1 階：入口と掲示
    a.rect(x0, base - 110, x0 + bw - 30, base - 104, shade(body, 0.8))
    dx = x0 + bw / 2 - 70
    a.rect(dx, base - 100, dx + 120, base, (0x3A, 0x3E, 0x46))
    a.rect(dx + 6, base - 94, dx + 57, base, (0xFF, 0xE4, 0xA8) if time in ("dusk", "night") else (0xA8, 0xC8, 0xDC))
    a.rect(dx + 63, base - 94, dx + 114, base, (0xFF, 0xE4, 0xA8) if time in ("dusk", "night") else (0xA8, 0xC8, 0xDC))
    a.rect(dx - 20, base - 112, dx + 140, base - 100, ORANGE)
    for bx in (x0 + 30, x0 + 110):
        a.rect(bx, base - 90, bx + 60, base - 30, (0xF8, 0xF4, 0xEA), r=3)
        a.rect(bx + 6, base - 84, bx + 54, base - 72, rgba(ORANGE, 220))
        for k in range(3):
            a.line([(bx + 8, base - 62 + k * 10), (bx + 52, base - 62 + k * 10)], (0xB0, 0xA8, 0x98), 2)
    # のぼり旗
    for fx in (x0 - 40, x0 + bw + 10):
        a.rect(fx, base - 170, fx + 3, base, (0x80, 0x80, 0x88))
        a.rect(fx + 3, base - 166, fx + 30, base - 60, ORANGE)
        a.rect(fx + 3, base - 166, fx + 30, base - 156, (0xFF, 0xF4, 0xE0))
    # 自転車
    nbikes = max(1, min(4, int((x0 + bw - 40 - (dx + 160)) / 40)))
    for k in range(nbikes):
        bx = dx + 160 + k * 40
        col = rng.choice([(0xC8, 0x3A, 0x3A), (0x3A, 0x6A, 0xC8), (0xE8, 0xC8, 0x3A), (0x40, 0x40, 0x48)])
        a.d.ellipse([(bx - 10) * SS, (base - 22) * SS, (bx + 10) * SS, (base - 2) * SS], outline=(0x30, 0x30, 0x34), width=2 * SS)
        a.d.ellipse([(bx + 16) * SS, (base - 22) * SS, (bx + 36) * SS, (base - 2) * SS], outline=(0x30, 0x30, 0x34), width=2 * SS)
        a.line([(bx, base - 12), (bx + 12, base - 26), (bx + 26, base - 12), (bx, base - 12)], col, 2.5)
    # 植え込み
    a.rect(x0 - 60, base - 18, x0 - 2, base, (0x4A, 0x7A, 0x44), r=8)
    a.rect(x0 + bw + 2, base - 18, x0 + bw + 70, base, (0x4A, 0x7A, 0x44), r=8)


def school_image(t):
    s = town_spec(t)
    rng = random.Random(s["slug"] + "/school")
    a = Art(W, H)
    base = H - 60
    hz = 470
    sky(a, s["time"], hz, rng)
    skyline(a, hz, s["time"], rng, 120, 260, depth=0.5, windows=s["time"] in ("dusk", "night"))
    haze(a, hz - 30, 80, s["time"], alpha=100)
    skyline(a, base - 40, s["time"], rng, 160, 320, 80, 160, depth=0.25, windows=s["time"] in ("dusk", "night"))
    for x in (60, 200, 1100, 1230):
        tree(a, x, base - 10, rng.uniform(150, 210), s["season"], rng)
    sidewalk(a, base - 40, H, col=(0xCC, 0xC4, 0xB6))
    school_building(a, s, rng, base)
    lamp(a, 150, base, s["time"])
    people(a, 260, 380, base + 18, 3, rng, 1.2)
    vignette(a)
    return a.finish()


# ---------------------------------------------------------------------------
# 首都圏の地図
# ---------------------------------------------------------------------------

MAP_W, MAP_H = 2048, 1792
LON0, LON1 = 139.3672, 140.0828
LAT0, LAT1 = 35.43, 35.94
MSS = 2

TOKYO_BAY = [
    (139.66, 35.15), (139.67, 35.28), (139.64, 35.34), (139.63, 35.40), (139.67, 35.42), (139.64, 35.465),
    (139.68, 35.49), (139.74, 35.52), (139.80, 35.545), (139.79, 35.565), (139.75, 35.59), (139.76, 35.62),
    (139.78, 35.635), (139.83, 35.63), (139.86, 35.635), (139.88, 35.625), (139.91, 35.64), (139.95, 35.66),
    (139.98, 35.685), (140.02, 35.665), (140.05, 35.64), (140.09, 35.605), (140.12, 35.57), (140.10, 35.52),
    (140.00, 35.45), (139.92, 35.38), (139.82, 35.30), (139.80, 35.15),
]
RIVERS = [
    [(139.77, 35.54), (139.70, 35.565), (139.67, 35.58), (139.63, 35.607), (139.57, 35.625), (139.53, 35.645),
     (139.47, 35.657), (139.42, 35.68), (139.36, 35.70), (139.30, 35.73)],
    [(139.84, 35.645), (139.84, 35.70), (139.81, 35.75), (139.73, 35.785), (139.65, 35.825), (139.58, 35.88),
     (139.53, 35.94), (139.48, 36.00)],
    [(139.90, 35.64), (139.89, 35.70), (139.88, 35.76), (139.88, 35.80), (139.90, 35.86), (139.88, 35.94),
     (139.83, 36.02)],
]
BORDERS = [
    [(139.20, 35.86), (139.35, 35.80), (139.45, 35.775), (139.55, 35.78), (139.63, 35.795), (139.72, 35.80),
     (139.80, 35.79), (139.88, 35.78)],
    [(139.53, 35.64), (139.50, 35.61), (139.48, 35.575), (139.45, 35.555), (139.41, 35.55), (139.34, 35.60),
     (139.26, 35.62)],
]
# 市街地が鉄道沿いに広がる（townmap.rs の RAILWAYS と同じ駅）
RAILWAYS = [
    [(139.767, 35.681), (139.777, 35.714), (139.761, 35.738), (139.711, 35.730), (139.700, 35.690), (139.702, 35.658), (139.739, 35.628), (139.757, 35.655), (139.767, 35.681)],
    [(139.767, 35.681), (139.700, 35.690), (139.620, 35.704), (139.580, 35.703), (139.480, 35.700), (139.414, 35.698), (139.339, 35.656)],
    [(139.700, 35.690), (139.650, 35.668), (139.541, 35.652), (139.480, 35.672), (139.447, 35.650), (139.343, 35.657)],
    [(139.700, 35.690), (139.668, 35.661), (139.599, 35.640), (139.570, 35.620), (139.508, 35.604), (139.446, 35.542), (139.438, 35.532)],
    [(139.702, 35.658), (139.670, 35.644), (139.627, 35.611), (139.611, 35.600), (139.558, 35.578), (139.495, 35.531)],
    [(139.702, 35.658), (139.669, 35.607), (139.660, 35.576), (139.631, 35.509), (139.622, 35.466)],
    [(139.622, 35.466), (139.697, 35.531), (139.739, 35.628), (139.767, 35.681), (139.777, 35.714), (139.721, 35.778), (139.657, 35.859), (139.624, 35.906)],
    [(139.711, 35.730), (139.654, 35.738), (139.584, 35.751), (139.473, 35.786)],
    [(139.767, 35.681), (139.814, 35.697), (139.907, 35.728), (139.985, 35.702), (140.113, 35.613)],
    [(139.777, 35.714), (139.805, 35.749), (139.900, 35.784), (139.971, 35.862)],
    [(139.697, 35.531), (139.660, 35.576), (139.570, 35.620), (139.477, 35.666), (139.414, 35.698)],
]
HILLS = [(139.40, 35.60, 0.10, 0.05), (139.44, 35.775, 0.07, 0.03), (139.38, 35.68, 0.05, 0.04), (139.52, 35.585, 0.06, 0.03), (140.02, 35.86, 0.06, 0.04)]
PARKS = [(139.405, 35.705, 0.012), (139.472, 35.800, 0.010), (139.694, 35.672, 0.006), (139.711, 35.685, 0.004), (139.575, 35.700, 0.005), (139.773, 35.716, 0.004), (139.655, 35.860, 0.005)]


def mp(lon, lat):
    return ((lon - LON0) / (LON1 - LON0) * MAP_W * MSS, (LAT1 - lat) / (LAT1 - LAT0) * MAP_H * MSS)


def blurred(img, r):
    return img.filter(ImageFilter.GaussianBlur(r * MSS))


def map_image():
    rng = random.Random("shutoken")
    w, h = MAP_W * MSS, MAP_H * MSS
    # 陸：ゆるい明暗と細かいむら
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    base = np.array([0x48, 0x6A, 0x50], np.float32)
    shadeg = 1.0 + 0.10 * (yy / h) - 0.06 * (xx / w)
    noise = np.array(Image.effect_noise((w // 16, h // 16), 30).resize((w, h), Image.BICUBIC), np.float32) / 255.0 - 0.5
    land = np.clip(base[None, None, :] * (shadeg[..., None] + 0.08 * noise[..., None]), 0, 255).astype(np.uint8)
    img = Image.fromarray(land, "RGB").convert("RGBA")
    d = ImageDraw.Draw(img, "RGBA")

    # 丘陵（濃い緑）と公園
    hills = Image.new("RGBA", (w, h))
    hd = ImageDraw.Draw(hills)
    for lon, lat, rx, ry in HILLS:
        x, y = mp(lon, lat)
        x1, y1 = mp(lon + rx, lat - ry)
        dx, dy = x1 - x, y1 - y
        hd.ellipse([x - dx, y - dy, x + dx, y + dy], fill=(0x24, 0x44, 0x30, 190))
    img.alpha_composite(blurred(hills, 60))

    # 市街地：鉄道沿いと都心に、小さな街区を散らす
    city = Image.new("RGBA", (w, h))
    cd = ImageDraw.Draw(city)
    pts = []
    for line in RAILWAYS:
        for (a0, b0), (a1, b1) in zip(line, line[1:]):
            for k in range(14):
                t = k / 14
                pts.append((a0 + (a1 - a0) * t, b0 + (b1 - b0) * t, 0.018))
    for _ in range(500):
        pts.append((rng.gauss(139.74, 0.06), rng.gauss(35.69, 0.045), 0.0))
    for lon, lat, spread in pts:
        for _ in range(6 if spread else 1):
            x, y = mp(lon + rng.gauss(0, spread), lat + rng.gauss(0, spread * 0.8))
            s = rng.uniform(10, 26) * MSS
            c = rng.choice([(0x8E, 0x8C, 0x7C), (0x96, 0x90, 0x80), (0x84, 0x88, 0x7C)])
            cd.ellipse([x - s, y - s * 0.8, x + s, y + s * 0.8], fill=c + (60,))
    img.alpha_composite(blurred(city, 6))
    glow = Image.new("RGBA", (w, h))
    gd = ImageDraw.Draw(glow)
    x, y = mp(139.74, 35.69)
    gd.ellipse([x - 420 * MSS, y - 360 * MSS, x + 420 * MSS, y + 360 * MSS], fill=(0x9A, 0x96, 0x80, 60))
    img.alpha_composite(blurred(glow, 120))

    for lon, lat, r in PARKS:
        x, y = mp(lon, lat)
        x1, _ = mp(lon + r, lat)
        rr = x1 - x
        d.ellipse([x - rr, y - rr * 0.8, x + rr, y + rr * 0.8], fill=(0x2C, 0x5A, 0x36, 200))

    # 海：岸に近いほど浅い色
    sea_mask = Image.new("L", (w, h))
    ImageDraw.Draw(sea_mask).polygon([mp(*p) for p in TOKYO_BAY], fill=255)
    deep = Image.new("RGBA", (w, h), (0x16, 0x34, 0x52, 255))
    shallow = Image.new("L", (w, h))
    ImageDraw.Draw(shallow).line([mp(*p) for p in TOKYO_BAY], fill=255, width=90 * MSS, joint="curve")
    shallow = ImageChops.multiply(blurred(shallow, 60), sea_mask)
    shallow_img = Image.new("RGBA", (w, h), (0x2E, 0x62, 0x86, 255))
    deep.paste(shallow_img, (0, 0), shallow)
    img.paste(deep, (0, 0), sea_mask)
    # 波打ち際
    d.line([mp(*p) for p in TOKYO_BAY], fill=(0x8C, 0xBC, 0xD4, 200), width=5 * MSS, joint="curve")
    d.line([mp(*p) for p in TOKYO_BAY], fill=(0xD8, 0xCC, 0xA0, 120), width=2 * MSS, joint="curve")
    # 海の波
    for _ in range(260):
        lon, lat = rng.uniform(139.75, 140.1), rng.uniform(35.43, 35.65)
        x, y = mp(lon, lat)
        if sea_mask.getpixel((int(min(max(x, 0), w - 1)), int(min(max(y, 0), h - 1)))) > 200:
            L = rng.uniform(20, 60) * MSS
            d.arc([x, y, x + L, y + L * 0.4], 200, 340, fill=(0x6A, 0x9A, 0xBC, 110), width=2 * MSS)

    # 川（岸の明るい色と水）
    for r in RIVERS:
        pts = [mp(*p) for p in r]
        d.line(pts, fill=(0x6E, 0x8E, 0x6A, 255), width=26 * MSS, joint="curve")
        d.line(pts, fill=(0x38, 0x70, 0x94, 255), width=14 * MSS, joint="curve")
        d.line(pts, fill=(0x5C, 0x98, 0xBC, 160), width=4 * MSS, joint="curve")
    # 都県境
    for b in BORDERS:
        pts = [mp(*p) for p in b]
        for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
            n = max(1, int(math.hypot(x1 - x0, y1 - y0) / (22 * MSS)))
            for k in range(0, n, 2):
                t0, t1 = k / n, min(1, (k + 1) / n)
                d.line([x0 + (x1 - x0) * t0, y0 + (y1 - y0) * t0, x0 + (x1 - x0) * t1, y0 + (y1 - y0) * t1],
                       fill=(255, 255, 255, 80), width=4 * MSS)
    # 周辺を少し暗く
    vig = np.ones((h // 8, w // 8), np.float32)
    vy, vx = np.mgrid[0:h // 8, 0:w // 8]
    dist = np.sqrt(((vx / (w / 8) - 0.5) / 0.75) ** 2 + ((vy / (h / 8) - 0.5) / 0.75) ** 2)
    vig = np.clip(1.0 - 0.35 * np.clip(dist - 0.5, 0, 1), 0, 1)
    vmask = Image.fromarray((255 * (1 - vig)).astype(np.uint8), "L").resize((w, h), Image.BICUBIC)
    dark = Image.new("RGBA", (w, h), (0x10, 0x1A, 0x16, 255))
    img.paste(dark, (0, 0), vmask)
    return img.resize((MAP_W, MAP_H), Image.LANCZOS).convert("RGB")


# ---------------------------------------------------------------------------


def save(img, rel):
    path = ROOT / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path, optimize=True)
    print(f"{rel}  ({path.stat().st_size // 1024} KB)")


def main():
    only = sys.argv[1:]

    def wanted(key):
        return not only or any(o in key for o in only)

    if wanted("map"):
        save(map_image(), "assets/images/map/shutoken_map.png")
    for t in TOWNS:
        if wanted(t[0]):
            save(town_image(t), f"assets/images/towns/town_{t[0]}.png")
            save(school_image(t), f"assets/images/schools/school_{t[0]}.png")


if __name__ == "__main__":
    main()
