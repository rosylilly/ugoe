---
id: T0416
title: SDK: 評価後のパーツの不透明度を公開する・着せ替え中に本体の物理を差し替える
status: in-progress
milestone: M7
crates: [ugoe-runtime, ugoe, ugoe-ffi, ugoe-wasm, docs]
depends_on: [T0414]
assignee: claude-worker-T0416
---

# T0416: SDK の残り (T0414 の後続)

## 目的

T0414 の後続の候補をまとめる。

## 受け入れ条件

- [ ] 評価後のパーツの不透明度 (祖先を掛けた値) を ugoe-runtime から公開し、C ABI と wasm に出す (今の C ABI の part_opacity は、直前の update が書いた上書きの値)
- [ ] 着せ替え中に本体の物理の設定を差し替える SDK の API (今は付け外しのたびに本体の設定から作り直すので失われる)。仕様 (docs/spec/modules.md と spec/) との関係を決める
- 黒箱の比較で view を渡すのは、spec/ だけを読む作業者の担当なので T0422 に分けた
- [ ] `just ci`・`just wasm-check`・`just ffi-check` が通る

## 参考

- `docs/tasks/T0414-host-actions-in-bindings.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
