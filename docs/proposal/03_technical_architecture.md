# 技術設計：Rust によるクロスプラットフォーム実装（Windows / WebAssembly）

> 前提資料：[02_game_proposal.md](./02_game_proposal.md)

## 1. 技術要件

| 要件 | 内容 |
|------|------|
| 言語 | **Rust**（stable） |
| 動作環境 | **Windows ネイティブアプリ**と **WebAssembly（ブラウザ）** を同一コードで動かす |
| 配信 | Web 版は**静的サイトとして Cloudflare から配信**する（サーバー処理なし） |
| UI | **既存の UI フレームワークは使わない**。UI は自作する |

### 1.1 「既存UIフレームワーク不使用」の線引き
UI の部品・レイアウト・入力処理を提供するライブラリは使いません。ウィンドウ・描画・フォントといった低レイヤーのライブラリは、UI フレームワークではないので使います。

| 区分 | ライブラリ | 判断 |
|------|-----------|------|
| 使わない（UIフレームワーク） | egui, iced, Slint, Dioxus, Yew, Leptos, Druid/Xilem, Bevy（UI を含むエンジン）, Tauri（WebView + HTML UI）, gtk-rs, fltk-rs | ✕ |
| 使う（ウィンドウ・入力） | `winit` | ○ |
| 使う（GPU描画） | `wgpu`（Windows は DX12/Vulkan、Web は WebGL2/WebGPU） | ○ |
| 使う（フォント） | `fontdue`（グリフのラスタライズ）、`ttf-parser` | ○ |
| 使う（画像） | `png` / `image`（PNG のデコードのみ） | ○ |
| 使う（Web連携） | `wasm-bindgen`, `web-sys`, `js-sys`, `console_error_panic_hook` | ○ |
| 使う（汎用） | `serde`, `postcard`/`ron`（セーブデータ）, `glam`（ベクトル演算）, `web-time` | ○ |

ボタン、カード、トグル、ステッパー、タブ、スクロール、グラフ、レイアウト計算はすべて自作の `juku-ui` クレートで作ります。

---

## 2. 全体構成

```
┌─────────────────────────────────────────────────────┐
│ juku-app        画面（マップ／教室4タブ／生徒カルテ…）  │
├──────────────────────────┬──────────────────────────┤
│ juku-ui  自作UIツールキット │ juku-sim  シミュレーション  │
│  レイアウト・ウィジェット   │  （描画・OS に一切依存しない）│
│  入力・テーマ・アニメ       │                          │
├──────────────────────────┤                          │
│ juku-render  wgpu 2D描画  │                          │
│  矩形/角丸/円弧/画像/文字   │                          │
├──────────────────────────┴──────────────────────────┤
│ juku-platform   ウィンドウ・入力・時間・保存・乱数シード    │
│   ├ native (Windows)   winit + ファイル保存               │
│   └ web (wasm32)       winit(web) + localStorage/IndexedDB│
└─────────────────────────────────────────────────────┘
```

### 2.1 Cargo ワークスペース

```
yyshop/
├─ Cargo.toml                 # [workspace]
├─ crates/
│  ├─ juku-sim/               # ゲームロジック（no I/O、決定論的）
│  ├─ juku-render/            # wgpu ベースの 2D レンダラ
│  ├─ juku-ui/                # 自作 UI ツールキット
│  ├─ juku-platform/          # native / web の差分を吸収
│  └─ juku-app/               # 画面とゲーム進行
├─ bins/
│  ├─ juku-desktop/           # Windows 用エントリ（main.rs）
│  └─ juku-web/               # wasm 用エントリ（cdylib, #[wasm_bindgen(start)]）
├─ assets/                    # フォント・画像（ビルド時に埋め込み or 配信）
├─ web/                       # index.html, _headers, ローダー JS（数十行）
└─ docs/
```

### 2.2 各クレートの責務

