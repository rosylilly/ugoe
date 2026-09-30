---
id: T0415
title: Godot: 配布の自動化・Light2D と XRFaceTracker からの入力・マスクの解像度
status: todo
milestone: M7
crates: [bindings/godot, docs]
depends_on: [T0411]
assignee:
---

# T0415: Godot の配布と入力の後続

## 目的

T0411 の後続の候補をまとめる。

## 受け入れ条件

- [ ] 配布の自動化: 各 OS (Linux・Windows・macOS) のビルドと、アセットライブラリ向けの zip を CI で作る (公開は利用者が行う)。範囲外の ci.yml の変更は team-lead に相談する
- [ ] Godot の `Light2D` から光源を取る (ノードのオプション)
- [ ] Godot の `XRFaceTracker` からトラッキングの入力への対応表
- [ ] アトラスに収まらずにマスクの解像度を下げたとき、元に戻す判断 (戻す条件と、画質の目安)
- [ ] 画面のある Godot エディタでの確認 (インスペクタで UgoeModel を選べるか、modules と lights の配列の編集、書き出したゲームに .ugoe が含まれるか) は、できなければ利用者の確認に残す
- [ ] `just ci`・`just godot-check` が通る

## 参考

- `docs/tasks/T0411-godot-followups.md` のメモ、`docs/adr/0026-godot-binding.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
