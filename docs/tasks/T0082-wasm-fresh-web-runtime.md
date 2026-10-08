---
id: T0082
title: "wasm-fresh が bindings/web の wasm の生成物 (src/wasm/) の古さも確かめ、古ければ作り直す"
status: todo
milestone: M10
crates: [scripts, bindings/web, justfile, docs]
depends_on: []
assignee:
---

# T0082: wasm-fresh が bindings/web の wasm の生成物の古さも確かめる

## 目的

T0062 で `just wasm-fresh` (scripts/wasm.mjs の `fresh`) を作り、web-check と pre-push の前にアプリ (エディタ・ビューア・スタジオ) の wasm の生成物の古さを
指紋で確かめるようにした。ところが `@ugoe/runtime` (bindings/web) の生成物 `bindings/web/src/wasm/` (`pnpm --filter @ugoe/runtime build` が作る。gitignore 済み) は
対象に入っていない。bindings/web/scripts/check.mjs は wasm 本体 (`ugoe_wasm_bg.wasm`) があれば生成物で型を検査するので、
前に一度ビルドした古い生成物が残っていると、新しい wasm の口 (例: T1402 の `WebPuppet.shape_changed`) を使う `avatar.ts` の型検査が落ちる
(2026-10-09 に main の `just ci` の web-check で `Property 'shape_changed' does not exist on type 'WebPuppet'` が出た。利用者の pre-push でも同じことが起こりうる)。

## 受け入れ条件

- [ ] bindings/web の生成物も、アプリと同じ指紋の仕組みで古さを確かめ、古ければ作り直す (または、作り直す方法が分かるエラーで止める。アプリと同じ扱いにそろえる)。
      生成物の無いときは今と同じくスタブで検査する
- [ ] bindings/web/scripts/build.mjs が生成物の隣に指紋を書く (scripts/wasm.mjs の `build` と同じもの。重複させずに共有する)
- [ ] scripts/wasm.mjs の単体テスト (あれば) に bindings/web の場合を足した。古い生成物を置いたまま `just web-check` が通ることを手で確かめ、メモに書いた
- [ ] docs/testing.md#wasm-の生成物の古さ と bindings/web/README.md を直した
- [ ] `just ci` が通る

## 参考

- T0062 のメモ、scripts/wasm.mjs、bindings/web/scripts/check.mjs・build.mjs、justfile の `wasm-fresh`・`web-check`、lefthook.yml の pre-push

## メモ

- オーケストレーターが main の `just ci` で見つけて起票した (利用者の「古い wasm で web-check が落ちる → 起票して修正」の方針、T0062)。
