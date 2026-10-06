---
id: T0492
title: "@ugoe/runtime: エディタ向けの問い合わせ (rotation_local_keyform など) を公開の API に含めるかを決める (判断が要る)"
status: done
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

- [x] 利用者に案を示して方針を決め、決めたことと理由を bindings/web/README.md と crates/ugoe-wasm/README.md (「エディタ向けの問い合わせ」) に書いた
- [x] 決めた方針に合わせて型 (`bindings/web/stub/ugoe_wasm.d.ts`) と、要るならビルドの設定・`scripts/check-wasm-types.mjs` を直した
- [x] `pnpm --filter @ugoe/runtime check`・`just wasm-check` と `just ci` が通る

## 参考

- T1660 のメモ (起票の候補)、ADR 0025 (@ugoe/runtime)
- crates/ugoe-wasm/src/web.rs・src/gizmo.rs、crates/ugoe-wasm/README.md、bindings/web/README.md、bindings/web/src/index.ts

## メモ

- T0058 で起票した (元の候補: T1660 (@ugoe/runtime で rotation_local_keyform を公開するか))。
- 判断 (2026-10-07): 利用者が案 B (メソッドは残し、型とドキュメントに「エディタ向け。予告なく変わる」と明記して公開の API の外とする) を選んだ。
- やったこと:
  - エディタ向けの問い合わせ 7 つ (`deformer_ids`・`warp_points`・`rotation_frame`・`rotation_local_keyform`・`deformer_point`・
    `deformer_jacobian`・`physics_particles`) の Rust の doc コメント (src/web.rs・src/web/physics.rs) に `@internal エディタ向け。予告なく変わる` を足した。
    wasm-bindgen がそのまま生成物の JSDoc に写すことを `just wasm` で確かめた。手書きの写し bindings/web/stub/ugoe_wasm.d.ts にも同じ印を付けた。
  - scripts/check-wasm-types.mjs: メンバーの JSDoc の `@internal` の有無を読み、全体の写しと生成物で違えば報告する (テストを追加)。
    bindings/web/stub/ugoe_wasm.test.mjs: Rust の doc コメントの `@internal` と写しの印が同じメソッドかを比べる (生成物が無い環境でも効く)。
  - 方針と理由を crates/ugoe-wasm/README.md (「エディタ向けの問い合わせ」。`deformer_ids` も一覧に足した) と bindings/web/README.md
    (「公開の API の範囲」) に書いた。
  - `crates:` の外: scripts/check-wasm-types.mjs と test (型の写しの検査。小さな変更なので行った)。
- やらなかったこと: 名前の前置き・`stripInternal` で型から消すことはしていない (案 B のとおり型には残す)。配布物から外したときの
  wasm の大きさは測っていない (README には「薄い口で得が小さい。測っていない」と書いた)。
- `just ci`・`just wasm-check`・`pnpm --filter @ugoe/runtime check`・生成物との `just wasm-types` が通った。
- 起票の候補: なし。
