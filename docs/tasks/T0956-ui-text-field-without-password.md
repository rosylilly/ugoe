---
id: T0956
title: "@ugoe/ui: TextField の type から password を外し、秘密の欄は SecretField だけにする"
status: todo
milestone: M10
crates: [packages/ui, docs]
depends_on: []
assignee:
---

# T0956: @ugoe/ui: TextField の type から password を外し、秘密の欄は SecretField だけにする

## 目的

T0949 で秘密の欄 `SecretField` (見せる・隠すの切り替え・候補を出さない) が入ったが、`TextField` の `type` にも `"password"` が残っていて、
秘密の欄を 2 通りに作れる (T0949 の候補)。今は使っている所が無いので、`TextField` から外して `SecretField` だけにする。

## 受け入れ条件

- [ ] `TextField` の `type` が `"text" | "search" | "url"` になり、`password` のテスト (`fields.test.ts`) を外した。アプリに `TextField` の `type="password"` が無いことを確かめた
- [ ] docs/ui.md の部品の一覧 (`TextField`・`SecretField`) を今の形に直した
- [ ] `pnpm --filter @ugoe/ui test` と `just ci` が通る

## 参考

- T0949 のメモ (起票の候補)
- packages/ui/src/components/TextField.svelte・SecretField.svelte、packages/ui/src/fields.test.ts、docs/ui.md

## メモ

- T0043 で起票した (元の候補: T0949 (`TextField` の `password`))。
