---
id: T0159
title: ugoe-core の Id<T> を文字列と比べられるようにする
status: in-progress
milestone: M9
crates: [ugoe-core, docs]
depends_on: []
assignee: claude-worker-T0159
---

# T0159: ugoe-core の Id<T> を文字列と比べられるようにする

## 目的

T0142 で文字列のままの ID の参照を `Id<T>` にそろえた結果、テストや呼び出し側に `id.as_str() == "..."` が増えた。
`Id<T>` に `PartialEq<str>`・`PartialEq<&str>` (と逆向き) を足せば `id == "..."` と書ける。

**判断が要る:** `ugoe-core` の公開 API の追加 (crates.io に出すクレート)。型の違う ID どうしを比べられないという `Id<T>` の目的を
損なわないか (文字列との比較は型を問わない) を確かめて決める。足さないと決めたら、理由を `crates/ugoe-core/README.md` に書いて閉じる。

## 受け入れ条件

- [ ] 足すか決めた (上の判断)。足すなら `PartialEq<str>`・`PartialEq<&str>`・`PartialEq<Id<T>> for str`/`&str` を実装し、doc コメントとテストを書いた
- [ ] 足したなら、ワークスペースの `as_str() == "..."` の形を置き換えた (`crates:` の外のクレートのテストは、置き換えずに件数をメモに書くか、別のタスクにする)
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- T0142 のメモ (起票の候補)
- crates/ugoe-core/src/id.rs (`Id<T>`)

## メモ

- T0019 で起票した (元の候補: T0142)。
