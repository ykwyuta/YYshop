#!/usr/bin/env bash
# Cloudflare（Workers Builds）用のビルド。wrangler.toml の [build] から、`wrangler deploy` の直前に呼ばれる。
#
# Cloudflare のビルド環境には Rust が入っていないため、無ければ入れてから tools/build-web.sh を実行し、
# dist/ を作る。ローカルや GitHub Actions など、Rust が入っている環境ではそのままビルドするだけ。
set -euo pipefail
cd "$(dirname "$0")/.."

export PATH="$HOME/.cargo/bin:$PATH"

# 1) Rust
if ! command -v cargo >/dev/null 2>&1; then
  echo "== Rust をインストールします =="
  # sh.rustup.rs が配布しているのと同じ公式の rustup-init を直接取得する
  tmp=$(mktemp -d)
  curl --proto '=https' --tlsv1.2 -sSfL -o "$tmp/rustup-init" \
    https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init
  chmod +x "$tmp/rustup-init"
  "$tmp/rustup-init" -y --profile minimal --default-toolchain stable --no-modify-path
  rm -rf "$tmp"
fi
rustup target add wasm32-unknown-unknown

# 2) wasm-bindgen（Cargo.lock と同じ版でないと動かない）
WB_VERSION=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | grep version | cut -d'"' -f2)
if ! command -v wasm-bindgen >/dev/null 2>&1 || [ "$(wasm-bindgen --version | cut -d' ' -f2)" != "$WB_VERSION" ]; then
  echo "== wasm-bindgen $WB_VERSION をインストールします =="
  tmp=$(mktemp -d)
  url="https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$WB_VERSION/wasm-bindgen-$WB_VERSION-x86_64-unknown-linux-musl.tar.gz"
  # ビルド済みのバイナリを使う（速い）。取れなければソースからビルドする
  if curl -sSfL "$url" -o "$tmp/wb.tgz" && tar -xzf "$tmp/wb.tgz" -C "$tmp"; then
    mkdir -p "$HOME/.cargo/bin"
    install -m 755 "$tmp"/wasm-bindgen-*/wasm-bindgen "$HOME/.cargo/bin/wasm-bindgen"
  else
    cargo install wasm-bindgen-cli --version "$WB_VERSION" --locked
  fi
  rm -rf "$tmp"
fi

# 3) ビルド（dist/ を作る）
./tools/build-web.sh