| クレート | 責務 | 依存してよいもの |
|----------|------|------------------|
| `juku-sim` | 生徒・講師・教室・競合・会計・入試判定。週送りは `fn step_week(&mut World, &Commands) -> WeekReport` | `serde` のみ（＋自前の PRNG） |
| `juku-render` | 図形・画像・グリフをまとめて描く 2D バッチレンダラ | `wgpu`, `fontdue`, `glam` |
| `juku-ui` | レイアウト、ウィジェット、ヒットテスト、フォーカス、スクロール、アニメーション、テーマ | `juku-render` |
| `juku-platform` | イベントループ、DPI、時計、保存、クリップボード、IME | `winit`, `web-sys`（wasm時） |
| `juku-app` | 画面遷移、ゲームの状態管理、sim と ui の接続 | 上記すべて |

**`juku-sim` を UI から完全に切り離すこと**が最重要です。これで、
- ネイティブで単体テストとバランス調整用のヘッドレス実行（1000年度分を回すなど）ができる
- Web とネイティブで同じシードなら同じ結果になる（決定論）
- 将来 UI を作り直しても、ゲームロジックはそのまま使える

---

## 3. 自作 UI ツールキット（`juku-ui`）

### 3.1 方式：即時モード API ＋ フレーム間で状態を保持
- 画面コードは毎フレーム `ui.card(|ui| { ... })` のように宣言的に書く（即時モード）
- 内部ではウィジェット ID ごとに状態（スクロール位置、アニメーション、押下中など）を `HashMap<WidgetId, State>` に保持
- 変化がないフレームは再描画しない（`request_redraw` 方式）。モバイルブラウザの電池消費を抑える

### 3.2 レイアウト
参考ゲームの画面はほぼ「縦スクロール ＋ カード ＋ 横並び」なので、Flexbox のサブセットで足ります。
- `Column` / `Row`（主軸方向の配置、gap、padding、align）
- `Grid`（2列のカード並び：施策カードなど）
- `Scroll`（慣性スクロール、ラバーバンド）
- `Pager`（横スワイプのカード送り＋ページドット：コースカード、レビュー）
- `Stack`（重ね合わせ：ヘッダーの上にキャラクター画像を重ねる）
- レイアウトは2パス（サイズ計測 → 配置）。結果はフレーム内でキャッシュする

### 3.3 ウィジェット一覧（MVP）

| ウィジェット | 参考画面での用途 |
|--------------|------------------|
| Text / Label（折り返し・禁則処理つき） | 全般 |
| Button / IconButton | 閉じる、タブ |
| Toggle | 開講トグル、業務委任 |
| Stepper（− 値 ＋、長押しで連続増減） | 月謝・年俸 |
| SegmentedTabs | 本科／講習／個別 |
| BottomTabBar | 生徒／コース／集客／経理 |
| Card / Panel（角丸・枠線・影） | コースカード、施策カード |
| StarRating（小数対応） | 口コミ評価 |
| ProgressBar（目盛りつき） | 教室長の能力 |
| DonutChart（円弧＋ラベル） | 粗利率、地域シェア |
| BarChart / LineChart | 週次の売上・利益、生徒数推移 |
| PentagonChart | 自社教材の五角形 |
| Table（右寄せ数値、前週比の色分け） | 損益計算書 |
| Toast / Modal | イベント、倫理ジレンマの選択 |

### 3.4 入力
- マウス・タッチ・ペンは `winit` のイベントを共通のポインタイベントにまとめる
- タップとドラッグの判定（移動しきい値）、長押し、慣性スクロール
- **日本語入力（IME）**：塾名・生徒名の入力に必要
  - Windows：`winit` の IME イベント（`Ime::Preedit` / `Ime::Commit`）を使う
  - Web：canvas は IME を受け取れないので、**透明な `<input>` を入力欄の位置に重ねて** `compositionupdate` / `input` イベントを Rust 側に渡す

### 3.5 テキスト描画
- フォント：**Noto Sans JP**（SIL OFL）。全部入れると巨大なので**サブセット化**する（JIS第1・第2水準＋ゲーム内の固有語、目標 2〜4MB）
- `fontdue` でグリフをラスタライズ → 動的なテクスチャアトラスに詰める（LRU で追い出し）
- 日本語は複雑なシェーピングが要らないので、自作の行組み（文字幅の合計と禁則処理）で足りる
- 数値は等幅数字（tabular figures）で揃え、損益計算書の桁がぶれないようにする
- 高DPI：`scale_factor` に合わせて、実ピクセルサイズでラスタライズする

