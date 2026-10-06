---
id: T0492
title: "@ugoe/runtime: エディタ向けの問い合わせ (rotation_local_keyform など) を公開の API に含めるかを決める (判断が要る)"
status: in-progress
milestone: M10
crates: [bindings/web, ugoe-wasm, docs]
depends_on: []
assignee: claude-worker-T0492
---

# T0492: @ugoe/runtime: エディタ向けの問い合わせ (rotation_local_keyform など) を公開の API に含めるかを決める (判断が要る)

## 目的

T1660 で `WebPuppet::rotation_local_keyform` (回転デフォーマの補間した局所の値) を wasm に足した。@ugoe/runtime (bindings/web) は
`WebPuppet` をそのまま出しているので、このメソッドも、前からある「エディタ向けの問い合わせ」(`warp_points`・`rotation_frame`・
`deformer_point`・`deformer_jacobian` など。crates/ugoe-wasm/README.md の「エディタ向けの問い合わせ」) も、npm の利用者から呼べて型にも出る。
これらを @ugoe/runtime の公開の API (版を上げるときに互換を守る範囲) に含めるかの方針が決まっていない。

**判断が要る** (公開 API の範囲)。作業者は下の案から勝手に選ばず、利用者に確かめてから進める。

### 案

- **A. 公開の API に含める:** 今のまま `WebPuppet` に置き、bindings/web/README.md に「ツール向けの問い合わせ」として載せる。
  変えるときは公開の API の変更として扱う。手間は少ないが、エディタの都合 (ギズモの形) で変えたいときの縛りが増える。
- **B. 呼べるが公開の API の外とする:** メソッドは残し、型 (`stub/ugoe_wasm.d.ts` と生成物) とドキュメントに「エディタ向け。予告なく変わる」と書く
  (TypeScript の `@internal` や名前の前置きなど)。wasm の大きさは変わらない。
- **C. 配布物から外す:** ugoe-wasm にエディタ向けの問い合わせの feature (例: `editor-queries`) を作り、エディタの `just wasm` だけで入れる。
  @ugoe/runtime の wasm と型には出ない (大きさも少し減る)。ビルドの設定と型の写し (`scripts/check-wasm-types.mjs`) が 2 通りになる。

### 推奨

B (メソッドは残し、公開の API の外と明記する)。エディタの問い合わせはギズモの形に合わせて変わりやすく、リリース前に縛るのは早い。
C は wasm を 2 通り作る手間に対して大きさの得が小さい (bindings/web/README.md#wasm-の大きさ の見積もりで確かめる)。

## 受け入れ条件

- [ ] 利用者に案を示して方針を決め、決めたことと理由を bindings/web/README.md と crates/ugoe-wasm/README.md (「エディタ向けの問い合わせ」) に書いた
- [ ] 決めた方針に合わせて型 (`bindings/web/stub/ugoe_wasm.d.ts`) と、要るならビルドの設定・`scripts/check-wasm-types.mjs` を直した
- [ ] `pnpm --filter @ugoe/runtime check`・`just wasm-check` と `just ci` が通る

## 参考

- T1660 のメモ (起票の候補)、ADR 0025 (@ugoe/runtime)
- crates/ugoe-wasm/src/web.rs・src/gizmo.rs、crates/ugoe-wasm/README.md、bindings/web/README.md、bindings/web/src/index.ts

## メモ

- T0058 で起票した (元の候補: T1660 (@ugoe/runtime で rotation_local_keyform を公開するか))。
- 判断 (2026-10-07): 利用者が案 B (メソッドは残し、型とドキュメントに「エディタ向け。予告なく変わる」と明記して公開の API の外とする) を選んだ。
