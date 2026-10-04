---
id: T0549
title: AI: rig.rs・module_rig.rs の大きな関数を分け、キーフォームの格子の補助を 1 つにする
status: in-progress
milestone: M9
crates: [ugoe-ai]
depends_on: [T0528]
assignee: claude-worker-T0549
---

# T0549: AI: rig.rs・module_rig.rs の大きな関数を分け、キーフォームの格子の補助を 1 つにする

## 目的

T0528 のレビューで見つけた読みにくさと重複 (振る舞いは変えない)。

- `rig::Layout::new` (約 150 行: 顔・体・目・口・髪) と `module_rig::ModuleLayout::new` (約 175 行: 追従の候補・本体のメッシュの振り分け・
  衣装のまとまり・揺れ・衝突体・差分)、`module_rig::rig_group` が長い。部位ごと・段階ごとの関数に分ける
- 格子の補助が 3 か所にある: `rig::Rig::keyform_at`・`Rig::has_key` (キーを `==` で比べる)、`turn::warp_keys`・`Builder::add_key`
  (1e-5 の許容誤差で比べる)、`turn::new_cells`。キーの比べ方をそろえ、格子の添字の計算を 1 つにする
  (`ugoe-rigging` の `grid` や `ugoe-editor-core` に同じものがあれば使う)
- `turn::new_cells` は軸のキーが空のとき添字の外を読む (`binding.keys[rest % n]`。検証済みのモデルでは起きない) ので、あわせて直す

## 受け入れ条件

- [ ] 上の関数を分け、格子の補助を 1 つにする。提案の中身 (コマンドの列) は変えない (既存のテストがそのまま通る)
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-ai/src/rig.rs`・`crates/ugoe-ai/src/module_rig.rs`・`crates/ugoe-ai/src/turn.rs`
- `docs/tasks/T0528-review-ai-autorig.md` のメモ

## メモ
