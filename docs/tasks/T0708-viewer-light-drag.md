---
id: T0708
title: ビューア: 光源をビューポートの上で直接ドラッグする
status: done
milestone: M4
crates: [apps/viewer, docs]
depends_on: [T0705]
assignee: claude-worker-T0708
---

# T0708: ビューア: 光源をビューポートの上で直接ドラッグする

## 目的

利用者の要望 (T0705 の後続の候補)。光源をパネルの丸い面・四角い面だけでなく、モデルの上で直接動かせるようにする。

## 受け入れ条件

- [x] 点光源の位置と、平行光源の向きを示す印をビューポートに重ね、ドラッグで動かせる (パン・ズームの操作とぶつからない切り替えか修飾キー)。パネルの値と同期する
- [x] 印の表示の有無を切り替えられる。拡大縮小や鏡像の表示でも位置が合う
- [x] ヘッドレスのブラウザで確かめる。README を更新する
- [x] `just ci`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0705-viewer-lighting.md` のメモ

## メモ

### やったこと

- `LightMarkers.svelte` (新規): ビューポートに重ねる SVG。形はエディタの光源の取っ手 (`LightingOverlay.svelte`) に合わせた。
  - 平行光源: 表示しているキャンバスの中央に破線の円 (半径はキャンバスの短い辺の 35% を 60〜220 CSS px に収める)、光が来る向きに光源の色の取っ手。
    取っ手のドラッグで方位・高さ (円の外は縁 = 真横から)。
  - 点光源: キャンバスの上の位置に放射線付きの取っ手。ドラッグで横・縦。面の上で届く範囲 `√(range² − z²)` を破線の円で描く。
  - **パンとの切り分け:** 修飾キーやモードではなく「取っ手を掴んだときだけ光源を動かす」(取っ手だけ `pointer-events: auto`、押したら
    `stopPropagation`)。ほかの場所のドラッグはパン、ホイールは取っ手の上でもズーム、取っ手の上のダブルクリックではフィットしない。
  - 出すのは `status === "ready"`・`usesLighting`・光源を当てている・印を出す設定のときだけ。切っている光源の印は出さない。
- `player.svelte.ts`: `view` (`WebPuppet.view_transform()` を devicePixelRatio で割った CSS px の変換。fit・zoomBy・panBy・resize・モデルを開いたときに読み直す)、
  `canvasSize`、`lightMarkers`・`setLightMarkers` (`ugoe-viewer.lighting.markers` に覚える、既定は出す)。
  `updateLighting(edit, { preview: true })` は描画だけを変えて覚えず、`commitLighting()` で最後に使った設定とモデルごとの設定に 1 回だけ書く
  (T0709 と同じ `setLighting` の経路)。覚える前に別のモデルを開いたら、`load` の最初で前のモデルの設定として覚える。
- パネルもドラッグ中は覚えないようにした (T0705 からスライダー・面のドラッグで毎回 localStorage に書いていた): スライダー・色は `oninput` で preview、
  `onchange` で commit。`LightPad` はポインタのドラッグ中は preview、離したときに `oncommit`。キーボードはすぐ覚える。
  パネルに「ビューポートに光源の印を出す」のチェック。
- `lighting.ts`: 幾何 (`lightDial`・`directionalHandle`・`anglesFromScreen`・`pointHandle`・`pointFromScreen`・`pointRangeOnCanvas`) と印の表示の保存。
  Vitest 5 件 (`lighting.test.ts`)。`stub.d.ts` に `view_transform`。
- 文書: `apps/viewer/README.md` (構成・操作の表・ライティングの表・「ビューポートの光源の印 (T0708)」の節・ドラッグ中は覚えないこと)、
  `docs/manual/lighting.md` (「ビューアで光源を直接動かす」)、`docs/spec/rendering.md#ホストでのライティング` のビューアの項目に 1 行。

### 確かめたこと

- `just ci` (main (T0125) に rebase し、`just wasm` をやり直した後)・`vite build`。
- ヘッドレスの Chrome (SwiftShader の WebGL2、ビルドした `dist/` を `vite preview`) で「サンプル」→「アバター (ライティング)」
  (スクリプトは `<scratchpad>/T0708/check.mjs`、出力は `run.log`):
  - 平行光源の取っ手を円の右の外へ → 方位 0°・高さ 0°、描画の左右の明るさ (背景を除く平均) 左 123 / 右 113.7 → 左 75.1 / 右 89.2。
    パネルのスライダーも同じ値。モデルごとの記憶への書き込みはドラッグ 1 回につき 1 回 (`setItem` を数えた)。
  - 「夜のランプ」で点光源の取っ手を (80%, 70%) へ → パネル・モデルごとの記憶・最後に使った設定がどれも (0.8, 0.7)、明るさ 左 42.4 / 右 76.8。
  - 空いたところのドラッグはパンになり光源は変わらない。ズーム (448%) とパンの後も取っ手の位置が `view` から計算した位置と 0.1px 以内。
    ズームした状態でのドラッグも、画面の移動量 / 倍率どおりに位置が変わる。ダブルクリックでフィットすると印も戻る。
  - 印を消す → 読み直しても消えたまま → 出す。光源を切ると印も消える。ライティングのないモデル (「アバター」) では出ない。
  - スクリーンショット: `<scratchpad>/T0708/shots/01-markers.png` (標準、平行光源の円と取っ手)・`02-directional-right.png`・`03-lamp.png`・
    `04-point-dragged.png`・`05-zoomed-panned.png`・`06-markers-off.png`。
  - 読み込み時の 404 が 1 件 (T0707・T0710 と同じもの)。

### 確かめていないこと・後続の候補

- Tauri (WebKitGTK) のウィンドウ、`.deb` のビルド、タッチ操作、高 DPI (devicePixelRatio ≠ 1) の画面では動かしていない (計算は dpr で割っている)。
- **鏡像の表示:** ビューポートには鏡像の表示が無い (フェイストラッキングの「鏡像で表示」とランドマークは、右のパネルのカメラの映像にだけ掛かる)。
  そのため印とランドマークは重ならず、反転の考慮も要らない。ビューポートに鏡像の表示を足すなら印の変換にも同じ反転を入れる (README に書いた)。
- 取っ手はキーボードで操作できない (`tabindex=-1`。パネルの面がキーボードで動かせる)。
- 大きく拡大すると平行光源の円の中心 (キャンバスの中央) が画面の外に出て取っ手が見えなくなる。表示している範囲の中央に置く案もある。
- 点光源の「手前」「範囲」をビューポートで変える操作 (範囲の円の縁のドラッグなど) は無い。
