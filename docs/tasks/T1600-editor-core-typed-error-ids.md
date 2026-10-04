---
id: T1600
title: editor-core のエラー型が持つ ID を型付きにするか決める
status: done
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: [T0678]
assignee: claude-worker-T1600
---

# T1600: editor-core のエラー型が持つ ID を型付きにするか決める

## 目的

T0142 で、モデルの中の文字列のままの ID の参照を `Id<T>` にそろえた。`ugoe-editor-core` のエラー型 (`EditError::NotFound` など)・
`ModuleError::ColliderNeedsBasePhysics`・`ColliderReference` は、まだ ID を `String` で持つ。型付きにすれば、取り違え (パーツの ID を
デフォーマの ID として報告する) を型で防げるが、エラーの引数 (ADR 0029 の `args`) は文字列で出すので、得るものは小さいかもしれない。

**判断が要る:** 型付きにするか。しないなら理由を `crates/ugoe-editor-core/README.md` に書いて閉じる。

## 受け入れ条件

- [x] 対象の型と、型付きにしたときの変更の大きさ (呼び出し側の数) をメモに書き、するか決めた
- [x] するなら置き換え、エラーの `code` と `args` (JSON) が変わらないことをテストで確かめた
- [x] `just ci` が通る

## 参考

- T0142 のメモ (起票の候補)
- crates/ugoe-editor-core/src/document/error.rs・module_project.rs

## メモ

- T0019 で起票した (元の候補: T0142 の editor-core のエラー型)。エラー型を触る T0678 の後に着手する。
- 対象と変更の大きさ (editor-core の本体 / editor-core のテスト / ほかのクレートとアプリ。パターン一致の数え上げ):
  `EditError::NotFound` 53 / 50 / 25、`DuplicateId` 16 / 19 / 8、`HasChildren` 6 / 7 / 0、`SelfReference` 6 / 2 / 0、
  `InvalidCell` 9 / 11 / 0、メッシュ・デフォーマ・スロット・グルーの固定の種類の欄 (`DeformerKindMismatch`・`MeshGeometry` など 10 種) で各 5〜12 / 1〜8、
  `ModuleError::ColliderNeedsBasePhysics` 6 / 3 / 2、`ModuleError::UnknownSlot` 5 / 6 / 0、`ColliderReference` 2 / 2 / アプリの IPC と 1 テスト。
- 決めたこと: **エラー型の ID は `String` のまま、データとして返す `ColliderReference` だけ `PhysicsRigId`・`ColliderId` にした。**
  - エラー型は `kind` (実行時の `&'static str`) と `id` の組が多く、`kind` にテクスチャ・モーション・表情・曲線・キーなど `Id<T>` の無い種類も入る。
    型付きにするには種類と ID を 1 つの列挙にまとめるしかなく、呼び出し側の 200 か所近くを変える。作る側はどれも型付きの ID から
    `to_string()` で作っていて取り違えの余地が小さく、ID は表示とコードの引数 (文字列) にしか使わない。`ugoe_model::validate::ValidationError` も同じ形。
  - 種類が欄で決まる一部のバリアント (メッシュの欄など) だけ型付きにする案は、同じ列挙の中で文字列と型付きが混ざるので採らなかった。
  - `ColliderNeedsBasePhysics` の `rig`・`collider` は、元の `ugoe_runtime::ComposeError::UnknownCollider` が文字列で、エラー型なので文字列のまま。
  - 理由は `docs/spec/model.md#ID` と `crates/ugoe-editor-core/README.md` に書いた。
- エラーの `code`・`args` は変えていない (既存の `message_tests` がそのまま通る)。`ColliderReference` の JSON が文字列の ID のままであることを
  `module_project_tests::colliders::collider_reference_serializes_ids_as_strings` で確かめた。TS の型 (`types.ts`) は doc コメントが増えただけ。
- `crates:` の外の変更: `apps/editor/src-tauri/src/sample.rs` のテスト 2 か所 (型付きになった `ColliderReference` との比較)、`just gen` による
  `apps/editor/src/lib/generated/types.ts` の doc コメント。
- やらなかったこと: `ModuleContext::unavailable_colliders` (`Vec<String>`) は IPC の要約 (`ModuleStatus::unavailable_colliders`) にそのまま渡すので文字列のまま。
  T0159 (`Id<T>` と文字列の比較) には頼っていない。
- 起票の候補: なし。
