---
id: T0121
title: 着せ替え: Puppet の API と合成の残り
status: in-progress
milestone: M4
crates: [ugoe, ugoe-runtime, ugoe-model, docs]
depends_on: [T0111, T0113]
assignee: claude-worker-T0121
---

# T0121: 着せ替え: Puppet の API と合成の残り

## 目的

T0111 で作った合成 (`compose`) を、ランタイム SDK から使えるようにし、T0111 で残した合成の制限を解く。

## 受け入れ条件

- [ ] `Puppet` にモジュールの付け外しの API があり、組み替えでパラメータの値と物理の状態の扱いが決まっている (`ParameterValues::carry_over` を使う)
- [ ] モジュールの最上位のパーツを、本体のパーツ (スロットで指定) にぶら下げられる
- [ ] 本体の衝突体をスロットで公開し、モジュールの物理がそれに当たる
- [ ] モジュールの要素をポーズのグループに入れられる
- [ ] `docs/spec/modules.md` を更新し、`just ci`・`just wasm-check` が通る

## 参考

- `docs/tasks/T0111-modules-compose.md` のメモ
- `docs/adr/0014-composable-modules.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
