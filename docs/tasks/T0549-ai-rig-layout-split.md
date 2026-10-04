---
id: T0549
title: AI: rig.rs・module_rig.rs の大きな関数を分け、キーフォームの格子の補助を 1 つにする
status: done
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

- [x] 上の関数を分け、格子の補助を 1 つにする。提案の中身 (コマンドの列) は変えない (既存のテストがそのまま通る)
- [x] `just ci` が通る

## 参考

- `crates/ugoe-ai/src/rig.rs`・`crates/ugoe-ai/src/module_rig.rs`・`crates/ugoe-ai/src/turn.rs`
- `docs/tasks/T0528-review-ai-autorig.md` のメモ

## メモ

- やったこと:
  - `rig::Layout::new` を部位ごとに分けた (`Labeled` (ラベルの付いたメッシュ)・`body_meshes`・`eye_parts`・`mouth_parts`・
    `hair_groups`・`union_of`)。
  - `module_rig::ModuleLayout::new` を段階ごとに分けた (`costume_meshes`・`owned_base_meshes`・`followed_warp`・`group_of`・
    `add_sway`・`mesh_frames`・`exposed_colliders`)。`rig_group` から包むワープの作成 (`create_frame`) とメッシュの追従
    (`follow_meshes`) を出し、`#[allow(clippy::too_many_arguments)]` を外した。
  - 格子の補助を `src/grid.rs` (crate 内) に 1 つにした: キーの比べ方は誤差 1e-5 (`KEY_EPSILON`。`ugoe-rigging` の中立の
    キーの判定と同じ) にそろえ (`key_position`・`has_key`)、要素の軸のキー (`bindings`・`axis_keys`)、セル → 線形インデックス
    (`cell_index`)、線形インデックス → セル (`cell_at`)。`Rig::keyform_at`・`Rig::add_keys` (旧 `has_key`)・
    `Builder::add_key` (旧 `warp_keys`)・`turn::new_cells` がこれを使う。`keyform_at` が返すセルは格子のキーで書く
    (誤差の内の値で引いても `SetKeyformCell` の完全一致の照合に通る)。
  - `turn::new_cells` はキーのない軸のセルを飛ばす (添字の外を読まない)。先に失敗するテスト
    (`new_cells_skip_an_axis_without_keys`) を書いてから直した。
  - 提案が変わらないことは、既存のテストに加えて、作業中だけ `Builder::validate` でコマンドの列を書き出し、
    分割の前後で `cargo test -p ugoe-ai` の全 52 件の提案が一致することで確かめた (書き出しはコミットしていない)。
- やらなかったこと:
  - `ugoe-rigging` の `grid` (`KEY_EPSILON`・`Layout`) と `ugoe-editor-core` の `grid` (`cell_index`) は `pub(crate)` なので
    使えなかった (公開するのは範囲の外)。ugoe-editor-core の `cell_index_in` はキーを `==` で比べるので、こちらは格子のキーで
    セルを書いて合わせている。
  - `module_rig` の軸の組み立て (`candidates` の `to_bits` での重ね、`mesh_keyforms` の `dedup`) はキーの完全一致のまま
    (格子を引くのではなく、本体の格子のキーをそのまま写すため)。
- 起票の候補:
  - `ugoe-editor-core` の格子の補助 (セル ↔ 線形インデックス・キーの比べ方) を公開し、ugoe-ai・ugoe-rigging と共有する
    (3 か所に同じ並びの計算がある)。
  - `module_rig::candidates` (約 110 行) と `rig_sway` (8 引数) も長い。モジュールのリグの文脈 (`context`・`layout`・
    `params`・`options`) を 1 つの構造体にまとめると引数が減る。
