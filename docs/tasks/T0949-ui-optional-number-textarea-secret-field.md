---
id: T0949
title: "@ugoe/ui: 空を持てる数値の欄・複数行の文字の欄・秘密の欄を足し、エディタの残りの <input> を移す"
status: done
milestone: M10
crates: [packages/ui, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0949
---

# T0949: @ugoe/ui: 空を持てる数値の欄・複数行の文字の欄・秘密の欄を足し、エディタの残りの <input> を移す

## 目的

T0917 で `TextField` などを足して 3 つのアプリの標準の文字の欄を移したが、部品が無いので標準の `<input>` のまま残したものがある
(docs/ui.md#足りない部品と口-t0917 の表、T0917 の候補):

- 空 (既定を使う) を持てる数値の欄: カーブのフェード・グルーのキー・推定の軸と許す差 (`NumberField` は空を持てない)。
- 複数行の文字の欄: アバターのメタデータ・AI の注文の `<textarea>`。
- 値を Svelte の状態に持たない秘密の欄: AI の設定の API キー (`type="password"`)。

## 受け入れ条件

- [x] `NumberField` が空を持てる (`optional` など。空のときの表示 (placeholder に既定値)・確定で空を送る・矢印キーで空から始めたときの値を決めてメモに書く)。部品のテストがある
- [x] 複数行の文字の部品 (`TextArea`。`TextField` と同じラベル・補足・誤り・確定の約束) と部品のテストがある
- [x] 値を状態に持たない秘密の欄の口 (`TextField` の見た目で、値を読み出さずに確定のときだけ渡すなど。形をメモに書く) と部品のテストがある
- [x] エディタの上の 3 種類の標準の `<input>`・`<textarea>` を移し、docs/ui.md の表から消した (移せないものは理由を残す)
- [x] 見本の画面 (`just ui-shots`) に足し、`just ui-audit --only editor` の指摘が増えない
- [x] `just ci` が通る

## 参考

- T0917 のメモ (やらなかったこと・起票の候補)、docs/ui.md#足りない部品と口-t0917
- packages/ui (`NumberField`・`TextField`)、apps/editor の `SettingsDialog`

## メモ

- T0040 で起票した (元の候補: T0917 (空を持てる数値の部品)・T0917 (`TextArea`)・T0917 (秘密の欄))。
  ツリーの名前の変更 (行の中で開く欄) は T0674 のメモに書き足した。
- T0041 (7 回目の起票のまとめ) から書き足した: T0674 の候補「`@ugoe/ui` の `Checkbox` に見えるラベルを持たない形 (`aria-label`) を足し、
  エディタのライティングの一覧 (光を受けるメッシュの行) の標準のチェックボックスを置き換える」。エディタの残りの標準の `<input>` を移すのと同じ流れなので、
  このタスクで扱う (行の高さ 24px と、包む `<label>` を押せる範囲にしている今の振る舞いを保つ)。

### やったこと (T0949)

- **`NumberField` の `optional`** (`packages/ui/src/components/NumberField.svelte`。型は `optional` で分かれる generics。docs/ui.md#空を持てる数値の欄):
  - 空の表示: 欄を空にし `placeholder` を出す。省くと `emptyValue` (空が表す値) を欄の書式で薄く出す。空の間は `aria-valuenow`・`aria-valuetext` を付けない。
  - 確定: 空 (空白だけも) にして Enter・フォーカスを外すと `onchange(null)`。もとから空なら呼ばない。読めない文字は確定前の値 (空なら空) に戻す。
  - 矢印キー: 空からは `emptyValue` から動かす (0.5 から上で 0.6)。無ければ範囲に収めた 0 から (値が NaN のときと同じ)。
  - 足した口: `oninput` (打つたびに読めた値を範囲に収めて渡す。空なら null、読めない文字の間は呼ばない。確定を待たずにボタンの可否を決める欄が使う)、
    `placeholder`、`aria-label` (グルーのキーの欄は見えるラベルが無いため)。
- **`TextArea`** (`<textarea>`): ラベル・補足・誤りの文・`tooltip`・`invalid`・Esc で戻すは `TextField` と同じ。Enter は改行、確定は Mod+Enter かフォーカスを外す。
  `spellcheck` は既定で true (文章の欄)。
- **`SecretField`**: 形は「値を props にも bind にも出さない。呼び手は `bind:filled` (空白を除いて空でないか) だけを受け、送るときに `bind:this` の
  `take()` で取り出す (取り出すと空にする)。捨てるのは `clear()`、`focus()` も出す」。Esc で入れかけを消し、空なら外側に渡す。Enter は標準のまま (form で送る)。
- 文字の欄の見た目 (TextField の `<style>`) を `tokens.css` に移し、3 つの部品で共有した (部品から `.css` を import すると、`vite/client` の型を持たない
  `@ugoe/player`・`@ugoe/face-tracking` の svelte-check が通らないため。Icon の見た目と同じ置き場)。
- エディタ: カーブのフェード (空の欄にモーションのフェードを薄く出す)・グルーの足すキーと新しい軸の最初のキー (空はパラメータの既定値。刻みは値域から
  `keyStep`、`keyValue` は数を受けるようにした)・推定のミラーの軸と許す差・左右の対の軸 (`oninput` で「Set partner」の可否) を `NumberField optional` に、
  アバターのメタデータの 3 つの `<textarea>` と AI の注文を `TextArea` に (注文は長すぎるとき `invalid`)、API キーを `SecretField` に移した。
  docs/ui.md の「足りない部品と口 (T0917)」の表から 3 行を消した (残りはパラメータの値の欄とチェックボックスなど)。
- テスト: `packages/ui/src/fields.test.ts` (18)、`CurveGraph.test.ts` (空にしたフェードが null で送られる)、`glues.test.ts` (`keyStep`・数の `keyValue`)。
- 確かめ: `just ui-shots` (21 枚すべて ok。見本に空を持てる数値・TextArea・SecretField を足した)、`just ui-audit --only editor --compare` (main の
  9fd1caa8 で撮った基準と比べて +0 -0 ~0、174 場面)、念のため viewer・studio も `--quick --compare` で +0 -0 ~0 (TextField の見た目を tokens.css へ
  移したため)、`just e2e --only editor` (15 件すべて通る)、`just ci` が通る。

### やらなかったこと

- パラメータの値の欄 (`ParametersPanel`) は T0917 の理由のまま標準の `<input>`。
- グルーのキーの欄・推定の欄・アバターのメタデータ・API キーの欄を描く部品のテスト (エディタの側) は書いていない (部品の側のテストと svelte-check で確かめた)。
  UI の監査の見本 (dress-up) にはグルーが無いので、グルーのキーの欄は監査の画像にも出ない。
- `TextField` の `type="password"` は残した (使っている所は無い)。

### 起票の候補

- `TextField` の `type` から `password` を外す (秘密は `SecretField` だけにする)。
- UI の監査の見本にグルーを 1 つ足し、グルーの編集の欄 (キー・軸) を撮る。GluesPanel・InferenceSection の部品のテスト (空の欄・`oninput` でボタンの可否)。
