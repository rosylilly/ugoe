---
id: T1600
title: editor-core のエラー型が持つ ID を型付きにするか決める
status: todo
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: [T0678]
assignee:
---

# T1600: editor-core のエラー型が持つ ID を型付きにするか決める

## 目的

T0142 で、モデルの中の文字列のままの ID の参照を `Id<T>` にそろえた。`ugoe-editor-core` のエラー型 (`EditError::NotFound` など)・
`ModuleError::ColliderNeedsBasePhysics`・`ColliderReference` は、まだ ID を `String` で持つ。型付きにすれば、取り違え (パーツの ID を
デフォーマの ID として報告する) を型で防げるが、エラーの引数 (ADR 0029 の `args`) は文字列で出すので、得るものは小さいかもしれない。

**判断が要る:** 型付きにするか。しないなら理由を `crates/ugoe-editor-core/README.md` に書いて閉じる。

## 受け入れ条件

- [ ] 対象の型と、型付きにしたときの変更の大きさ (呼び出し側の数) をメモに書き、するか決めた
- [ ] するなら置き換え、エラーの `code` と `args` (JSON) が変わらないことをテストで確かめた
- [ ] `just ci` が通る

## 参考

- T0142 のメモ (起票の候補)
- crates/ugoe-editor-core/src/document/error.rs・module_project.rs

## メモ

- T0019 で起票した (元の候補: T0142 の editor-core のエラー型)。エラー型を触る T0678 の後に着手する。