### 3.6 描画（`juku-render`）
- wgpu で 2D の**バッチレンダラ**を作る。描画命令を `Vec<Instance>` にためて、1〜数回のドローコールで描く
- 図形は**SDF（符号付き距離場）シェーダ**で描く：角丸矩形・円・円弧（ドーナツグラフ）・線を、どの解像度でもアンチエイリアスつきで描ける
- 画像（キャラクター・アイコン）は1枚のアトラスにまとめる
- クリップ矩形のスタックでスクロール領域を切り抜く
- Web 側は互換性を優先して **WebGL2 バックエンド**を標準にし、WebGPU が使える環境では WebGPU を使う
  - canvas は一度 WebGPU のコンテキストを取ると WebGL2 に切り替えられないため、surface を作る前にアダプタの有無だけを確認してから選ぶ

---

## 4. プラットフォーム差分（`juku-platform`）

| 項目 | Windows | Web (wasm32) |
|------|---------|--------------|
| イベントループ | `winit` | `winit`（web バックエンド、canvas に描画） |
| 描画バックエンド | DX12（予備として Vulkan） | WebGL2（WebGPU があれば使用） |
| 時刻 | `std::time` | `web-time`（`performance.now()`） |
| セーブ | `%APPDATA%\YYshop\saves\*.sav` | localStorage（Base64）。データが大きくなったら IndexedDB へ移行 |
| 乱数シード | OS 乱数 | `crypto.getRandomValues` |
| スレッド | 必要なら使う | 使わない（sim はシングルスレッドで設計） |
| アセット | exe に `include_bytes!` で埋め込み | 別ファイルを `fetch`（キャッシュが効く） |
| パニック | ログファイルに記録 | `console_error_panic_hook` |

- 違いは `#[cfg(target_arch = "wasm32")]` を `juku-platform` の中だけに閉じ込め、上位のクレートには出さない
- sim 内の乱数は**自前の PRNG（PCG32 など）をシードから回す**。`getrandom` の環境差に影響されず、リプレイやバグ再現ができる
- セーブデータは `postcard` でバイナリ化し、バージョン番号つきヘッダーを付ける（将来の移行のため）

---

## 5. ビルドと配布

### 5.1 Windows
```
cargo build -p juku-desktop --release --target x86_64-pc-windows-msvc
```
- 単体の `.exe`（アセット埋め込み）で配布。`#![windows_subsystem = "windows"]` でコンソールを出さない
- アイコンとバージョン情報は `winres` などのビルドスクリプトで埋め込む
- 配布形態：まずは zip。必要になったら MSIX やインストーラを検討する

### 5.2 WebAssembly
```
cargo build -p juku-web --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir dist/pkg target/wasm32-unknown-unknown/release/juku_web.wasm
wasm-opt -Oz dist/pkg/juku_web_bg.wasm -o dist/pkg/juku_web_bg.wasm
```
- `web/index.html` は canvas 1枚と、wasm を読み込む数行の JS だけ（UI は全部 Rust 側で描く）
- リリースビルドは `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"` でサイズを抑える
- 目標サイズ：wasm 本体 3MB 以下（Brotli 圧縮後 1MB 前後）

### 5.3 Cloudflare での静的配信
- `dist/`（`index.html`、`pkg/*.wasm`、`pkg/*.js`、`assets/`）を **Cloudflare Workers の静的アセット配信（または Cloudflare Pages）** に置く。サーバー処理は不要
- デプロイ：`wrangler deploy`（Workers）または `wrangler pages deploy dist`（Pages）
- `_headers` でキャッシュとセキュリティを設定する
  ```
  /index.html
    Cache-Control: no-cache
  /pkg/*
    Cache-Control: public, max-age=31536000, immutable
  /assets/*
    Cache-Control: public, max-age=604800, stale-while-revalidate=86400
  /*
    Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'
    X-Content-Type-Options: nosniff
  ```
