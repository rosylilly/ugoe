---
id: T0956
title: "@ugoe/ui: TextField の type から password を外し、秘密の欄は SecretField だけにする"
status: done
milestone: M10
crates: [packages/ui, docs]
depends_on: []
assignee: claude-worker-T0956
---

# T0956: @ugoe/ui: TextField の type から password を外し、秘密の欄は SecretField だけにする

## 目的

T0949 で秘密の欄 `SecretField` (見せる・隠すの切り替え・候補を出さない) が入ったが、`TextField` の `type` にも `"password"` が残っていて、
秘密の欄を 2 通りに作れる (T0949 の候補)。今は使っている所が無いので、`TextField` から外して `SecretField` だけにする。

## 受け入れ条件

- [x] `TextField` の `type` が `"text" | "search" | "url"` になり、`password` のテスト (`fields.test.ts`) を外した。アプリに `TextField` の `type="password"` が無いことを確かめた
- [x] docs/ui.md の部品の一覧 (`TextField`・`SecretField`) を今の形に直した
- [x] `pnpm --filter @ugoe/ui test` と `just ci` が通る

## 参考

- T0949 のメモ (起票の候補)
- packages/ui/src/components/TextField.svelte・SecretField.svelte、packages/ui/src/fields.test.ts、docs/ui.md

## メモ

- T0043 で起票した (元の候補: T0949 (`TextField` の `password`))。
- T0956 で行ったこと:
  - `TextField` の `type` を `"text" | "search" | "url"` にし、doc コメントで秘密は `SecretField` と案内した。
  - `fields.test.ts` の `password` の検査 (`is a labelled password box that does not suggest`) は `SecretField` のもの
    (`TextField` の `password` の検査はもともと無かった) なので残した。代わりに `components.test.ts` に、`TextField` の `type` が
    `"text" | "search" | "url"` であることの型の検査 (`expectTypeOf`。svelte-check が見る) を足した。`password` を戻すと svelte-check が失敗することを確かめた。
  - アプリ・packages・bindings に `TextField` の `type="password"` (と標準の `<input type="password">`) が無いことを grep で確かめた
    (`password` は `SecretField` の中だけ)。移すものは無かった。
  - docs/ui.md の部品の一覧: `TextField` に `type` の値を、`SecretField` に「秘密の欄はこの部品だけで作る」を書いた。
  - `pnpm --filter @ugoe/ui test` (141 件)・`check`・`just ui-shots` (見た目の変化なし)・`just ci` が通った。
    アプリの画面は触っていないので `just e2e` は流していない。
- 起票の候補: なし。
