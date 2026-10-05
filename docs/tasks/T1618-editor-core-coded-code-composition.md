---
id: T1618
title: "editor-core: Coded::code を元のクレートのコードから組み立て、手で並べたコードの一覧をなくす"
status: todo
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: [T0168]
assignee:
---

# T1618: editor-core: Coded::code を元のクレートのコードから組み立て、手で並べたコードの一覧をなくす

## 目的

T0167 で合成のエラーのコードを `ugoe-runtime` の `ComposeError::code()` から作るようにしたが、editor-core の `Coded::code` は `&'static str` なので、
元のクレートのコードに領域の名前 (`validation.`・`infer.`・`world.`・`normal_map.`・`compose.`) を付けたものを、手で並べた一覧で持っている (T0167 の候補)。
`Coded::code` を `Cow<'static, str>` などにして組み立てれば、元のクレートにコードが増えても一覧を直さずに済む。同じ着せ替えのエラーを触る T0168 の後に着手する。

## 受け入れ条件

- [ ] `Coded::code` が元のクレートのコードと領域の名前から組み立てたコードを返し、`validation`・`infer`・`world`・`normal_map`・`compose` の手で並べた一覧が無い
- [ ] 出すコードの文字列は今と同じ (エディタの訳の表と `codes.json` を変えない)。元のクレートのすべてのコードについて今と同じ文字列になることを確かめるテストがある
- [ ] `just ci` が通る

## 参考

- T0167 のメモ (起票の候補)
- ADR 0029

## メモ

- T0032 で起票した (元の候補: T0167)。
