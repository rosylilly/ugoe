---
id: T0850
title: "ポーズを持つモジュールのフィクスチャを足し、着せ替えの直後にフェードを飛ばすことをスタジオで確かめる"
status: todo
milestone: M9
crates: [ugoe-model, apps/studio, docs]
depends_on: []
assignee:
---

# T0850: ポーズを持つモジュールのフィクスチャを足し、着せ替えの直後にフェードを飛ばすことをスタジオで確かめる

## 目的

T0844 で、スタジオは読み直したシーンで選んだポーズの要素をフェードせずに出し、`follow_match.rs` にポーズの切り替えを入れた。
`ugoe_model::fixtures` のモジュール (`long_hair_module`・`dress_module`) はポーズを持たない (`ModulePose::default()`) ので、
モジュールを着せた直後 (モジュールのポーズのグループ) にフェードを飛ばすことは確かめていない (T0844 の候補)。

## 受け入れ条件

- [ ] `ugoe_model::fixtures::modules` に、ポーズのグループ (2 つ以上の要素) を持つモジュールのフィクスチャがある (既存のフィクスチャの値は変えない)。
      doc コメントで何を確かめるためのものかを書いた
- [ ] `conductor.rs` のテストで、そのモジュールを着せたシーンを読み直した直後に、選んだ要素が不透明度 1・ほかが 0 (フェードの途中でない) ことを確かめる
- [ ] `follow_match.rs` で、着せ替えの直後のポーズの切り替えでも指揮役と受け手が一致することを確かめる
- [ ] フィクスチャの一覧 (docs/testing.md など、載せている所があれば) を直した
- [ ] `just ci` が通る

## 参考

- T0844 のメモ (やったこと・起票の候補)
- crates/ugoe-model/src/fixtures/modules.rs、apps/studio/src-tauri/src (`conductor.rs`・`follow_match.rs`)

## メモ

- T0041 で起票した (元の候補: T0844 (ポーズを持つモジュールのフィクスチャ))。`ugoe-model` は fixtures (テスト用のデータ) だけを足す。
