---
id: T0074
title: "fuzz/Cargo.lock の依存の版を、ルートの Cargo.lock とそろえるかを決め、そろえるならずれを検査する"
status: in-progress
milestone: M11
crates: [fuzz, scripts, docs]
depends_on: [T0073]
assignee: claude-worker-T0074
---

# T0074: fuzz/Cargo.lock の依存の版を、ルートの Cargo.lock とそろえるかを決め、そろえるならずれを検査する

## 目的

ワークスペースの外の `fuzz/Cargo.lock` の版が、ルートの `Cargo.lock` と違うものがある (例: glam 0.33.12 と 0.33.11。T0072 の候補)。
fuzz が回すコードと出荷するコードで依存の版が違うと、fuzz が見つけた (あるいは見つけない) 不具合が出荷物と食い違うおそれがある。
`bindings/godot/Cargo.lock` も同じ形。

## 受け入れ条件

- [ ] そろえるか (ルートの版に合わせる・ずれを検査で止める) を、作業者が決めて理由をメモと docs/testing.md に書いた
- [ ] そろえるなら、共通の依存の版がルートと同じになるよう lock を直し、ずれを見つける検査 (`just groups` か `just fuzz-lint` など) を足した。検査がずれを見つけることを一時的な変更で確かめた
- [ ] `just fuzz-lint` と `just ci` が通る

## 参考

- T0072 のメモ、fuzz/Cargo.lock、bindings/godot/Cargo.lock、scripts/check-licenses.mjs

## メモ

- T0072 のワーカーの報告から起票した (2026-10-07)。check-licenses.mjs を触る T0073 の後にした。
