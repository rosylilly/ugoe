---
id: T0718
title: ビューア: ブラウザのドラッグ中の見立て・範囲の取っ手の向き・一覧の並びを覚える
status: done
milestone: M4
crates: [apps/viewer, docs]
depends_on: [T0717]
assignee: claude-worker-T0718
---

# T0718: ビューアの小さな残り

## 目的

T0717 の後続の候補をまとめる。

## 受け入れ条件

- [x] ブラウザ版では、ドラッグ中にファイル名が分からないので、開けるかを見立てられない。種類 (MIME) や数だけでも示せるか確かめ、できる範囲で示す
- [x] 点光源の範囲の取っ手が、光源が画面の中央に近いと右向きに固定される。向きの決め方を見直す
- [x] 覚えたものの一覧の並びの選択を覚える
- [x] Vitest とヘッドレスのブラウザで確かめる。`just ci`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0717-viewer-small-followups.md` のメモ

## メモ

### やったこと

- **ブラウザのドラッグ中の見立て (`drop.ts` の `describeDropTypes`・`classifyDragType`):** ブラウザはドラッグ中に名前を渡さないが、
  `DataTransfer.items` の数と `kind`・`type` (MIME) は分かる (ヘッドレスの Chrome の本物のドラッグで、保護されたドラッグ中でも
  `[["file","application/json"],["file","image/png"]]` が見えることを確かめた)。JSON は開ける、種類の付いたほかのもの (画像・文書) は
  開けない、種類が空・`application/octet-stream` (`.ugoe`・`.ugoemod`・フォルダ) は分からない、と分ける。分からないものがあれば
  「N 個のファイル (N 個は、開けるかを落としたときに名前で確かめます)」で枠は青 (新しい `kind: "unknown"`)。開けない種類だけなら赤、
  混ざれば黄。一緒に来る文字列の項目は数えない。`items` の無いブラウザでは見立てず枠だけ (今までどおり)。`App.svelte` の `dragover` で呼ぶ。
- **範囲の取っ手の向き (`lighting.ts` の `rangeGrip`):** 光源から画面の中央への向きをやめ、縁が最初に画面に入ってくるところに置く。
  円が画面を覆うときは光源からいちばん遠い画面の角 (同じ遠さなら右下・右上・左下・左上の順)、円が画面の外にあるときは光源に
  いちばん近い画面の点。どちらも取っ手は光源から伸ばした線の上にあるので、矢印の向き = 光源から離れる向きになり、縁が画面に入るところで
  取っ手と縁が重なる。光源が中央に近いときも右向きに固定されない。`rangeRadiusFromGrip`・`LightMarkers.svelte` の扱いは変えていない。
- **一覧の並びを覚える (`rememberedModels.ts` の `loadRememberedOrder`・`saveRememberedOrder`):** localStorage の
  `ugoe-viewer.remembered-order` に `recent` / `name` を覚える。新しいキーなので前の保存値には触れない。無い・知らない値・読めないときは
  新しい順。`MemoryDialog.svelte` は選んだときに覚え、開くたびに読み直す (ほかのタブで選び直したものも拾う)。
- Vitest: `drop.test.ts` +2 (種類の分け方・見立て)、`lighting.test.ts` (範囲の取っ手の位置と向きを書き直し。中央の近く・左の外・線の上)、
  `rememberedModels.test.ts` +2 (覚える・壊れた保存値)。
- 文書: `apps/viewer/README.md` (使い方の表・ブラウザのドラッグ中の見立て・並びを覚える・範囲の取っ手の位置)、
  `docs/manual/lighting.md` (範囲の取っ手・並び)、`docs/manual/dress-up.md` (ブラウザのドラッグ中)。

### 確かめたこと

- `just ci` (main に rebase し、`just wasm` とビューア用の `just wasm` をやり直した後)・`vite build`。
- ヘッドレスの Chrome (ビルドした `dist/` を `vite preview`、ホストはブラウザのまま)。スクリプトは scratchpad の `T0718/check.mjs`、
  出力は `T0718/run.log`:
  - 合成の `dragover` (種類付きの `File`): JSON → 青「1 個を開きます」、.ugoe (種類なし) + JSON → 青の「確かめます」、
    .ugoe + .png → 黄、.png + .txt → 赤、`dragleave` で枠が消える。
  - CDP の本物のドラッグ (`Input.dispatchDragEvent`、実ファイル): .ugoe → `["file",""]` で青の「確かめます」、.json + .png → 黄、
    .png + .txt → 赤、フォルダ → `["file",""]` で青の「確かめます」。
  - 見本「アバター (ライティング)」の夜のランプ (円が画面を覆う) → 取っ手は右下の角 (光源は左上)。光源を画面の中央のわずかに右下
    (51%, 52%) へ → 取っ手は左上の角 (T0717 では右の縁)。取っ手を光源の向きへ 150px → 範囲 1.4 → 1.209。
  - 覚えたものの一覧で「名前の順」を選ぶ → 閉じて開き直しても、ページを読み込み直しても名前の順。
  - スクリーンショット: `<scratchpad>/T0718/shots/`
    の `01-browser-unknown.png`・`02-browser-some.png`・`03-browser-none.png`・`04-grip-lamp.png`・`05-grip-near-center.png`・
    `06-grip-dragged.png`・`07-memory-order-kept.png`。
  - 読み込み時の 404 が 1 件 (T0707 以降と同じもの)。

### 確かめていないこと

- Firefox・Safari のドラッグ中の `items` (Safari はドラッグ中に `items` が空のことがある。そのときは枠だけで、今までどおり)。
  OS が `.ugoe` に種類を付ける環境 (いまはどのアプリも登録していない。`ugoe` を含む種類は開けるとみなす)。
- 範囲の取っ手のタッチ操作・高 DPI。

### 後続の候補

- 光源が画面の中央をまたいで動くと、覆うときの範囲の取っ手が角から角へ飛ぶ (いちばん遠い角が入れ替わる)。気になるなら前の角を保つ
  ヒステリシスを足す。
- ビューアのアプリにファイルの種類 (`.ugoe`・`.ugoemod`) を登録すれば、ブラウザでも種類で見立てられる (Tauri の `fileAssociations`)。
