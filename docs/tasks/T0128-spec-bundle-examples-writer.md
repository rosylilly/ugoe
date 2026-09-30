---
id: T0128
title: 公開仕様: .ugoe の例のファイルと、独立の書き出し・往復の試験
status: todo
milestone: M5
crates: [spec, tools/spec-check, docs]
depends_on: [T0120]
assignee:
---

# T0128: 公開仕様: .ugoe の例のファイルと、独立の書き出し・往復の試験

## 目的

T0120 の後続の候補。バイナリの資料は `assets/compat/` にあるが仕様の一部ではなく、`spec/examples/` は JSON だけ。
また書き出し (spec/bundle.md#writing) は独立の実装がない。

## 受け入れ条件

- [ ] 作業者は `crates/` と `docs/spec/` を読まず、`spec/` だけを参照する
- [ ] `spec/examples/` に小さな `.ugoe` (と `.ugoemod`) の例を置く (圧縮あり・なし、テクスチャ、機能の宣言)。大きさを抑える
- [ ] `tools/spec-check` に書き出しを足し、独立の書き出し → 参照実装の読み込み (`ugoe validate` など黒箱で) → 独立の読み込み、の往復で内容が保たれることを確かめる
- [ ] 見つかった仕様の抜け・曖昧さの一覧と `spec/` への修正 (`docs/spec/` の同期は team-lead に頼む)
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0120-spec-independent-reader.md`・`spec/bundle.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
