---
id: T0717
title: ビューア: ドロップと光源の印の細かな残り
status: done
milestone: M4
crates: [apps/viewer, docs]
depends_on: [T0713, T0714]
assignee: claude-worker-T0717
---

# T0717: ビューア: ドロップと光源の印の細かな残り

## 目的

T0713・T0714 の後続の候補をまとめる。

## 受け入れ条件

- [x] フォルダや読めない拡張子をドロップしたとき、読めるものだけを開き、残りは飛ばしたと日本語で知らせる (今は read_model_file の英語のエラー)
- [x] ドラッグ中 (enter の paths) に、読めるファイルかどうかを示す
- [x] 点光源の範囲の円の縁が画面の外にあるときも、ビューポートで範囲を変えられる (縁を端に寄せた小さな取っ手など)
- [x] 端に寄せた平行光源の円が、キャンバスの中央を示していないことが見た目で分かる
- [x] 覚えたものの一覧を新しい順に並べられる (覚えた日時を持つ)
- [x] 取っ手にフォーカスがあるときの F キー (フィット) の扱いを決める
- [x] Vitest とヘッドレスのブラウザで確かめる。`just ci`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0713-viewer-drop-paths.md`・`docs/tasks/T0714-viewer-lighting-followups.md` のメモ

## メモ

### やったこと

- **ドロップ (`drop.ts` の `openDroppedItems`):** 拡張子 (`.ugoe`・`.json`・`.ugoemod`、`read_model_file` と同じ) で開けるものだけを読み
  (`Promise.allSettled`)、読めたものを今までどおり `openDroppedFiles` で開く。拡張子の無いもの (フォルダ)・ほかの拡張子は読まずに、
  読めなかったもの (拡張子の付いたフォルダ・消えたファイル) も飛ばし、名前を日本語でステータスバーに出す (`skippedMessage`。
  4 個以上は「ほか N 個」。次に開くまで出す)。何も開けなければ「開けるファイルがありませんでした: …」。ブラウザの File API のドロップも同じ経路。
  Rust のバックエンドは変えていない (英語のエラーは出さずに、飛ばした理由を日本語にまとめる)。
- **ドラッグ中の見立て:** `FileDropEvent` の `hover` に `paths` (Tauri の `enter` の `paths`) を足し、`describeDropPaths` で
  すべて開ける (青)・一部を飛ばす (黄)・開けるものが無い (赤) を枠の色と上の文で示す。ブラウザではドラッグ中に名前が分からないので青のまま。
  ついでに、落とす枠 (`main` の `outline`) が canvas に隠れてサイドバーにしか見えていなかったので、`main` に重ねた要素 (`.drop-frame`) で描くようにした。
- **範囲の取っ手 (`lighting.ts` の `rangeGrip`・`rangeRadiusFromGrip`):** 範囲の円の縁が画面 (縁から 14px の内側) を通らない
  (円が画面を覆う・まるごと画面の外) ときに、光源から画面の中央へ向かう向きの縁の点を画面の内側に収めた位置に、両向きの矢印の小さな
  取っ手を出す。その向きに沿って動かした分だけ半径を変える。ドラッグ中は縁が画面に入っても消さない。端に寄せた点光源の取っ手と重なるときは出さない。
- **寄せた平行光源の円:** `LightDial` に `shifted`・`canvasCenter` を足し、寄せたときは円を点線・中心を白抜きにして、中心から
  本当のキャンバスの中央の向きに破線の矢印を出す。
- **覚えた日時:** `ModelMemory` の項目に `at` (ミリ秒) を足した (`set` のたび。`now` で差し替えられる)。版 (1) は変えず、`at` の無い前の
  保存値もそのまま読む (日時なし)。`mergeRemembered` に `order` (`recent` 既定・`name`) と行の `rememberedAt` (光源と組の新しい方)。
  一覧に「覚えた日時」の列と「並び」の選択 (2 行以上のとき)。開いているモデルはどちらでも先頭。日時なしは新しい順の最後 (その中は名前の順)。
- **取っ手にフォーカスがあるときの F:** フィットのままにした (理由は README: 取っ手は文字のキーを使わない。フィットしてもフォーカスは残り、印は描き直される。
  拡大して端に寄せた印を戻すのにも使える)。コードは変えず、README・マニュアル・`LightMarkers.svelte` の先頭のコメントに書いた。
- Vitest: `drop.test.ts` +5 (混ざったドロップ・何も開けない・拡張子の判定・見立て・名前の省略。hover の `paths`)、`lighting.test.ts` +2
  (範囲の取っ手の位置と半径、寄せた円の `shifted`)、`modelMemory.test.ts` +1 (日時と前の保存値)、`rememberedModels.test.ts` +2 (新しい順・日時の表示)。
- 文書: `apps/viewer/README.md` (使い方の表・開けないものが混ざったドロップ・覚えた日時と並び・寄せた円・範囲の取っ手・F)、
  `docs/manual/lighting.md` (一覧の並び・寄せた円・範囲の取っ手・F)、`docs/manual/dress-up.md` (ビューアでのドロップ)。

### 確かめたこと

- `just ci` (main に rebase し、`just wasm` とビューア用の `just wasm` をやり直した後)・`vite build`。src-tauri は変えていない。
- ヘッドレスの Chrome (ビルドした `dist/` を `vite preview`、T0713 と同じ Tauri を模したホスト。`readPath` は拡張子の違うものと
  見本に無いものに `read_model_file` と同じ英語のエラーを投げる)。スクリプトは scratchpad の `T0717/check.mjs`、出力は `T0717/run.log`:
  - ドラッグ中: [.ugoe, .ugoemod] → 青「2 個を開きます」、[.ugoe, フォルダ, .png] → 黄「1 個を開きます (2 個は開けないので飛ばします)」、
    [フォルダ] → 赤、離れると枠が消える。
  - [フォルダ, dress-up.ugoe, face.png, old.ugoe (無い), twin-tails.ugoemod] のドロップ → dress-up に TwinTails が付き、
    ステータスバーに「開けないものを 3 個飛ばしました: Pictures、face.png (…)。old.ugoe (読めませんでした)」。フォルダと .png は読みに行かない。
    [フォルダ, notes.txt] → 「開けるファイルがありませんでした…」で、開いているモデルはそのまま。次のドロップで知らせは消える。
  - avatar-lit の夜のランプ (範囲 1.4、円の半径 1025px が画面を覆う) → 右下の端 (966, 742) に範囲の取っ手 → 左へ 150px で範囲 1.4 → 1.251、
    モデルごとの記憶にも入る (日時付き)。
  - 左上を中心に 1200% → 円は寄せて点線・白抜きの中心・中央の向きの矢印。平行光源の取っ手にフォーカスして F → 100% に戻り、フォーカスは取っ手のまま。
  - 日時なしの前の保存値を 1 つ入れて一覧を開く → 新しい順で 2 行に日時、前の保存値は「日時なし」で最後。名前の順に切り替えられる。
  - スクリーンショット: `/tmp/claude-1000/-home-rosylilly-src-github-com-rosylilly-ugoe/32f91561-94f7-4861-9c03-8fe8c9ff7192/scratchpad/T0717/shots/`
    の `01-hover-all.png`・`02-hover-some.png`・`03-hover-none.png`・`04-mixed-drop.png`・`05-range-grip.png`・`06-range-grip-dragged.png`・
    `07-shifted-dial.png`・`08-memory-recent.png`・`09-memory-name.png`。
  - 読み込み時の 404 が 1 件 (T0707 以降と同じもの)。

### 確かめていないこと (利用者の確認に残す)

- Tauri (WebKitGTK・WebView2・WKWebView) の本物のウィンドウでのドロップ: `enter` に本当に `paths` が来るか (Tauri v2 の型では来る)、
  フォルダを落としたときにパスで来るか、枠の色が変わるか。`.deb` などのビルドもしていない。
- 範囲の取っ手のタッチ操作・高 DPI。

### 後続の候補

- ブラウザ版のドラッグ中は名前が分からないので見立てられない (`DataTransferItem` の `type` は `.ugoe` では空)。
- 範囲の取っ手は、光源と画面の中央が近いと右向きに固定する。取っ手の位置が、ドラッグの向きと画面の上の円の縁の見え方と合わない場合がある (円の縁が画面の外なので)。
- 一覧の並びの選択は覚えない (開くたびに新しい順)。
