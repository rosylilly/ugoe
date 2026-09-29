---
id: T0315
title: 物理: 静止の姿で衝突体に入っている髪を知らせる
status: todo
milestone: M4
crates: [ugoe-cli, ugoe-editor-core, apps/editor, docs]
depends_on: [T0311]
assignee:
---

# T0315: 物理: 静止の姿で衝突体に入っている髪を知らせる

## 目的

T0311 で足した `PhysicsSystem::rest_pose_collisions` を、利用者が気づける場所につなぐ。

## 受け入れ条件

- [ ] `ugoe validate` が、静止の姿で衝突体に入っている鎖を警告として出す (エラーにはしない)
- [ ] エディタの Verify タブ (または物理の画面) に同じ警告を出し、該当の鎖と衝突体を選べる
- [ ] PSD の取り込みや物理のテンプレートの適用の直後にも知らせるかを決めて書く
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0311-physics-collision-robustness.md` のメモ
- `docs/spec/physics.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
