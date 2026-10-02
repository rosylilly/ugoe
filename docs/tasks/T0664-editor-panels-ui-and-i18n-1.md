---
id: T0664
title: エディタ: パネルの移行 1 (インスペクタ・ツリー・パラメータ・タイムライン・ビューポートの道具)
status: done
milestone: M8
crates: [apps/editor, docs]
depends_on: [T0663]
assignee: claude-worker-T0664
---

# T0664: エディタ: パネルの移行 1 (インスペクタ・ツリー・パラメータ・タイムライン・ビューポートの道具)

## 目的

T0663 の続き。編集の中心のパネルを共有部品と翻訳に移す。

## 受け入れ条件

- [x] panels/inspector・tree・parameters・timeline と viewport の道具の表示を共有部品に置き換え、文言をカタログに移す
- [x] 見た目と操作が変わらないことをヘッドレスの Chrome で確かめる (両方の言語)
- [x] `just ci` が通る

## 参考

- T0663 のメモ

## メモ

- **やったこと:**
  - インスペクタ (InspectorPanel と inspector/ のすべて)・ツリー (TreePanel・tree/)・パラメータ (ParametersPanel・parameters/・parameterValue.ts)・
    タイムライン (timeline/)・ビューポートの道具 (Viewport・ToolOverlay・saveScreenshot.ts・tools/) を @ugoe/ui に移し、文言をカタログ
    (`inspector`・`tree`・`parameters`・`timeline`・`viewport`) に移した。許す一覧から、これらのファイルはすべて消えた (0 件)。
    `.ts` の文 (`avatar.ts`・`inference.ts`・`depth.ts`・`timeline.ts`・`expressions.ts`・`edit.ts`・`mirror.ts` など) は呼ぶたびに `t()` を通す関数にした。
  - 部品: 選択欄は `Select`、チェックは `Checkbox`、数値の欄は `NumberField`、ボタンは `Button small`・`IconButton`・`TipButton`、
    知らせは `Notice` (StaleRigNotice・RederiveNotice・描画用の複製のずれ・アバターの準拠の結果)、タブは `TabStrip` (ツリーの Parts / Deformers・
    タイムラインの Motions / Expressions)、モーションのイベントは `Popover`。入・切のボタンは `pressed` (Form edit・Add / Remove pairs・
    Module outline (前はチェックボックス)・表情のプレビュー・区間の種類)。`title` 属性は Tooltip にした (下の例外を除く)。
  - 操作の変わったところ: ツリーの削除の確かめを行の下の枠から `AlertDialog` に (既定のフォーカスは取り消し)。スクリーンショットの保存の
    知らせを Toast に、失敗は閉じるまで残る `Notice` に。RederiveNotice の「Dismiss」は Notice の閉じるボタンに (docs/editor.md・
    manual/warnings.md を直した)。数値の欄は NumberField の操作 (矢印で増減、Enter かフォーカスを外して確定、表示は桁を丸める)。
  - `app.css` に h3 の形を足した (docs/ui.md の約束)。`lib/ui/baseLock.ts` の `useBaseLock()`: BaseLock の `fieldset disabled` が効かない
    部品 (Slider のつまみなど) に無効を渡すための context (docs/ui.md の表に書いた。T0664 の範囲では使う所が無く、T0665 の物理などで使える)。
  - 確かめたこと: `just ci`・`cargo check -p ugoe-editor`。画面は T0663 のブリッジ (scratchpad の `T0664/run.sh`・`shots.mjs`) で、
    英語と日本語で撮った (scratchpad の `T0664/shots/`。モデル・Screenshot のツールチップ・メッシュのインスペクタ・キーフォーム・推定・深度・
    メッシュ編集・フォームの編集・デフォーマ・タイムラインのモーションと表情・ツリーの削除の確かめ・モジュールの本体の要素 (読み取り専用))。
    T0663 の撮った前の画面と比べ、並びと大きさはほぼ同じ。
- **確かめていないこと:** 本物の Tauri のウィンドウ。パラメータのスライダーのドラッグ・ドープシートやグラフのドラッグ・ギズモの操作 (文言だけ
  移したが、手で動かしてはいない)。スクリーンショットの保存の Toast と失敗の Notice、モーションのイベントの Popover は画面に出していない。
  狭い幅 (1024px)。日本語の訳語のいくつか (「Unwrap」=「包みを解く」、authored / derived =「手作り」/「推定」、アバターのレベルの名前) は仮。
- **やらなかったこと (そのままにしたもの):**
  - パラメータのスライダーはネイティブの `<input type="range">` のまま (キーの印の位置がつまみの幅に合わせて計算されていて、Slider に替えると
    印がずれるため)。文言だけ移した。
  - 空にできる数値の欄 (ミラーの Axis・Tolerance、対の Mirror axis、カーブごとのフェードの In・Out) は `<input type="number">` のまま
    (NumberField が空を受けないため)。文字の欄 (名前など) と textarea もネイティブのまま (@ugoe/ui に部品が無い)。
  - ツリーの行の印 (base・mask・hidden・描画順)・ドープシートの行と印・カーブごとのフェードの欄の説明は、訳した `title` のまま
    (行の数だけ Tooltip を作ると重く、行はフォーカスを受けないため)。T0907 で見直す。
  - 見出しの太さ 600 は、この環境のフォントでは普通の太さに見える (前はブラウザの既定の 700)。
- **範囲外の変更:** `scripts/hardcoded-strings-allowlist.json` (縮めただけ)。docs/ui.md・docs/editor.md・docs/manual/warnings.md (crates: の docs の中)。
- **後続の候補:**
  - @ugoe/ui の NumberField に「空 = 既定」を許す形と、文字の欄の部品 (TextField) を足し、上のネイティブの欄を移す。
  - パラメータのスライダーを Slider に移す (Slider に目盛り・印を出す仕組みが要る)。
  - ビューポートの左上の文 (メッシュ編集の操作の説明・本体のモデルの知らせ) が右上の `webgl2` と重なる (前から。日本語で長くなりやすい)。
  - タイムラインの `keyBatch` (Undo の履歴の名前) は作った時点の言語で残る。

