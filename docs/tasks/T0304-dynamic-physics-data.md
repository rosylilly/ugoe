---
id: T0304
title: 動的物理のデータ定義
status: done
milestone: M3
crates: [ugoe-model]
depends_on: []
assignee: claude-worker-T0304
---

# T0304: 動的物理のデータ定義

## 目的

物理の出力先 (`deformer_angle` / `deformer_offset` / `vertices`)、衝突体 (円・カプセル)、重力の基準のデフォーマ、粒子の `collision_radius` を定義する (インターフェースを先に入れるタスク)。

## 受け入れ条件

- [x] 型を定義し、serde で往復できる
- [x] `validate` が出力先・衝突体の取り付け先の参照を検査する
- [x] 古いデータ (パラメータへの出力だけ) が読める
- [x] `docs/spec/physics.md` と `docs/spec/model.md` を更新する
- [x] `just ci` が通る

## 参考

- `docs/adr/0011-dynamic-physics.md`
- `docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`

## メモ

- やったこと
  - `PhysicsOutput::parameter` を `target: PhysicsTarget` (`parameter` / `deformer_angle` / `deformer_offset` / `vertices` / `Unknown`) に置き換えた。旧形式の `parameter` フィールドは手書きの `Deserialize` で `parameter` 出力先として読む。T0301 向けに `PhysicsOutput::parameter()` (パラメータ出力先なら ID) を用意した。
  - `vertices` の対象は `VertexOwner` (`{"mesh": ID}` / `{"warp": ID}`)、重みは頂点 (制御点) ごとの密な配列。
  - `PhysicsSettings::colliders` (`PhysicsCollider { id, name, attach, shape }`、形状は円とカプセル、未知の形状は `Unknown`)、`PhysicsRig::colliders` / `frame`、`PhysicsParticle::collision_radius` を追加した (いずれも `serde(default)`)。
  - 検証は `validate::validate_physics(&physics, &model)` (`validate/physics.rs`)。`ValidationError` に `DeformerKindMismatch` と `WeightCount` を追加した。`unknown::in_physics` が未知の出力先と形状を報告する。
  - `physics.dynamic` は 1.0 の基準の機能なので、`FEAT` の宣言と `SCHEMA_VERSION` は変えていない。
- 範囲外の変更: 構造体リテラルの追従のため、`ugoe-format` (bundle.rs のテスト) と `ugoe-editor-core` (project_tests.rs) にフィールドを足した。先に入った T0301 の `ugoe-physics` は、出力先を `PhysicsOutput::parameter()` で読むように直した (パラメータ以外の出力先は読み飛ばす)。生成物 `apps/editor/src/lib/generated/types.ts` を更新した。
- 残した課題 (T0305 の候補)
  - `ugoe-format` の読み込みで `validate_physics` を呼ぶ (今は `validate` だけ)。
  - 衝突にはチェーンの根元のワールド位置が要る。根元を取り付けるデフォーマ (または `frame` を兼ねるか) は T0305 で決める。
  - 衝突体の位置はワープの局所座標系 (UV) でも表せるが、半径は変形を受けない。ワープへの取り付けの扱いは T0305 で確かめる。
