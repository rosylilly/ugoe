---
id: T0336
title: "内部の整理: ugoe-physics の積分と衝突の部品を pub(crate) にし、Chain・Particle を読むだけの型にする (T0316 の所見)"
status: in-progress
milestone: M9
crates: [ugoe-physics, ugoe-wasm, ugoe, docs]
depends_on: []
assignee: claude-worker-T0336
---

# T0336: 物理の公開の範囲を狭める

## 目的

T0316 (物理のレビュー) が見送った「公開の範囲の見直し」を、T0195 が洗い出した。積分の 1 ステップや衝突の補助などの実装の部品が `pub` のままで、
`Chain` も外から作って進められる。約束のためではなく内部の整理として (利用者の方針 2026-10-11。docs/release.md#公開の範囲)、
部品を `pub(crate)` にし、`Chain`・`Particle` は外から読むだけの型にする。

## 受け入れ条件

- [ ] モジュール `chain`・`collider` を非公開にし、`Chain`・`Particle` を根で再公開した
- [ ] `Chain::new`・`settle`・`step`、`Segment`・`StepInput`・`WorldCollider` (`blocks`・`push_out` を含む)・`SweepBuffer`・`constrain_segment`・
      `SWEEP_MAX_STEP_DEGREES`・`signed_angle`・`rest_offset`・`ROOT_ROTATION_FOLLOW`・`BASE_FPS`・`MAX_STEPS_PER_UPDATE` を `pub(crate)` にした
- [ ] `Chain` は `particles`・`is_finite` などの読むメソッドだけを公開にした。`ugoe-wasm` の物理の表示 (`PhysicsSystem::chain` → `particles`) と
      `crates/ugoe/tests/m3_puppet.rs` がそのまま使える (使えなければ使う側を直す)
- [ ] 狭めて使われなくなった項目は消すかテストの中へ移した (`#[allow(dead_code)]` で黙らせない)。単体テストは今の数値のまま通る
- [ ] crates/ugoe-physics/README.md の `chain::Chain`・`collider::…` の説明を直した (docs/spec/physics.md の `ROOT_ROTATION_FOLLOW`・`MAX_STEPS_PER_UPDATE` は
      値の説明として残す。docs/testing.md の `WorldCollider::blocks` は内部の時間の内訳の説明なのでそのまま)
- [ ] `just wasm-check`・`just nostd-check` が通る
- [ ] `just ci` が通る

## 参考

- T0195 のメモ (ugoe-physics の表・起票の候補 5)、T0316 のメモ (見送った所見)、docs/release.md#公開の範囲
- docs/spec/physics.md

## メモ

- T0095 で起票した (元の候補: T0195 (physics: T0316 の内容。Chain・Particle は読むだけで残し、Chain::new・settle・step・Segment・StepInput・
  WorldCollider・constrain_segment・signed_angle・rest_offset などを pub(crate)、モジュール chain・collider を非公開。README も))。
- M9 の後続 (T0195・T0316 と同じ)。番号は物理・モーションの帯の空き。互換の別名・移行の説明・semver の注記は要らない (利用者の方針 2026-10-11)。
- `crates:` の `ugoe-wasm`・`ugoe` は、使う側の追従 (呼び方を変える小さな直し) だけ。
