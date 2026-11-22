#!/usr/bin/env python3
"""仮の画像（地図・街・校舎）を作る。本番の画像ができたら、同じパスに上書きする。

    python3 tools/gen_placeholder_images.py            # まだ無い画像だけ作る
    python3 tools/gen_placeholder_images.py --force    # すべて作り直す

必要: Pillow。一覧は docs/images/README.md と同じ。
"""

import math
import random
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONT_BOLD = ROOT / "assets/fonts/NotoSansJP-Bold.subset.otf"
FONT_REG = ROOT / "assets/fonts/NotoSansJP-Regular.subset.otf"

# (slug, 名前, 読み, 地方, 街の景色の型, 校舎の型)
TOWNS = [
    ("kichijoji", "吉祥寺", "きちじょうじ", "東京西部", "park", "zakkyo"),
    ("seijogakuenmae", "成城学園前", "せいじょうがくえんまえ", "東京西部", "residential", "white"),
    ("chofu", "調布", "ちょうふ", "東京西部", "city", "office"),
    ("fuchu", "府中", "ふちゅう", "東京西部", "avenue", "wood"),
    ("futakotamagawa", "二子玉川", "ふたこたまがわ", "東京西部", "river", "glass"),
    ("urawa", "浦和", "うらわ", "埼玉", "residential", "brick"),
    ("omiya", "大宮", "おおみや", "埼玉", "city", "office"),
    ("tokorozawa", "所沢", "ところざわ", "埼玉", "park", "suburb"),
    ("shibuya", "渋谷", "しぶや", "都心", "downtown", "tower"),
    ("myogadani", "茗荷谷", "みょうがだに", "都心", "avenue", "stone"),
    ("yokohama", "横浜", "よこはま", "神奈川", "port", "office"),
    ("tamaplaza", "たまプラーザ", "たまぷらーざ", "神奈川", "residential", "white"),
    ("musashikosugi", "武蔵小杉", "むさしこすぎ", "神奈川", "towers", "podium"),
    ("toyosu", "豊洲", "とよす", "湾岸・千葉", "canal", "glass"),
    ("funabashi", "船橋", "ふなばし", "湾岸・千葉", "city", "zakkyo"),
    ("kashiwa", "柏", "かしわ", "湾岸・千葉", "deck", "office"),
]

# 地方ごとの空の色（上・下）と差し色
SKY = {
    "東京西部": ((0x6C, 0x9B, 0xC8), (0xF2, 0xC9, 0x8E)),
    "埼玉": ((0x4E, 0x8F, 0xD0), (0xC8, 0xE0, 0xEE)),
    "都心": ((0x3A, 0x2E, 0x5E), (0xE8, 0x8A, 0x5C)),
    "神奈川": ((0x2E, 0x6E, 0xB0), (0xA8, 0xD8, 0xE8)),
    "湾岸・千葉": ((0x23, 0x4E, 0x70), (0xF0, 0xA8, 0x78)),
}

ORANGE = (0xE3, 0x9B, 0x3A)
W, H = 1280, 720

# 地図の範囲（ゲームの townmap.rs の MAP_BOUNDS と同じ）
MAP_W, MAP_H = 2048, 1792
LON0, LON1 = 139.3672, 140.0828
LAT0, LAT1 = 35.43, 35.94


def font(size, bold=True):
    return ImageFont.truetype(str(FONT_BOLD if bold else FONT_REG), size)


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def shade(c, k):
    return tuple(max(0, min(255, int(v * k))) for v in c)


def sky(d, top, bottom, h, bands=14):
    for i in range(bands):
        y0, y1 = h * i // bands, h * (i + 1) // bands + 1
        d.rectangle([0, y0, W, y1], fill=lerp(top, bottom, i / (bands - 1)))


def tree(d, x, y, r, c):
    d.rectangle([x - r * 0.12, y, x + r * 0.12, y + r * 1.1], fill=shade(c, 0.5))
    d.ellipse([x - r, y - r, x + r, y + r * 0.6], fill=c)


def building(d, x, y, w, h, c, lit, rng, win=18):
    d.rectangle([x, y, x + w, y + h], fill=c)
    for wy in range(int(y + 14), int(y + h - 10), win + 8):
        for wx in range(int(x + 10), int(x + w - 14), win + 6):
            col = lit if rng.random() < 0.55 else shade(c, 1.25)
            d.rectangle([wx, wy, wx + win * 0.6, wy + win * 0.7], fill=col)


