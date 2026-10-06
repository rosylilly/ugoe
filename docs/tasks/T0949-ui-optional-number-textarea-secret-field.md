---
id: T0949
title: "@ugoe/ui: 空を持てる数値の欄・複数行の文字の欄・秘密の欄を足し、エディタの残りの <input> を移す"
status: in-progress
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

- [ ] `NumberField` が空を持てる (`optional` など。空のときの表示 (placeholder に既定値)・確定で空を送る・矢印キーで空から始めたときの値を決めてメモに書く)。部品のテストがある
- [ ] 複数行の文字の部品 (`TextArea`。`TextField` と同じラベル・補足・誤り・確定の約束) と部品のテストがある
- [ ] 値を状態に持たない秘密の欄の口 (`TextField` の見た目で、値を読み出さずに確定のときだけ渡すなど。形をメモに書く) と部品のテストがある
- [ ] エディタの上の 3 種類の標準の `<input>`・`<textarea>` を移し、docs/ui.md の表から消した (移せないものは理由を残す)
- [ ] 見本の画面 (`just ui-shots`) に足し、`just ui-audit --only editor` の指摘が増えない
- [ ] `just ci` が通る

## 参考

- T0917 のメモ (やらなかったこと・起票の候補)、docs/ui.md#足りない部品と口-t0917
- packages/ui (`NumberField`・`TextField`)、apps/editor の `SettingsDialog`

## メモ

- T0040 で起票した (元の候補: T0917 (空を持てる数値の部品)・T0917 (`TextArea`)・T0917 (秘密の欄))。
  ツリーの名前の変更 (行の中で開く欄) は T0674 のメモに書き足した。
- T0041 (7 回目の起票のまとめ) から書き足した: T0674 の候補「`@ugoe/ui` の `Checkbox` に見えるラベルを持たない形 (`aria-label`) を足し、
  エディタのライティングの一覧 (光を受けるメッシュの行) の標準のチェックボックスを置き換える」。エディタの残りの標準の `<input>` を移すのと同じ流れなので、
  このタスクで扱う (行の高さ 24px と、包む `<label>` を押せる範囲にしている今の振る舞いを保つ)。
