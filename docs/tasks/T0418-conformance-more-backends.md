---
id: T0418
title: 適合テスト: Godot と Unity を乗せる・wasm の描画結果の読み戻しで WebGPU も比べる
status: in-progress
milestone: M7
crates: [tests/conformance, bindings/godot, bindings/unity, ugoe-wasm, bindings/web, docs]
depends_on: [T0408, T0411, T0405]
assignee: claude-worker-T0418
---

# T0418: 適合テストの対象を広げる

## 目的

T0408 の後続の候補をまとめる。

## 受け入れ条件

- [ ] Godot の `build.mjs --conformance` を、tests/conformance の計画 (plan.json) と compare に乗せる (段の止め方の違いと、Godot にだけあるケースの移し方は tests/conformance/README.md に書いてある)
- [ ] Unity の Unity に依存しない C# の層 (dotnet) を、頂点の比較に乗せる
- [ ] wasm に描画結果を読み戻す API と、不透明度などを取り出す API を足し、ヘッドレスの Chrome の WebGPU でも画像を比べられるようにする (いまは WebGL2 だけ)。bindings/web の型のスタブも揃える
- [ ] `just ci`・`just conformance` が通る

## 参考

- `docs/tasks/T0408-conformance-tests.md`・`docs/tasks/T0411-godot-followups.md`・`docs/tasks/T0405-unity-package.md` のメモ、`tests/conformance/README.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