- ファイル名にハッシュを付けて（例：`juku_web_bg.3f9a1c.wasm`）、`immutable` で長期キャッシュする
- Cloudflare の1ファイルあたりのサイズ上限（25MiB）に収まるよう、フォントと wasm のサイズを CI で監視する
- `.wasm` が `application/wasm` で配信されることを確認する（`WebAssembly.instantiateStreaming` に必要）

### 5.4 CI（GitHub Actions）
| ジョブ | 内容 |
|--------|------|
| check | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`（主に sim） |
| sim-balance | ヘッドレスで100年度分を実行し、KPI の分布をレポート（係数を変えたときの退行検知） |
| build-windows | `windows-latest` で exe を作り、成果物（artifact）として保存 |
| build-web | wasm ビルド → `wasm-opt` → サイズチェック |
| deploy | main ブランチへのマージ時に Cloudflare へデプロイ（API トークンは GitHub Secrets に置く） |

---

## 6. 技術リスクと対策

| リスク | 対策 |
|--------|------|
| UI を自作する工数 | MVP に必要なウィジェット（3.3）に絞る。参考画面はカードと表が中心で、複雑な部品は少ない |
| canvas 描画のアクセシビリティ（読み上げ不可） | 当面は対象外とし、文字サイズ設定とコントラスト確保で補う。将来、主要な数値を隠し DOM に出す案を検討 |
| Web の IME 対応 | 隠し `<input>` 方式を早い段階で試作して検証する |
| 日本語フォントのサイズ | サブセット化し、フォントは wasm と別ファイルにして並行ダウンロードする |
| WebGL2 と WebGPU の挙動差 | WebGL2 を基準に開発し、CI で Chromium のヘッドレス実行によるスクリーンショット比較を行う |
| 決定論の崩れ（浮動小数点） | sim の主要計算は固定小数点または整数（金額は「円」の i64）で行う |

---

## 7. 実装の順番（MVP まで）
1. ワークスペースの雛形を作り、`juku-sim` の週送りをヘッドレスで動かす（テストつき）
2. `juku-render`：wgpu で角丸矩形・円弧・文字を Windows と Web の両方で表示する
3. `juku-platform`：入力・DPI・保存を両環境で通す。Cloudflare への試験デプロイ
4. `juku-ui`：レイアウト、Text、Button、Toggle、Stepper、Scroll、Card
5. 経理タブ（損益計算書とグラフ）→ コースタブ → 集客タブ → 生徒タブの順に画面を作る
6. 年度報告書と合格発表の演出

---

## 8. 実装状況（MVP 着手時点）

| 項目 | 状況 |
|------|------|
| `juku-sim` | 週次シミュレーション（集客・学習・ストレス・退塾・講師の疲労・会計・口コミ）、2 月の入試と進級、倫理イベント 4 種、教室長への業務委任 4 種、セーブ（postcard）。テストとヘッドレスのバランス確認つき |
| `juku-render` | wgpu の 2D バッチレンダラ。角丸矩形・円弧・線・五芒星を SDF で描画。fontdue によるグリフアトラス |
| `juku-ui` | 即時モード UI。縦積み＋列分割のレイアウト、慣性スクロール、トグル・ステッパー（長押し連続）・セグメント・星・メーター・モーダル、ドーナツ／棒／折れ線／レーダーグラフ、禁則処理つき折り返し |
| `juku-platform` | winit 0.30 + wgpu 30。Windows（DX12/Vulkan）と Web（WebGPU → WebGL2 フォールバック）。マウス・タッチ・ホイール、高 DPI |
| `juku-app` | タイトル、教室・コース・集客・経理・年度の 5 タブ、判断イベントと合格発表のモーダル、自動保存 |
| 配信 | `tools/build-web.sh` で `dist/` を生成（ファイル名ハッシュ・サイズチェック）、`wrangler.toml`（Workers 静的アセット）、GitHub Actions |

設計からの変更点
- Web のセーブは IndexedDB ではなく localStorage で開始した（データが小さいため）
- フォントはファイル名にハッシュを付けていないため、キャッシュは 1 週間＋ stale-while-revalidate とした
- 日本語入力（IME）は MVP の画面に文字入力がないため未着手（α で塾名入力とあわせて実装する）
- 自社教材の五角形、複数教室、競合 AI の能動的な行動は α 以降（→ 9 章で実装済み）

---

## 9. α 機能：自社教材・複数教室・競合 AI

### 9.1 `juku-sim` の構成変更
1 教室分の状態（生徒・講師・教室長・コース・施策・口コミ・週次レポート）を `Classroom` に切り出し、`World` は会社全体を持つ形にした。

| モジュール | 内容 |
|------------|------|
| `world.rs` | 会社（資金・ブランド・信頼・教材・エリア・教室・競合・入試・イベント）と週送りの進行 |
| `classroom.rs` | 1 教室の週次処理（入塾受け入れ・学習・退塾・講師の疲労・会計・口コミ・委任業務）とエリア需要の配分 |
| `materials.rs` | 教材の 5 軸と開発プロジェクト、学習・ストレス・入試への効果 |
| `market.rs` | エリア・競合塾・競合の行動（4 週ごと）・年度替わり |
| `gen.rs` | 生徒・講師・教室長の生成 |

週送りの順序：委任業務と講習の受講決定 → エリアごとの需要配分（自塾と競合の魅力度の 4 乗比）→ 各教室の学習・退塾・会計 → 本部費用（教材開発・本部運営費）→ 競合の行動 → 不祥事判定 → イベント → 暦（年度替わりで入試・進級・教材の陳腐化・競合のブランド更新）。

### 9.2 自社教材
| 項目 | 値 |
|------|----|
| 立ち上げ費 | ¥2,500,000（以後、教材原価が月 ¥2,600 → ¥1,300／人） |
| 改訂 | 1 軸を 4 週で +0.12。週あたり費用 = ¥120,000 ×（1 + 現在値 × 1.5） |
| 効果 | 網羅性：上位層の伸びと最難関校の合格率／難度：上位は伸び、下位は伸び悩みストレス増／わかりやすさ：中下位の伸びと意欲・授業評価／演習量：全員の伸びとストレス／入試対応：合格判定に偏差値換算で最大 ±1.5 |
| 陳腐化 | 年度替わりに入試対応 −0.2（市販教材は出版社が毎年改訂するので一定） |

### 9.3 複数教室
- エリアは 5 つ（けやき台・桜ヶ丘・港南・緑町・東雲）。市場規模・家賃・開校費・出店している競合が違う
- 1 エリア 1 教室。開校時に教室長を 3 人の候補から選ぶ。新教室は学生講師 2 人・生徒ゼロから始まる
- ブランド力（合格実績）は会社全体で共有し、全教室の集客に効く
- 2 教室目から本部運営費（週 ¥40,000／教室）がかかる

### 9.4 競合 AI
4 週ごとに、各社が出店エリアごとのシェアの変化を見て行動する。

| 状況 | 大手難関型 | 面倒見型 | 格安オンライン型 | 個別指導型 |
|------|-----------|----------|------------------|------------|
| シェアが下がった | 大規模広告（認知度 +0.25） | 面談・補習の強化（面倒見 +0.03） | 1 割値下げ | 入塾金無料キャンペーン |
| シェアが上がった | 上限まで 5% 値上げ（半々の確率） | 同左 | 同左 | 同左 |
| 自塾が半年以上かけて 3 割以上のシェアを取ったエリア | 出店（30%） | 出店（10%） | — | 出店（20%） |
| 同じエリアに授業力 70 以上の講師がいる | 引き抜き（12%） | — | — | 引き抜き（8%） |
| 生徒が少ない状態が 4 か月続く | 撤退 | 撤退 | — | 撤退 |

引き抜きは判断イベントになる（給与 2 割増で引き止める／送り出すと相手の授業力が上がる）。保護者対応を委任していれば教室長が倫理観に応じて判断する。
