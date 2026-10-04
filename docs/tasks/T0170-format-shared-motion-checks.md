---
id: T0170
title: ugoe-format の check_motion・check_expression を公開し、editor-core の写しを置き換える
status: todo
milestone: M9
crates: [ugoe-format, ugoe-editor-core, docs]
depends_on: [T1607]
assignee:
---

# T0170: ugoe-format の check_motion・check_expression を公開し、editor-core の写しを置き換える

## 目的

T1606 でエディタのプロジェクトのモーション・表情も検証するようにしたが、`crates/ugoe-editor-core/src/project.rs` の `check_motion`・`check_expression` は
`crates/ugoe-format/src/lib.rs` の同じ名前の非公開の関数の写しになっている (T1606 の候補)。片方だけ直る食い違いを避ける。

## 受け入れ条件

- [ ] `ugoe-format` が `check_motion`・`check_expression` (または同じ働きの関数) を公開し、doc コメントがある
- [ ] editor-core の写しを消し、公開した関数を使う。既存のテスト (拒む理由のコード `format.invalid_motion`・`format.invalid_expression`) がそのまま通る
- [ ] `ugoe-format` の README を更新した
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T1606 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T1606)。editor-core の `project.rs` を触る T1607 の後に着手する。
- T0015 の候補「`ProjectError::InvalidModule` の文を `describe_errors` に、`format.invalid_*` のコード、`message_codes.rs` が `validation_errors()` で引数を作る」は、
  T0163・T1606 で入った (`project.rs` の `InvalidModule`・`message_codes.rs` の `validation_args`) ので起票しない。
