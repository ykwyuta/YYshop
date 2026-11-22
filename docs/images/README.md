# 未生成画像リスト

首都圏攻略マップ・街・校舎の画像は、あとで画像生成で作ります。今のゲームでは、それぞれの場所に「画像未生成：ファイル名」と書いた仮の絵を出しています（`crates/juku-app/src/images.rs`）。

- 合計 **33 枚**：地図 1 枚、街 16 枚、校舎 16 枚
- 置き場所：`assets/images/` の下（下の表のパス）
- 画像を置いたあとに、読み込んで描く処理を足します（今は未実装。スプライトの atlas は幅を揃える必要があるので、別テクスチャで読み込む予定）

## 共通のスタイル

すべての画像で揃えるもの：

- アニメ調の背景美術。やわらかい光で、色数は少なめ。ゲームの UI（深い緑・黒板色・オレンジのアクセント）になじむ色にする
- 文字・看板の文字・ロゴ・実在の店名やブランドは入れない
- 人物は小さく、顔がわからない程度にする
- 実在の街だとわかる地形・街並みの特徴は入れるが、写真のような再現にはしない

共通のプロンプト（英語、各画像の説明の前に付ける）：

```
anime background art, soft warm lighting, clean painterly style, muted palette with deep green and warm orange accents, no text, no signage lettering, no logos, no recognizable people
```

## 1. 地図（1 枚）

| ファイル | 大きさ | 内容 |
|---|---|---|
| `assets/images/map/shutoken_map.png` | 2048×1792（横 64 km × 縦 56 km の比） | 首都圏の地図の背景。真上から見たイラスト地図 |

- 範囲：経度 139.37〜140.08、緯度 35.43〜35.94（中心 東経 139.725・北緯 35.685）。北が上。ゲームはこの範囲に合わせて街を置くので、範囲と縮尺は変えない
- 入れるもの：東京湾の海岸線、多摩川・荒川・江戸川、都県境（うっすら）、23 区あたりの市街地の密集、丘陵の緑（多摩丘陵・狭山丘陵）
- 入れないもの：駅・街の名前・鉄道（ゲーム側で描く）、文字
- プロンプト例：`top-down illustrated map of the Tokyo metropolitan area, Tokyo Bay in the lower right, rivers Tama, Arakawa and Edogawa, dense city in the center, green hills in the west, fantasy RPG world map style, no text`

## 2. 街の画像（16 枚）

街を選んだときの詳細カードの上に出ます。大きさはすべて **1600×900（16:9）**。

| 地方 | 街 | ファイル | 描くもの |
|---|---|---|---|
| 東京西部 | 吉祥寺 | `assets/images/towns/town_kichijoji.png` | 井の頭公園の池とボート、木漏れ日。奥に駅前のアーケード街 |
| 東京西部 | 成城学園前 | `assets/images/towns/town_seijogakuenmae.png` | 銀杏並木と生け垣の続く邸宅街、秋の黄色い並木道 |
| 東京西部 | 調布 | `assets/images/towns/town_chofu.png` | 再開発された駅前広場と、遠くに多摩川の河川敷 |
| 東京西部 | 府中 | `assets/images/towns/town_fuchu.png` | 大きな神社へ続くけやき並木の参道、落ち着いた街並み |
| 東京西部 | 二子玉川 | `assets/images/towns/town_futakotamagawa.png` | 多摩川の広い河川敷と鉄橋、川向こうに新しい高層ビル |
| 埼玉 | 浦和 | `assets/images/towns/town_urawa.png` | 緑の多い文教地区の住宅街、学校帰りの小学生（小さく）、レンガ調の歩道 |
| 埼玉 | 大宮 | `assets/images/towns/town_omiya.png` | 何本もの線路が集まる大きな駅と、長い神社の参道の並木 |
| 埼玉 | 所沢 | `assets/images/towns/town_tokorozawa.png` | 広い公園の芝生と空、郊外の住宅地、遠くに飛行機 |
| 都心 | 渋谷 | `assets/images/towns/town_shibuya.png` | 夕暮れの大きな交差点と高層ビル群、坂の多い街 |
| 都心 | 茗荷谷 | `assets/images/towns/town_myogadani.png` | 坂道の桜並木と、大学・学校の古い校舎が並ぶ文教地区 |
| 神奈川 | 横浜 | `assets/images/towns/town_yokohama.png` | 港と観覧車、レンガ倉庫、海沿いの高層ビルの夜景 |
| 神奈川 | たまプラーザ | `assets/images/towns/town_tamaplaza.png` | 緩やかな丘の並木道と、整った戸建ての住宅地 |
| 神奈川 | 武蔵小杉 | `assets/images/towns/town_musashikosugi.png` | 林立するタワーマンションと、ベビーカーが行き交う広い歩道 |
| 湾岸・千葉 | 豊洲 | `assets/images/towns/town_toyosu.png` | 運河と橋、ガラス張りのタワーマンション群、水辺の遊歩道 |
| 湾岸・千葉 | 船橋 | `assets/images/towns/town_funabashi.png` | にぎやかな駅前の商店街と、遠くに東京湾の干潟 |
| 湾岸・千葉 | 柏 | `assets/images/towns/town_kashiwa.png` | 駅前の二層の歩行者デッキと、街路樹の通り |

