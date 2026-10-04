---
id: T0463
title: "@ugoe/runtime: MotionSampler を出すかと、本物の wasm で *_json() の出力のキーを型と比べる"
status: in-progress
milestone: M9
crates: [bindings/web, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0463
---

# T0463: @ugoe/runtime: MotionSampler を出すかと、本物の wasm で *_json() の出力のキーを型と比べる

## 目的

- T0451 で wasm に `MotionSampler` を足したが、`bindings/web/src/index.ts` からは出していない (`sample_motion_json` だけ。T0451 の候補)。
- T0452 で `*_json()` の戻り値の TS の型 (`queries.ts`) を足したが、型と実際のキーは Rust の doc と手で合わせている。本物の wasm
  (適合テストの `wasm`) で出力のキーを型と比べる検査が無い (T0452 の候補)。

## 受け入れ条件

- [ ] `MotionSampler` を `@ugoe/runtime` から出すかを決め、出すなら型と README の使い方とテストがある (出さないなら理由を README に)
- [ ] 適合テスト (または bindings/web のテスト) が、本物の wasm の `*_json()` の出力のキーを `queries.ts` の型のキーと比べ、食い違いで落ちる
- [ ] `just web-check`・`just conformance`・`just ci` が通る

## 参考

- T0451・T0452 のメモ (起票の候補)、T0435 (手書きの型の写しと生成物の比較)

## メモ

- T0026 で起票した (元の候補: T0451 の `MotionSampler` の公開、T0452 の `*_json()` のキー)。
