---
id: T0627
title: リギング操作の使い勝手 (優先度 高)
status: done
milestone: M2
crates: [ugoe-editor-core, apps/editor, docs/manual]
depends_on: [T0624]
assignee: claude-worker-T0627
---

# T0627: リギング操作の使い勝手 (優先度 高)

## 目的

操作マニュアル (T0624) を書く過程で、チュートリアルの手順をたどる中で見つかった問題のうち、作業を止めてしまうもの・迷わせるものを直す。詳細は `docs/tasks/T0624-user-manual.md` のメモ。

## 受け入れ条件

- [x] **パラメータのミラーの対を UI から設定できる** (`Parameter::mirror`。今は `model.json` を手で書き換えるしかない)。チュートリアルの `model.json` を編集する回避策を取り除く
- [x] **初回のミラーが何もしない問題:** キーの追加でできた `A` のセルがあると、「Overwrite authored cells」を付けないとミラーが何も書かない。何も書かなかったときは理由を表示し、既定の挙動を見直す (例: まだ一度も編集していないセルは上書きしてよい、または初回だけ確認を出す)
- [x] **要素を名前で選べる:** ミラーの相手・ミラー元・テクスチャなどの選択肢に、ID だけでなく表示名 (日本語のレイヤー名) を出す。デフォーマの既定の ID と名前を分かりやすくする (例: 包んだ要素の名前から付ける)
- [x] **パーツを選んで包める:** パーツを選んで「Wrap in deformer」すると、その中のメッシュ (とデフォーマ) をまとめて包む
- [x] **Undo / Redo のショートカット** (Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y、macOS は Cmd)
- [x] **右の列の配置:** パラメータがインスペクタの下に隠れてスクロールが多い。右の列が横にはみ出す。パラメータとインスペクタを見やすく並べる (タブ・折りたたみ・幅の調整など)
- [x] 操作マニュアルを更新し、`just ci`・`just gen` (差分なし)・エディタのビルドが通る

## 参考

- `docs/tasks/T0624-user-manual.md` のメモ
- `docs/manual/tutorial.md`

## メモ

- **パラメータの対:** `set_parameter_mirror` は相手側を変えなかったので、`set_mirror_pair` と同じく双方向に張り、前の
  相手の対を外すようにした (`document.rs` の `pair` を共通化。存在しない相手は検証エラーではなく `NotFound`)。画面は
  パラメータの編集欄の「Mirror」(なし / 自分自身 / 相手。選ぶとすぐ送る) と、標準の命名 (`EyeLOpen` ↔ `EyeROpen`、
  `AngleX`・`AngleZ`・`…BallX` は自分自身) からの提案ボタン (`panels/parameters/`)。チュートリアルの `model.json` の
  書き換えは取り除き、9.1 を画面の操作にした。
- **初回のミラー:** 既定の挙動を変えた。明示的な `mirror_keyforms` は、`overwrite_authored` がなくても先の **未編集の
  セル** (`authored` で、基準のセルではなく、値が基準のセルと同じもの = キーを追加しただけのセル) を書き換える
  (`grid::unedited_cells`。実行中だけ `derived` にし、書かなかったものは `authored` に戻す)。ファイル形式に新しい由来を
  足す案は `ugoe-model` を変えるので採らなかった (規則は `docs/spec/keyform-inference.md#未編集のセル`)。さらに
  `ApplyReport::mirrored` (`MirrorReport { written, unedited, kept_authored }`) を返し、画面は「Mirror」の下に結果を出し、
  1 つも書かなければ理由と「Overwrite authored cells and mirror」を出す。チュートリアルの右目は `M A` になる
  (基準のセルは残る)。自動のミラーし直しは今までどおり `mirror` のセルだけ。
- **名前:** 選択欄と見出しは `顔 (Layer8)` の形 (`lib/labels.ts`)。ミラーの相手・ミラー元・ミラー元の記録・テクスチャ
  (使っているメッシュの名前を添える)・マスク・デフォーマの親・インスペクタの先頭。`create_*_deformer` は `name` が空なら
  最初のパーツか子の名前から `顔のワープ` / `Face warp` を作り、`id` を省略すると名前 (英数字がなければ種類 + 要素の ID:
  `WarpLayer13`、デフォーマを包むときは接頭辞を重ねず `RotationGroup18`) から作る。「Wrap in deformer」に「Name」欄を足した。
- **パーツを包む:** `create_*_deformer` に `parts` を足した (パーツと子孫のパーツの、同じ集まりのデフォーマの下にない
  いちばん外側のメッシュ・デフォーマ。`children` と重なれば 1 つ)。画面はパーツを選ぶと `parts` で送る。テストは
  見た目の保存・階層の保存・パーツの外のデフォーマの下の要素・重複・名前と ID・拒否。
- **Undo / Redo:** `App.svelte` がウィンドウで受ける。入力欄の中では効かない (`lib/shortcuts.ts`、Vitest)。
- **右の列:** インスペクタとパラメータを別々にスクロールする上下の分割にし、区切りをドラッグ / 矢印キーで動かせる
  (割合は `localStorage`、`lib/layout.ts`)。列の幅を 280 → 320px にし、選択欄に `min-width: 0` を付けて横のはみ出しを
  なくした (ヘッドレスで `scrollWidth - clientWidth = 0` を確認)。
- **確かめたこと:** T0624 と同じ方法 (ビルドした画面 + 本物の `Document` への HTTP ブリッジ + ヘッドレス Chrome。
  ブリッジの `app_info` は本物と同じ `0.0.0` を返すようにした) で、チュートリアルを新しい手順で最初から行った: 頭の
  パーツを「Name」`顔のワープ` で包む (`WarpGroup18`、8 メッシュ)、`頭の回転` (`RotationGroup18`)、目は名前を空で
  `左目のワープ` (`WarpLayer13`)・`右目のワープ`、パラメータ 5 つ、`EyeLOpen` の「Mirror」で `EyeROpen` (相手側も対)、
  `AngleX` の「Pair with itself」、キーと端の編集、左目を閉じる、右目の「Partner」を名前で選び「Mirror」だけで `M A`
  と結果の文、Ctrl + Z / Ctrl + Shift + Z / Ctrl + Y (入力欄の中では文書を戻さない)、深度からの回転で `R`、区切りの
  ドラッグと End、Save As・Export、`ugoe info` と `ugoe render --param AngleX=30 --param EyeLOpen=0 --param EyeROpen=0`
  で両目を閉じて振り向いた絵。1 つも書かないミラー (セルを Mark authored した後) の文と「Overwrite authored cells and
  mirror」で `M M` になることも確認。マニュアルの画像 5 枚を撮り直した (ヘッダーは `v0.0.0`)。パラメータの対は
  パラメータを作った直後に設定した (チュートリアルでは 9 章。順番は結果に関係しない)。
- **確かめていないこと:** 実際の Tauri のウィンドウでの操作とショートカット (WebView がキーを先に取らないか)、macOS の Cmd。
- **T0628 との関係:** マニュアルの画像を撮り直したので、T0628 の「ヘッダーの vbridge の修正」は満たされるはず (衝突したら
  どちらかの画像で撮り直す)。`EditorDocument.apply` が `ApplyReport | null` を返すようにした (`MeshEditTool.svelte` の
  1 行を合わせた)。`ParametersPanel.svelte` には「Mirror」の部品を 1 行足しただけ。
- **後続の候補:** デフォーマの ID の変更 (名前を変えても ID は包んだときのまま)。ミラーの通知 (`RederiveWarning`) の文も
  名前で出す。
