---
id: T0435
title: ugoe-wasm の手書きの型の写しを生成物と比べる
status: in-progress
milestone: M9
crates: [bindings/web, packages, apps/editor, apps/viewer, apps/studio, scripts, docs]
depends_on: [T0424]
assignee: claude-worker-T0435
---

# T0435: ugoe-wasm の手書きの型の写しを生成物と比べる

## 目的

wasm-bindgen の生成物が無い環境で型検査を通すため、`WebPuppet` の型の手書きの写しが 5 か所にある:
`bindings/web/stub/ugoe_wasm.d.ts`・`apps/viewer/src/lib/wasm/stub.d.ts`・`apps/editor/src/lib/viewport/ugoe-wasm-stub.d.ts`・
`apps/studio/src/lib/wasm/stub.d.ts`・`packages/player/src/wasm.ts`。すでにずれている (T0424 の時点で `bindings/web` の写しに
`avatar_slots_json` が無い)。公開 API を変えるたびに手で直すのは漏れやすいので、生成物と写しを比べる検査を用意するか、写しを 1 つにまとめる。
T0424 (ugoe-wasm のレビュー) で見つけた。

## 受け入れ条件

- [ ] 写しを減らす (共有できるものは 1 つの写しから使う) か、残す理由を書く
- [ ] `just wasm` で作った `ugoe_wasm.d.ts` と写しを比べ、メソッドの抜け・型の違いを報告するスクリプト (CI の wasm を作るジョブで走らせる)
- [ ] `bindings/web/stub/ugoe_wasm.d.ts` に `avatar_slots_json(): string` を足す
- [ ] `just ci` が通る

## 参考

- `bindings/web/scripts/check.mjs` (T0003 は写しを写し直す仕組みを直した。写しと生成物のずれは扱っていない)
- `crates/ugoe-wasm/README.md#ビルド`

## メモ
