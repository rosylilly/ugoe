---
id: T0419
title: Unity: T0414 の API を C# に出す・描画のバッチング・UPM のリリースの自動化・Linear の色空間
status: in-progress
milestone: M7
crates: [bindings/unity, .github/workflows, docs]
depends_on: [T0405, T0414]
assignee: claude-worker-T0419
---

# T0419: Unity の後続

## 目的

T0405 の後続の候補をまとめる。

## 受け入れ条件

- [ ] T0412・T0414 で足した C ABI (show_pose・set_effects_json・part_*・motion_event・physics_json) を C# に出し、dotnet のテストを足す
- [ ] 描画のバッチング (要素ごとの MeshRenderer と SortingGroup をまとめる) を検討し、効くなら入れる
- [ ] UPM パッケージのリリースの自動化 (タグで zip を作る。公開は利用者が行う)
- [ ] Linear の色空間でも半透明の色が参照と合うようにするか、Gamma を要求して README に書く
- [ ] 実際の Unity での確認 (シェーダのコンパイル・シーンと .meta・マスクの上下の向き・Built-in と URP・Android と iOS) は、できなければ利用者の確認に残す
- [ ] `just ci`・`just unity-check` が通る

## 参考

- `docs/tasks/T0405-unity-package.md` のメモ、`bindings/unity/README.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