## 3. 校舎の画像（16 枚）

その街に開いた自塾の教室の外観です。教室タブの一番上と、地図の詳細カードに出ます。大きさはすべて **1600×900（16:9）**（教室タブでは上下を少し切って横長に表示）。

塾の看板は色（オレンジ）と形だけにして、文字は入れません。全校舎で同じオレンジの看板・ロゴ風の形を共通にすると、チェーンらしく見えます。

| 地方 | 街 | ファイル | 校舎のようす |
|---|---|---|---|
| 東京西部 | 吉祥寺 | `assets/images/schools/school_kichijoji.png` | 創業の本校。駅前商店街の 4 階建ての雑居ビル、2〜3 階に塾。窓から明かり |
| 東京西部 | 成城学園前 | `assets/images/schools/school_seijogakuenmae.png` | 邸宅街になじむ白壁・低層の上品な校舎、植栽と小さな門 |
| 東京西部 | 調布 | `assets/images/schools/school_chofu.png` | 駅前の新しい中層ビルの 2 フロア、ガラス窓に机の列が見える |
| 東京西部 | 府中 | `assets/images/schools/school_fuchu.png` | けやき並木沿いの木目調の 2 階建て、温かい雰囲気 |
| 東京西部 | 二子玉川 | `assets/images/schools/school_futakotamagawa.png` | 多摩川を望むガラス張りのモダンなビル、テラス付き |
| 埼玉 | 浦和 | `assets/images/schools/school_urawa.png` | レンガ調の落ち着いた 3 階建て、自転車置き場に子どもの自転車 |
| 埼玉 | 大宮 | `assets/images/schools/school_omiya.png` | 駅前の大きなオフィスビルの 1 フロア、ビル壁面にオレンジの看板 |
| 埼玉 | 所沢 | `assets/images/schools/school_tokorozawa.png` | 郊外の通り沿いの 2 階建て、広い駐輪場と送迎の車 |
| 都心 | 渋谷 | `assets/images/schools/school_shibuya.png` | 旗艦校。坂の途中の高層ビル、大きなガラスのファサード、夜の明かり |
| 都心 | 茗荷谷 | `assets/images/schools/school_myogadani.png` | 緑の多い大通り沿いの、石とガラスの品のある校舎 |
| 神奈川 | 横浜 | `assets/images/schools/school_yokohama.png` | 港が見えるオフィスビル、海風を感じる明るい外観 |
| 神奈川 | たまプラーザ | `assets/images/schools/school_tamaplaza.png` | 並木道沿いのベージュ外壁の低層校舎、花壇 |
| 神奈川 | 武蔵小杉 | `assets/images/schools/school_musashikosugi.png` | タワーマンション低層階の商業フロアに入った教室 |
| 湾岸・千葉 | 豊洲 | `assets/images/schools/school_toyosu.png` | 運河沿い、タワー群のふもとの近未来的な白いビル |
| 湾岸・千葉 | 船橋 | `assets/images/schools/school_funabashi.png` | 駅前アーケード近くの雑居ビル、夕方の明かり |
| 湾岸・千葉 | 柏 | `assets/images/schools/school_kashiwa.png` | 駅前の歩行者デッキに直結したビル、デッキから入口が見える |
