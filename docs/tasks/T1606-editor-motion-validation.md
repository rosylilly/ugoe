---
id: T1606
title: "エディタ: プロジェクトのモーションと表情も検証し、拒んだ理由をコードで出す"
status: todo
milestone: M9
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee:
---

# T1606: エディタ: プロジェクトのモーションと表情も検証し、拒んだ理由をコードで出す

## 目的

T0323 でモーションと表情の数値の検証 (`validate_motion`・`validate_expression`) を足し、`ugoe-format` の読み込みは
`FormatError` の新しい 2 つの変種で拒むようになった。エディタには 2 つの漏れがある。

- `.ugoeproj` のモーション・表情は `ugoe-editor-core` の `project/files.rs` が `serde_json` で直接読むので検証されない
  (書き出したバンドルを読み直すと拒まれる)。
- `message_codes.rs` は `FormatError` の知らない変種を `format.corrupt` にまとめるので、新しい 2 つは「壊れている」と出る。
  `format.invalid_motion`・`format.invalid_expression` のコードと翻訳を足す。

## 受け入れ条件

- [ ] プロジェクトの読み込みで NaN を含むモーション・表情が拒まれ (場所つき)、テストがある
- [ ] `format.invalid_motion`・`format.invalid_expression` のコードと翻訳を足し、`docs/editor.md#エラーと警告のコード` を更新した (メッセージの一覧の検査 T0681 が通る)
- [ ] `just ci` が通る

## 参考

- T0323 のメモ (起票の候補)
- crates/ugoe-editor-core/src/project/files.rs・message_codes.rs

## メモ

- T0019 で起票した (元の候補: T0323 のエディタのコードと `.ugoeproj` の検査)。
