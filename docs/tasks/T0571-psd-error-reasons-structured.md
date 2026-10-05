---
id: T0571
title: "ugoe-psd の Invalid・Unsupported の理由を構造にして psd_read.* の detail を訳し、EditorError::PsdRead に原因を持たせる"
status: todo
milestone: M9
crates: [ugoe-psd, ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee:
---

# T0571: ugoe-psd の Invalid・Unsupported の理由を構造にして psd_read.* の detail を訳し、EditorError::PsdRead に原因を持たせる

## 目的

T0558 で、PSD の読み込みの失敗は `psd_read.*` のコードと原因を持つようになったが、次は英語の文のまま出る (T0558 の候補 2 つ):

- `ugoe-psd` の `PsdError::Invalid(String)`・`Unsupported(String)` の理由 (30 か所ほどで文を組む)。`psd_read.invalid`・`psd_read.unsupported` の `detail` が英語。
- エディタのバックエンドの `psd.read_file_failed` (AI の提案の PSD を読む `read_tree`。`EditorError::PsdRead { path, detail }`) は原因を持たず、`detail` が英語。

## 受け入れ条件

- [ ] `PsdError` の `Invalid`・`Unsupported` の理由を種類と値の構造 (例: `UnsupportedDepth { depth }`・`UnsupportedColorMode { mode }`・`GroupsTooDeep { max }`) にした
      (`#[non_exhaustive]` を保つ。ugoe-psd は crates.io に出していないので互換の別名は要らない)。テストで種類と値を確かめる
- [ ] editor-core の `Coded for PsdError` が理由の種類ごとのコードと引数を出し、エディタの日英のカタログで訳す (`sourced_codes` のテストと見本を直した)
- [ ] `EditorError::PsdRead` が原因 (`PsdError` の `Message`) を持ち、`psd.read_file_failed` の `detail` も `psd_read.*` で訳される。テストがある
- [ ] crates/ugoe-psd/README.md・docs/editor.md のエラーの節を直した
- [ ] `just ci` が通る

## 参考

- T0558 のメモ (判断・やらなかったこと・起票の候補)
- crates/ugoe-psd/src (`PsdError`)、crates/ugoe-editor-core (`Coded`・`sourced`)、apps/editor/src-tauri (`EditorError::PsdRead`)

## メモ

- T0038 で起票した (元の候補: T0558 (`Invalid`・`Unsupported` の理由と `EditorError::PsdRead`))。同じエラーの組み立てを触る T0572 はこのタスクの後に着手する。
