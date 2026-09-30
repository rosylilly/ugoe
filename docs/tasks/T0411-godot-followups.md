---
id: T0411
title: Godot: CI での検査・ライティングと着せ替えとトラッキング・リソースとしての取り込み
status: todo
milestone: M7
crates: [bindings/godot, docs]
depends_on: [T0406]
assignee:
---

# T0411: Godot の後続

## 目的

T0406 の後続の候補をまとめる。

## 受け入れ条件

- [ ] CI で `node bindings/godot/build.mjs --check` と `--conformance` を回す (Godot と Xvfb を入れる。版を固定する)。範囲外の ci.yml の変更は team-lead に相談する
- [ ] ライティング (法線マップ・光源)・着せ替え (.ugoemod の付け外し)・トラッキングの入力をノードに出す
- [ ] `.ugoe` を Godot のリソースとして取り込む (ResourceFormatLoader)
- [ ] 1 要素あたり 8 つまでのマスクの上限をなくす。アニメーション中に入れ子のマスクが 1 フレーム遅れないかを確かめる
- [ ] Godot のアセットライブラリでの配布の準備 (手順だけ。公開は利用者が行う)
- [ ] Vulkan のレンダラでの見た目、Windows・macOS でのビルドは、できなければ利用者の確認に残す
- [ ] justfile に godot-build / godot-check を足す (Godot が無ければ入れ方を案内して止める。just ci には入れない)。bindings/godot/check-licenses.mjs を scripts/check-licenses.mjs の小さな関数に統合するかを決め、ADR 0026 の検査の方法を揃える
- [ ] `just ci` と `node bindings/godot/build.mjs --check` が通る

## 参考

- `docs/tasks/T0406-godot-extension.md` のメモ、`docs/adr/0026-godot-binding.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
