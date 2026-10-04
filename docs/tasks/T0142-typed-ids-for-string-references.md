---
id: T0142
title: 文字列のままの ID の参照を Id<T> にそろえるか決める
status: done
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

- [x] そろえる / そろえないを決め、理由を `docs/spec/model.md#ID` (またはこのタスクのメモ) に書いた
- [x] そろえるなら、型を変え、下流のクレートとエディタの生成した型 (`just gen`) を直した
- [x] `just ci` が通る

## 参考

- docs/tasks/T0136-review-core-model.md (所見の表)
- docs/spec/model.md#ID

## メモ

### 結果 (claude-worker-T0142)

**そろえた。** 理由と規則は `docs/spec/model.md#ID` に書いた。版は 0.0.0 で JSON と TypeScript の形は変わらず (どちらも文字列)、
種類の違う ID の取り違えをコンパイル時に防げるため。

- `ugoe-model`: 別名 `HitAreaId`・`GlueId`・`ParameterGroupId` (ルートから再公開)、`physics::PhysicsRigId`・`physics::ColliderId` を加え、
  `HitArea::id`・`Glue::id`・`ParameterGroup::id`・`PhysicsRig::id`・`PhysicsCollider::id` と、参照 `AvatarHitArea::hit_area`・
  `AvatarProfile::slots` (`SlotId`)・`PhysicsRig::colliders`・`Slot::colliders`・`Parameter::group` を型付きにした。
  `ParameterGroup` と `Parameter::group` はタスクの列挙に無かったが、同じ不揃いなので一緒にそろえた。
- `MirrorSource::id` は `String` のまま。メッシュとデフォーマのどちらも指すので、理由を doc コメントに書いた。
  同じ理由で `Command::SetMirrorPair::mirror` も `String` のまま。
- 下流: `ugoe-runtime` (合成)・`ugoe-physics` (`RestPoseCollision` の `rig`・`collider` も型付きに)・`ugoe-rigging`
  (`KeyformTarget::Glue(GlueId)`)・`ugoe`・`ugoe-wasm`・`ugoe-editor-core` (`Command` の物理のリグ・衝突体・グルー・
  パラメータグループ・アバターのスロットの ID、`KeyformTarget::Glue`、`physics_check::RestPoseCollision`、`draw_clone::glue_id` の戻り値)・
  `ugoe-ai`・`ugoe-cli`・`ugoe-mcp`・エディタの src-tauri。エラー型・差分の表示・IPC の要約 (`ModuleSummary` の `base_rigs` など)
  は文字列のまま (表示用)。
- `just gen` の差分: `types.ts` は doc コメントだけ、`command.schema.json` は `Id` の説明 (`description`) が付いただけ。
  公開仕様の JSON Schema (`spec/schema/`) は変わらない。英語の公開仕様 (`spec/model.md#identifiers`) は Rust の型に触れないので直していない。
- `crates:` は `ugoe-model` と `docs` だが、受け入れ条件どおり下流のクレートとアプリも直した (型の変更に合わせる機械的な直しだけ)。
- 振る舞いは変えていないので、新しいテストは足していない (既存のテストを型に合わせて直した)。
- 確かめたこと: `just ci` が通る。`just wasm-check` が通る (wasm32 だけでビルドする `ugoe-wasm/src/web.rs` に直しが要った)。
  `bindings/godot` は `cargo check --offline` が通る。`bindings/unity` は C ABI だけを使うので影響なし (確かめていない)。

起票の候補:

- `ugoe-core` の `Id<T>` に `PartialEq<str>` / `PartialEq<&str>` を足すと、テストの `id.as_str() == "..."` が `id == "..."` で書ける
  (`ugoe-core` の公開 API の追加なので、このタスクではしなかった)。
- `ugoe-editor-core` のエラー型 (`EditError::NotFound` など)・`ModuleError::ColliderNeedsBasePhysics`・`ColliderReference` は ID を `String` で持つ。
  型付きにするかは別に決める。
