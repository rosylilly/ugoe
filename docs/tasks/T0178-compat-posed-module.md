---
id: T0178
title: "互換性テスト: ポーズを持つモジュール (jacket_module) の .ugoemod を assets/compat/dev に足す"
status: in-progress
milestone: M9
crates: [ugoe-format, assets/compat, docs]
depends_on: []
assignee: claude-worker-T0178
---

# T0178: 互換性テスト: ポーズを持つモジュール (jacket_module) の .ugoemod を assets/compat/dev に足す

## 目的

互換性テストの資料 (`assets/compat/dev/`) のモジュール (`long-hair.ugoemod`・`dress.ugoemod`) はポーズのグループを持たない。T0850 で
ポーズを持つモジュールのフィクスチャ `ugoe_model::fixtures::jacket_module` が入ったので、これを資料に足し、モジュールのポーズの読み書きを
互換性テストで確かめる (T0850 の候補)。

## 受け入れ条件

- [ ] `crates/ugoe-format/tests/compat.rs` の `dev_corpus` に `jacket_module` から作る `.ugoemod` (例 `jacket.ugoemod`) を足し、`assets/compat/dev/` に置いた (`UGOE_UPDATE_FIXTURES=1` で作る)
- [ ] 読んだモジュールのポーズのグループ (要素とパーツ) が書き出す前と同じであることをテストで確かめる。`dress-up-base.ugoe` に取り付けられることも確かめる
- [ ] ポーズの追従 (`ModulePose`) を持たせるかを決めた (`jacket_module` が持たなければ、持つモジュールを資料にだけ足すかをメモに書いた)
- [ ] docs/testing.md#互換性テスト の資料の一覧 (あれば) と assets/compat/README.md を直した
- [ ] `just crate ugoe-format` と `just ci` が通る

## 参考

- T0850 のメモ (やらなかったこと・起票の候補)
- crates/ugoe-format/tests/compat.rs、assets/compat/README.md、docs/testing.md#互換性テスト

## メモ

- T0043 で起票した (元の候補: T0850 (`assets/compat/dev/` のポーズを持つモジュール))。
