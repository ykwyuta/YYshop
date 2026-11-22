#!/usr/bin/env bash
# Web 版をビルドして dist/ に静的サイト一式を出力する。
#   必要: rustup target add wasm32-unknown-unknown / cargo install wasm-bindgen-cli（Cargo.lock と同じ版）
#   任意: wasm-opt（binaryen）があればサイズ最適化する
set -euo pipefail
cd "$(dirname "$0")/.."

OUT=dist
PROFILE=release-web
cargo build -p juku-web --target wasm32-unknown-unknown --profile "$PROFILE"

rm -rf "$OUT"
mkdir -p "$OUT/pkg" "$OUT/assets/fonts"
wasm-bindgen --target web --no-typescript --out-dir "$OUT/pkg" \
  "target/wasm32-unknown-unknown/$PROFILE/juku_web.wasm"

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
    "$OUT/pkg/juku_web_bg.wasm" -o "$OUT/pkg/juku_web_bg.wasm"
else
  echo "warning: wasm-opt が見つからないため最適化をスキップします" >&2
fi

# ファイル名に内容のハッシュを付けて長期キャッシュできるようにする
hash() { sha256sum "$1" | cut -c1-10; }
WASM_HASH=$(hash "$OUT/pkg/juku_web_bg.wasm")
WASM_NAME="juku_web_bg.$WASM_HASH.wasm"
mv "$OUT/pkg/juku_web_bg.wasm" "$OUT/pkg/$WASM_NAME"
JS_HASH=$(hash "$OUT/pkg/juku_web.js")
JS_NAME="juku_web.$JS_HASH.js"
mv "$OUT/pkg/juku_web.js" "$OUT/pkg/$JS_NAME"

sed -e "s/__JS__/$JS_NAME/" -e "s/__WASM__/$WASM_NAME/" web/index.html > "$OUT/index.html"
cp web/_headers "$OUT/_headers"
cp assets/fonts/*.subset.otf assets/fonts/OFL.txt "$OUT/assets/fonts/"

# Cloudflare の 1 ファイル上限（25 MiB）に対するサイズチェック
LIMIT=$((25 * 1024 * 1024))
for f in $(find "$OUT" -type f); do
  size=$(stat -c%s "$f")
  if [ "$size" -gt "$LIMIT" ]; then
    echo "error: $f が 25 MiB を超えています（$size bytes）" >&2
    exit 1
  fi
done
echo "== dist =="
find "$OUT" -type f -exec ls -l {} \; | awk '{printf "%10d  %s\n", $5, $9}'
