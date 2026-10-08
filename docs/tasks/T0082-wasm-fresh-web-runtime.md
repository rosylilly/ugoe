---
id: T0082
title: "wasm-fresh が bindings/web の wasm の生成物 (src/wasm/) の古さも確かめ、古ければ作り直す"
status: done
milestone: M10
crates: [scripts, bindings/web, justfile, docs]
depends_on: []
assignee: claude-worker-T0082
---

# T0082: wasm-fresh が bindings/web の wasm の生成物の古さも確かめる

## 目的

T0062 で `just wasm-fresh` (scripts/wasm.mjs の `fresh`) を作り、web-check と pre-push の前にアプリ (エディタ・ビューア・スタジオ) の wasm の生成物の古さを
指紋で確かめるようにした。ところが `@ugoe/runtime` (bindings/web) の生成物 `bindings/web/src/wasm/` (`pnpm --filter @ugoe/runtime build` が作る。gitignore 済み) は
対象に入っていない。bindings/web/scripts/check.mjs は wasm 本体 (`ugoe_wasm_bg.wasm`) があれば生成物で型を検査するので、
前に一度ビルドした古い生成物が残っていると、新しい wasm の口 (例: T1402 の `WebPuppet.shape_changed`) を使う `avatar.ts` の型検査が落ちる
(2026-10-09 に main の `just ci` の web-check で `Property 'shape_changed' does not exist on type 'WebPuppet'` が出た。利用者の pre-push でも同じことが起こりうる)。

## 受け入れ条件

- [x] bindings/web の生成物も、アプリと同じ指紋の仕組みで古さを確かめ、古ければ作り直す (または、作り直す方法が分かるエラーで止める。アプリと同じ扱いにそろえる)。
      生成物の無いときは今と同じくスタブで検査する
- [x] bindings/web/scripts/build.mjs が生成物の隣に指紋を書く (scripts/wasm.mjs の `build` と同じもの。重複させずに共有する)
- [x] scripts/wasm.mjs の単体テスト (あれば) に bindings/web の場合を足した。古い生成物を置いたまま `just web-check` が通ることを手で確かめ、メモに書いた
- [x] docs/testing.md#wasm-の生成物の古さ と bindings/web/README.md を直した
- [x] `just ci` が通る

## 参考

- T0062 のメモ、scripts/wasm.mjs、bindings/web/scripts/check.mjs・build.mjs、justfile の `wasm-fresh`・`web-check`、lefthook.yml の pre-push

## メモ

- オーケストレーターが main の `just ci` で見つけて起票した (利用者の「古い wasm で web-check が落ちる → 起票して修正」の方針、T0062)。
- やったこと (2026-10-09):
  - `scripts/wasm.mjs` に `WEB_RUNTIME_OUT` (`bindings/web/src/wasm`) を足し、`check`・`fresh` が 4 つの生成物 (`OUTS`) を確かめる。
    指紋はアプリと同じもの。`bindings/web/scripts/build.mjs` は `scripts/wasm.mjs` の `fingerprint`・`writeFingerprint`・
    `requireWasmBindgen` を使って生成物の隣に `ugoe_wasm.fingerprint` を書く (wasm-bindgen の版の検査の重複も消した)。指紋は `dist/wasm/` に写さない。
  - 古い `@ugoe/runtime` の生成物は止めずに作り直す (アプリと同じ扱い)。配布用のプロファイル `web-dist` で作るので、`fresh` は
    `build.mjs --wasm-only` (新設。wasm の生成だけで `dist/` は作らない) を呼ぶ。wasm-bindgen-cli が無ければ、アプリと同じく入れ方と
    `rm -r bindings/web/src/wasm` を出して止まる。
  - 生成物かどうかを、ディレクトリが空でないかでなく wasm 本体 (`ugoe_wasm_bg.wasm`) の有無で見分けるようにした
    (`bindings/web/scripts/check.mjs` が生成物の無いときに置くスタブの写しを、指紋の無い古い生成物と取り違えないため。アプリも同じ判定にした)。
  - `bindings/web/scripts/check.mjs` は、生成物が古ければ作り直し方の警告を出す (検査は続ける。`just web-check` は先に作り直すので出ない)。
  - 単体テスト (`scripts/wasm.test.mjs`) に、`@ugoe/runtime` の生成物の古さ・指紋の無い生成物・型だけのスタブの写しを飛ばすことを足した。
  - 手で確かめたこと: worktree の `bindings/web/src/wasm` に、指紋の無い古い生成物 (main の生成物の写しから `shape_changed(): boolean;` を消したもの) を置くと、
    直す前と同じく `pnpm --filter @ugoe/runtime check` が `src/avatar.ts(947,46): error TS2339: Property 'shape_changed' does not exist on type 'WebPuppet'.` で落ち
    (直した後は加えて古さの警告が出る)、`just web-check` は `bindings/web/src/wasm: no ugoe_wasm.fingerprint` と出して作り直し、通った。
    2 回目の `just wasm-fresh` は「up to date」で 1 秒かからない。`just wasm <out>` の指紋と `build.mjs` の指紋が一致することも見た。
    作り直しは mbx の置き場に当たって約 5 秒 (当たらなければ fat LTO で約 2 分。mbx の表示の「1m 50s saved」から)。
  - 範囲 (`crates:`) の外の小さな変更: lefthook.yml の pre-push の web-check に `--filter @ugoe/runtime` を足した (wasm-fresh が作り直す
    生成物を pre-push でも検査するため。tsc だけで軽い) のと、AGENTS.md のフックの説明の 1 行。
- やらなかったこと: wasm-bindgen-cli の無い機械での止まり方 (`requireWasmBindgen` は T0062 と同じ経路) は手で試していない。
- `just ci` は通った (2026-10-09、rebase 後の 889bee48)。

