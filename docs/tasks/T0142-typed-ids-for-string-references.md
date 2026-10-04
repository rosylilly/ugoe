---
id: T0142
title: 文字列のままの ID の参照を Id<T> にそろえるか決める
status: in-progress
milestone: M9
crates: [ugoe-model, docs]
depends_on: [T0136]
assignee: claude-worker-T0142
---

# T0142: 文字列のままの ID の参照を Id<T> にそろえるか決める

## 目的

T0136 のレビューで、ID とその参照の型が要素によって違うことを見つけた。パラメータ・パーツ・デフォーマ・メッシュ・スロットは `Id<T>` だが、
次は `String` のまま。

- `HitArea::id`、`Glue::id`、`PhysicsRig::id`、`PhysicsCollider::id`
- 参照: `AvatarHitArea::hit_area`、`AvatarProfile::slots` (`SlotId` ではない)、`PhysicsRig::colliders`、`Slot::colliders`、`MirrorSource::id`

JSON の形は変わらない (どちらも文字列) が、Rust の公開 API の変更なので、そろえるかどうかを決め、そろえるならインターフェースを先に入れる小さな PR にする
(multi-agent.md)。`MirrorSource::id` はメッシュとデフォーマのどちらも指すので、`String` のままにする理由を doc コメントに書くのでもよい。

## 受け入れ条件

- [ ] そろえる / そろえないを決め、理由を `docs/spec/model.md#ID` (またはこのタスクのメモ) に書いた
- [ ] そろえるなら、型を変え、下流のクレートとエディタの生成した型 (`just gen`) を直した
- [ ] `just ci` が通る

## 参考

- docs/tasks/T0136-review-core-model.md (所見の表)
- docs/spec/model.md#ID

## メモ
