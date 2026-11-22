# 合格の街 〜塾長シミュレーター〜

中学受験塾の経営シミュレーションゲーム。Rust で書かれており、**Windows アプリ**と **WebAssembly（ブラウザ）** の両方で動きます。
UI は既存の UI フレームワークを使わず、`winit`（ウィンドウ・入力）と `wgpu`（GPU 描画）の上に自作しています。

- 企画・設計：[`docs/proposal/`](docs/proposal/README.md)

## 遊び方

1 教室の塾長として、毎週「次の週へ」を押して経営を進めます。

| タブ | できること |
|------|-----------|
| 教室 | 保護者の口コミ（4 軸評価）、気になる生徒、教室長の能力・年俸・業務委任、講師の採用と退職 |
| コース | 在籍数の推移、宿題の量（学力とストレスのトレードオフ）、コースの開講と料金（粗利つき） |
| 集客 | 地域の生徒シェアと競合塾、ブランド力・認知度、8 種類の集客施策 |
| 経理 | 売上・純利益の推移、週次の損益計算書（前週比つき）、企業価値 |
| 年度 | 年間カレンダー、過去の入試結果、保存 |

毎年 2 月 1 日に入試があり、6 年生の合否が「ブランド力」として翌年の集客を左右します。
途中で倫理的な判断を迫るイベント（宿題の増量要望、志望校を下げさせるか、合格実績の上積みなど）が起こります。

## 構成

```
crates/
  juku-sim/       ゲームロジック（描画・OS 非依存、決定論的、テストつき）
  juku-render/    wgpu の 2D バッチレンダラ（SDF 図形・グリフアトラス）
  juku-ui/        自作の即時モード UI ツールキット（レイアウト・ウィジェット・グラフ）
  juku-platform/  Windows / Web の差分（ウィンドウ・入力・GPU 初期化・保存）
  juku-app/       画面とゲーム進行
bins/
  juku-desktop/   Windows 版のエントリ（フォントを exe に埋め込み）
  juku-web/       Web 版のエントリ（cdylib）
web/              index.html と Cloudflare 用 _headers
assets/fonts/     Noto Sans JP（JIS 第 1・第 2 水準にサブセット化、SIL OFL）
tools/            Web ビルド・フォントのサブセット化
```

## 開発

```sh
# テスト（主にシミュレーション）
cargo test --workspace

# バランス確認：ヘッドレスで 3 年 × 5 シードを回す
cargo run -p juku-sim --example balance --release -- 3 5

# ネイティブで起動（Windows では exe ができる）
cargo run -p juku-desktop --release
```

### Windows 版

```sh
cargo build -p juku-desktop --release --target x86_64-pc-windows-msvc
# => target/x86_64-pc-windows-msvc/release/yyshop.exe（単体で動く）
```

セーブデータは `%APPDATA%\YYshop\saves\` に保存されます。

### Web 版

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked   # Cargo.lock と同じ版
./tools/build-web.sh                                        # => dist/
python3 -m http.server -d dist 8080                         # http://localhost:8080
```

`wasm-opt`（binaryen）が入っていれば自動でサイズを最適化します。
描画は WebGPU が使えればそれを、使えなければ WebGL2 を使います。セーブデータは localStorage に保存されます。

### Cloudflare へのデプロイ

`dist/` は静的ファイルだけなので、Cloudflare Workers の静的アセット配信（または Pages）にそのまま置けます。

```sh
./tools/build-web.sh
npx wrangler deploy            # wrangler.toml の設定で dist/ を配信
# Pages の場合: npx wrangler pages deploy dist --project-name yyshop
```

GitHub Actions（`.github/workflows/ci.yml`）は main へのマージ時に自動でデプロイします。
リポジトリのシークレットに `CLOUDFLARE_API_TOKEN` と `CLOUDFLARE_ACCOUNT_ID` を設定してください（未設定ならデプロイはスキップされます）。

## ライセンス

- コード：MIT OR Apache-2.0
- フォント：Noto Sans JP — SIL Open Font License 1.1（`assets/fonts/OFL.txt`）
- 作中の塾名・学校名・人名はすべて架空です。
