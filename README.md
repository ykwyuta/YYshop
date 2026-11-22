# 合格の街 〜塾長シミュレーター〜

中学受験塾の経営シミュレーションゲーム。Rust で書かれており、**Windows アプリ**と **WebAssembly（ブラウザ）** の両方で動きます。
UI は既存の UI フレームワークを使わず、`winit`（ウィンドウ・入力）と `wgpu`（GPU 描画）の上に自作しています。

- 企画・設計：[`docs/proposal/`](docs/proposal/README.md)

## 遊び方

最初は 1 教室の塾長として、毎週「次の週へ」を押して経営を進めます。資金が貯まったら別のエリアに教室を増やしていきます。

| タブ | できること |
|------|-----------|
| 教室 | 保護者の口コミ（4 軸評価）、気になる生徒、教室長の能力・年俸・業務委任、講師の採用と退職 |
| コース | 在籍数の推移、宿題の量（学力とストレスのトレードオフ）、コースの開講と料金（粗利つき） |
| 集客 | エリアの生徒シェア、競合塾の動き、ブランド力・認知度、8 種類の集客施策 |
| 経理 | 教室の損益計算書（前週比つき）、会社全体の成績（教室別・本部費用）、企業価値 |
| 本部 | 教室一覧、新しい教室の開校、自社教材の開発（5 軸）、年間カレンダー、入試結果、保存 |

- **入試**：毎年 2 月 1 日に入試があり、全教室の 6 年生の合否が会社の「ブランド力」として翌年の集客を左右します。
- **複数教室**：5 つのエリア（市場規模・家賃・競合がそれぞれ違う）に 1 教室ずつ開けます。教室が増えたら教室長への業務委任が鍵です。
- **自社教材**：市販教材から自社教材に切り替えると、網羅性・難度・わかりやすさ・演習量・入試対応の 5 軸を改訂で鍛えられます。入試対応は毎年古くなります。
- **競合塾**：大手難関型・面倒見型・格安オンライン型・個別指導型の 4 社が 4 週ごとに動き、値下げ・広告・出店・撤退・講師の引き抜きを仕掛けてきます。
- **倫理イベント**：宿題の増量要望、志望校を下げさせるか、合格実績の上積みなど、短期の数字と長期の信頼を天秤にかける判断が起こります。

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