def house(d, x, y, w, c, roof):
    h = w * 0.7
    d.rectangle([x, y - h, x + w, y], fill=c)
    d.polygon([(x - 8, y - h), (x + w / 2, y - h - w * 0.45), (x + w + 8, y - h)], fill=roof)
    d.rectangle([x + w * 0.2, y - h * 0.6, x + w * 0.42, y - h * 0.3], fill=(0xFF, 0xE6, 0xA8))


def scene(d, kind, rng, region):
    top, bottom = SKY[region]
    sky(d, top, bottom, H)
    horizon = int(H * 0.62)
    far = shade(lerp(top, bottom, 0.6), 0.75)
    # 遠景のビル
    x = -20
    while x < W:
        w = rng.randint(50, 120)
        h = rng.randint(60, 200 if kind in ("downtown", "towers", "canal", "port") else 110)
        d.rectangle([x, horizon - h, x + w, horizon], fill=far)
        x += w + rng.randint(0, 20)
    ground = (0x8A, 0x86, 0x7E) if kind in ("downtown", "towers", "deck", "city") else (0x4F, 0x6E, 0x55)
    lit = (0xFF, 0xD9, 0x8A)
    if kind in ("river", "canal", "port"):
        water = (0x3D, 0x7D, 0xA8) if kind != "canal" else (0x2F, 0x6A, 0x8C)
        d.rectangle([0, horizon, W, H], fill=water)
        for i in range(30):
            yy = rng.randint(horizon + 10, H - 10)
            xx = rng.randint(0, W)
            d.line([xx, yy, xx + rng.randint(30, 90), yy], fill=shade(water, 1.3), width=3)
        if kind == "river":
            d.rectangle([0, H - 110, W, H], fill=(0x7A, 0x9A, 0x5A))
            d.rectangle([0, horizon - 30, W, horizon - 16], fill=(0x8A, 0x8A, 0x90))
            for k in range(0, W, 80):
                d.rectangle([k, horizon - 16, k + 10, horizon + 40], fill=(0x70, 0x70, 0x78))
        if kind == "port":
            cx, cy, r = int(W * 0.78), horizon - 150, 120
            d.ellipse([cx - r, cy - r, cx + r, cy + r], outline=(0xF0, 0xF0, 0xF0), width=6)
            for a in range(12):
                ang = a * math.pi / 6
                d.line([cx, cy, cx + r * math.cos(ang), cy + r * math.sin(ang)], fill=(0xE0, 0xE0, 0xE0), width=2)
            d.rectangle([60, horizon - 70, 420, horizon], fill=(0xA8, 0x55, 0x3E))
        if kind == "canal":
            for k in range(5):
                bx = 120 + k * 230
                building(d, bx, horizon - 330 + k * 20, 150, 330 - k * 20, (0x8F, 0xA8, 0xC0), lit, rng)
            d.rectangle([0, horizon + 70, W, horizon + 84], fill=(0xC8, 0xC8, 0xD0))
        return
    d.rectangle([0, horizon, W, H], fill=ground)
    if kind == "park":
        d.ellipse([W * 0.15, horizon + 60, W * 0.85, H + 120], fill=(0x3D, 0x7D, 0xA8))
        for k in range(14):
            tree(d, rng.randint(0, W), horizon + rng.randint(-10, 40), rng.randint(40, 80), (0x3E, 0x8A, 0x4A))
    elif kind == "residential":
        for k in range(8):
            house(d, 40 + k * 160, horizon + 140, 110, (0xEE, 0xE6, 0xD8), (0x8A, 0x4B, 0x3A))
        for k in range(10):
            tree(d, 20 + k * 135, horizon + 150, 46, (0xD8, 0xB0, 0x3A))
    elif kind == "avenue":
        d.polygon([(W * 0.42, horizon), (W * 0.58, horizon), (W * 0.9, H), (W * 0.1, H)], fill=(0x9A, 0x92, 0x84))
        for k in range(7):
            t = k / 6
            s = 30 + 70 * t
            y = horizon + (H - horizon) * t
            tree(d, W * (0.40 - 0.32 * t), y - s, s, (0x3E, 0x8A, 0x4A))
            tree(d, W * (0.60 + 0.32 * t), y - s, s, (0x3E, 0x8A, 0x4A))
    elif kind in ("downtown", "towers"):
        n = 6 if kind == "towers" else 9
        for k in range(n):
            bw = rng.randint(110, 170)
            bh = rng.randint(300, 470)
            building(d, k * (W // n) + 10, horizon - bh, bw, bh, (0x5E, 0x6E, 0x8A), lit, rng)
        if kind == "downtown":
            for k in range(-6, 7):
                d.line([W / 2 + k * 60, horizon + 20, W / 2 + k * 110, H], fill=(0xE8, 0xE8, 0xE8), width=10)
    elif kind == "deck":
        for k in range(6):
            building(d, k * 220, horizon - 240, 190, 240, (0x8A, 0x86, 0x7E), lit, rng)
        d.rectangle([0, horizon - 40, W, horizon - 10], fill=(0xC8, 0xC2, 0xB0))
        for k in range(0, W, 120):
            d.rectangle([k, horizon - 10, k + 14, H], fill=(0xA8, 0xA2, 0x92))
    else:  # city
        for k in range(8):
            bw = rng.randint(130, 170)
            bh = rng.randint(150, 300)
            building(d, k * 165, horizon - bh, bw, bh, (0x8A, 0x80, 0x76), lit, rng)
        d.rectangle([0, horizon, W, horizon + 60], fill=(0x9A, 0x92, 0x84))


def school(d, kind, rng):
    base = int(H * 0.86)
    lit = (0xFF, 0xD9, 0x8A)
    specs = {
        # (幅, 高さ, 色)
        "zakkyo": (360, 470, (0xB8, 0xA8, 0x92)),
        "white": (520, 260, (0xF2, 0xEE, 0xE6)),
        "office": (480, 470, (0x9A, 0xA4, 0xAE)),
        "wood": (520, 250, (0xA8, 0x7A, 0x52)),
        "glass": (480, 520, (0x7E, 0xA8, 0xC8)),
        "brick": (500, 330, (0xA8, 0x5A, 0x44)),
        "suburb": (560, 240, (0xE8, 0xDE, 0xC8)),
        "tower": (380, 600, (0x56, 0x6E, 0x8E)),
        "stone": (520, 330, (0xC8, 0xC0, 0xB0)),
        "podium": (600, 260, (0xD8, 0xD4, 0xCC)),
    }
    bw, bh, c = specs[kind]
    x0 = (W - bw) // 2
    if kind == "podium":
        building(d, x0 + 80, base - bh - 340, bw - 160, 340, (0x9A, 0xA8, 0xB8), lit, rng)
    building(d, x0, base - bh, bw, bh, c, lit, rng, win=26)
    # オレンジの看板（文字は入れない）と入口
    d.rectangle([x0 + 20, base - bh + 18, x0 + bw - 20, base - bh + 58], fill=ORANGE)
    d.ellipse([x0 + 30, base - bh + 24, x0 + 58, base - bh + 52], fill=(0xFF, 0xF4, 0xE0))
    d.rectangle([x0 + bw / 2 - 45, base - 110, x0 + bw / 2 + 45, base], fill=(0x3A, 0x3A, 0x40))
    d.rectangle([x0 + bw / 2 - 40, base - 104, x0 + bw / 2 + 40, base], fill=(0xFF, 0xE6, 0xA8))
    d.rectangle([0, base, W, H], fill=(0x8A, 0x86, 0x7E))
    for k in range(6):
        tree(d, 60 + k * 240 if k < 3 else W - 60 - (k - 3) * 240, base - 40, 42, (0x3E, 0x8A, 0x4A))


def caption(img, title, sub, badge):
    d = ImageDraw.Draw(img, "RGBA")
    f1, f2, f3 = font(76), font(30, False), font(30)
    tw = d.textlength(title, font=f1)
    sw = d.textlength(sub, font=f2)
    bw = max(tw, sw) + 80
    x0, y0 = (W - bw) / 2, H * 0.30
    d.rounded_rectangle([x0, y0, x0 + bw, y0 + 150], radius=24, fill=(0, 0, 0, 120))
    d.text((W / 2, y0 + 58), title, font=f1, fill=(255, 255, 255), anchor="mm")
    d.text((W / 2, y0 + 118), sub, font=f2, fill=(235, 235, 235), anchor="mm")
    bwid = d.textlength(badge, font=f3) + 36
    d.rounded_rectangle([24, 24, 24 + bwid, 74], radius=25, fill=ORANGE + (235,))
    d.text((24 + bwid / 2, 49), badge, font=f3, fill=(0x1A, 0x12, 0x08), anchor="mm")


def town_image(t):
    slug, name, kana, region, kind, _ = t
    rng = random.Random(slug)
    img = Image.new("RGB", (W, H))
    scene(ImageDraw.Draw(img), kind, rng, region)
    caption(img, name, f"{kana}・{region}", "仮画像")
    return img


def school_image(t):
    slug, name, kana, region, _, kind = t
    rng = random.Random(slug + "school")
    img = Image.new("RGB", (W, H))
    d = ImageDraw.Draw(img)
    top, bottom = SKY[region]
    sky(d, top, bottom, H)
    far = shade(lerp(top, bottom, 0.6), 0.75)
    x = -20
    while x < W:
        w = rng.randint(60, 130)
        h = rng.randint(80, 220)
        d.rectangle([x, int(H * 0.86) - h, x + w, int(H * 0.86)], fill=far)
        x += w + rng.randint(0, 30)
    school(d, kind, rng)
    caption(img, f"{name}校", "校舎", "仮画像")
    return img


# ---- 地図 ----
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


def mp(lon, lat):
    return ((lon - LON0) / (LON1 - LON0) * MAP_W, (LAT1 - lat) / (LAT1 - LAT0) * MAP_H)


def map_image():
    rng = random.Random("map")
    img = Image.new("RGB", (MAP_W, MAP_H), (0x34, 0x55, 0x42))
    d = ImageDraw.Draw(img, "RGBA")
    # 丘陵（西の多摩丘陵・狭山丘陵）は濃い緑、23 区は街の色
    for lon, lat, r in [(139.40, 35.60, 260), (139.45, 35.77, 200), (139.38, 35.68, 220), (139.52, 35.58, 180)]:
        x, y = mp(lon, lat)
        d.ellipse([x - r, y - r * 0.7, x + r, y + r * 0.7], fill=(0x2A, 0x48, 0x34, 200))
    for k in range(400):
        lon = rng.gauss(139.74, 0.07)
        lat = rng.gauss(35.69, 0.05)
        x, y = mp(lon, lat)
        s = rng.randint(6, 18)
        d.rectangle([x, y, x + s, y + s], fill=(0x6A, 0x74, 0x6A, 90))
    d.polygon([mp(*p) for p in TOKYO_BAY], fill=(0x1D, 0x3C, 0x55))
    d.line([mp(*p) for p in TOKYO_BAY] + [mp(*TOKYO_BAY[0])], fill=(0x5B, 0x88, 0xA8), width=4)
    for r in RIVERS:
        d.line([mp(*p) for p in r], fill=(0x3B, 0x6E, 0x8C), width=12, joint="curve")
    for b in BORDERS:
        pts = [mp(*p) for p in b]
        for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
            n = max(1, int(math.hypot(x1 - x0, y1 - y0) / 24))
            for k in range(0, n, 2):
                t0, t1 = k / n, min(1, (k + 1) / n)
                d.line([x0 + (x1 - x0) * t0, y0 + (y1 - y0) * t0, x0 + (x1 - x0) * t1, y0 + (y1 - y0) * t1],
                       fill=(255, 255, 255, 70), width=4)
    # 画面に必ず映る東京湾の中に印を置く
    f = font(44)
    bx, by = mp(139.99, 35.48)
    d.rounded_rectangle([bx - 120, by - 35, bx + 120, by + 35], radius=30, fill=ORANGE + (220,))
    d.text((bx, by), "仮画像", font=f, fill=(0x1A, 0x12, 0x08), anchor="mm")
    return img


def save(img, rel, force):
    path = ROOT / rel
    if path.exists() and not force:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    img = img.quantize(colors=128, method=Image.Quantize.MEDIANCUT).convert("RGB")
    img.save(path, optimize=True)
    print(rel)


def main():
    force = "--force" in sys.argv
    save(map_image(), "assets/images/map/shutoken_map.png", force)
    for t in TOWNS:
        save(town_image(t), f"assets/images/towns/town_{t[0]}.png", force)
        save(school_image(t), f"assets/images/schools/school_{t[0]}.png", force)


if __name__ == "__main__":
    main()
