---
id: T0498
title: "ugoe-ffi: C ABI のヘッダの関数と fuzzing.rs が呼ぶ関数の一覧を比べ、fuzz の入れ忘れを検査で止める"
status: in-progress
milestone: M11
crates: [ugoe-ffi, scripts, docs]
depends_on: []
assignee: claude-worker-T0498
---

# T0498: ugoe-ffi: C ABI のヘッダの関数と fuzzing.rs が呼ぶ関数の一覧を比べ、fuzz の入れ忘れを検査で止める

## 目的

T0497 で `ugoe_ffi::fuzzing::run` が C ABI の関数を一通り呼ぶ fuzz の入口を足したが、C ABI に関数を足したときに fuzzing.rs に入れ忘れても
気づけない (T0497 の候補)。

## 受け入れ条件

- [ ] ヘッダ (`crates/ugoe-ffi` の生成されたヘッダ) の関数と、fuzzing.rs が呼ぶ関数を比べ、呼んでいない関数があれば名前を出して失敗する検査を足した (`just refs` の ffi の検査 (`scripts/check-ffi-bindings.mjs`) に足すのが素直)。呼ばなくてよい関数 (`ugoe_abi_version` など) は理由付きの除外の一覧に置く
- [ ] 一時的に関数を 1 つ外して、検査が見つけることを確かめた
- [ ] crates/ugoe-ffi/README.md (関数を足すときの手順) に書いた
- [ ] `just refs`・`just ffi-check`・`just ci` が通る

## 参考

- T0497 のメモ、crates/ugoe-ffi/src/fuzzing.rs、scripts/check-ffi-bindings.mjs

## メモ

- T0497 のワーカーの報告から起票した (2026-10-07)。
