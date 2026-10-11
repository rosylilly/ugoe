---
id: T0087
title: "fuzz.yml: CI で cargo-fuzz が musl のターゲットでビルドして、5 つのターゲットがすべて fuzz の前に落ちるのを直す"
status: in-progress
milestone: M11
crates: [.github/workflows, fuzz, justfile, docs]
depends_on: []
assignee: claude-worker-T0087
---

# T0087: fuzz.yml のビルドの失敗を直す

## 目的

2026-10-11 に初めて手で起動した fuzz.yml (run 38098547009、main d7b0aaeb) で、5 つのターゲット (bundle・module・image・json・ffi) がすべて fuzz を始める前のビルドで落ちた。
cargo-fuzz が `cargo build --target x86_64-unknown-linux-musl` を呼び、`sanitizer is incompatible with statically linked libc` と `can't find crate for std` で止まる。
taiki-e/install-action が入れたビルド済みの cargo-fuzz が musl 向けで、既定のターゲット (ホストの triple) を musl と見ていると見られる (T0085 のメモ)。手元の `just fuzz` は通る。

## 受け入れ条件

- [ ] 原因を確かめた (cargo-fuzz の既定のターゲットの決め方。出典をメモに書く)
- [ ] 直した: `fuzz/run.mjs` (または just のレシピ) がホストの gnu のターゲット (`rustc -vV` の host) を `--target` で明示する、または CI で cargo-fuzz を gnu 向けに入れる。
      手元 (Linux・macOS・Windows の扱い) で振る舞いが変わらないこと
- [ ] 手元で `just fuzz <ターゲット> 10` と `just fuzz-check` が通る。musl の cargo-fuzz を模した状況 (`--target` を外したときに musl になる) で直す前は落ちることを、できる範囲で確かめた
- [ ] docs/testing.md#fuzz と fuzz/README.md を直した
- [ ] `just ci` が通る
- [ ] (人の手かオーケストレーター) push の後に fuzz.yml を workflow_dispatch で流し直して通ることを確かめる

## 参考

- .github/workflows/fuzz.yml、fuzz/run.mjs、ADR 0032、T0085 のメモ (定期のワークフローの初回)

## メモ

- オーケストレーターが、初回の fuzz.yml の失敗から起票した (利用者の方針: CI の失敗は起票してすぐ直す)。
