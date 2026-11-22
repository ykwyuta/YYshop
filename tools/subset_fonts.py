#!/usr/bin/env python3
"""Noto Sans JP を JIS X 0208（第1・第2水準）＋ASCII などにサブセット化する。

使い方:
    pip install fonttools
    python3 tools/subset_fonts.py <NotoSansJP-Regular.otf> <NotoSansJP-Bold.otf>

元フォント: https://github.com/notofonts/noto-cjk (Sans/SubsetOTF/JP) — SIL OFL 1.1
"""
import sys
from pathlib import Path

from fontTools import subset

OUT_DIR = Path(__file__).resolve().parent.parent / "assets" / "fonts"


def charset() -> str:
    chars = set(chr(c) for c in range(0x20, 0x7F))
    # JIS X 0208 全域（EUC-JP の 2 バイト領域）
    for hi in range(0xA1, 0xFF):
        for lo in range(0xA1, 0xFF):
            try:
                chars.add(bytes([hi, lo]).decode("euc_jp"))
            except UnicodeDecodeError:
                pass
    # 半角カナ・よく使う記号
    chars.update(chr(c) for c in range(0xFF61, 0xFFA0))
    chars.update("¥©®°±×÷…‥―‐–—・「」『』【】〜～♪★☆●○◎◆◇■□▲△▼▽→←↑↓∞％＋－")
    return "".join(sorted(chars))


def main() -> None:
    text = charset()
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for src in sys.argv[1:]:
        src = Path(src)
        out = OUT_DIR / (src.stem + ".subset.otf")
        opts = subset.Options()
        opts.layout_features = []
        opts.hinting = False
        opts.name_IDs = ["*"]
        opts.notdef_outline = True
        font = subset.load_font(str(src), opts)
        sub = subset.Subsetter(opts)
        sub.populate(text=text)
        sub.subset(font)
        subset.save_font(font, str(out), opts)
        print(f"{out}: {out.stat().st_size:,} bytes, {len(text)} chars")


if __name__ == "__main__":
    main()
