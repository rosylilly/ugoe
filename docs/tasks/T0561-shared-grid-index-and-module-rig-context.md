---
id: T0561
title: 格子のセルと線形の添字の計算を 1 か所にまとめ、module_rig の引数を文脈の構造体にまとめる
status: done
milestone: M9
crates: [ugoe-rigging, ugoe-editor-core, ugoe-ai]
depends_on: []
assignee: claude-worker-T0561
---

# T0561: 格子のセルと線形の添字の計算を 1 か所にまとめ、module_rig の引数を文脈の構造体にまとめる

## 目的

T0549 の候補。

- 格子の補助 (セル ↔ 線形の添字・キーの比べ方) が `ugoe-editor-core`・`ugoe-ai`・`ugoe-rigging` の 3 か所に同じ並びの計算として書かれている。
  `ugoe-rigging` は editor-core に依存できない (editor-core と ugoe-ai が rigging に依存する) ので、置き場所は `ugoe-rigging` か、より下の層にする。
- `crates/ugoe-ai/src/module_rig.rs` の `candidates` (約 110 行) と `rig_sway` (8 引数) が長い。モジュールのリグの文脈 (`context`・`layout`・`params`・`options`) を
  1 つの構造体にまとめて引数を減らす。

## 受け入れ条件

- [x] 格子の補助が 1 か所にあり、3 つのクレートの写しがそれを使う。補助にテストがある
- [x] `candidates`・`rig_sway` が文脈の構造体を受け、clippy の `too_many_arguments` の許可を外せた
- [x] 振る舞いは変えない (既存のテストがそのまま通る)
- [x] `just layers` と `just ci` が通る

## 参考

- T0549 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0549 の 2 つ)。
- やったこと:
  - 格子の補助を `ugoe_rigging::grid_index` (公開のモジュール) に 1 つにした: 誤差 `KEY_EPSILON` (1e-5)・`key_position`・
    `axis_lens`・`cell_count`・`linear_index`・`cell_coords`。積と添字は飽和させ、キーのない軸はキー数 1 として数える
    (editor-core の `linear_index` の `max(1)` と同じ)。テストは `grid_index.rs` にある。
  - `ugoe-rigging`: 中立のキーの判定 (`Layout::new`)・`Layout::coords`・ミラーのキーの照合がこれを使う。`Layout` の
    `stride` (加算合成とミラーの添字の計算) は格子の検査を通った後の値なので残した。
  - `ugoe-editor-core`: `linear_index` と `NEUTRAL_EPSILON` を消し、`slice_mask`・`combination_cells`・`add_key` の
    `stride`・中立のキーの判定がこれを使う。`cell_index_in`・`nearest_key` のキーの完全一致 (`==`) は変えていない。
  - `ugoe-ai`: `grid.rs` の `KEY_EPSILON`・`key_position`・`cell_count` の写しを消し、`cell_index`・`cell_at` を共有の
    計算で組み直した (セルの指定 `KeyformCell` を作る部分だけが残る)。
  - `module_rig`: 配置を調べる文脈 `Survey` (モデル・スロットの文脈・取り付け先・使えるパラメータ) と、リグを組む文脈
    `RigContext` (`module`・`layout`・`params`・`options`) を作った。`candidates` は `Survey` を受け、鎖
    (`anchor_chain`)・軸 (`chain_axes`)・覆う矩形 (`warp_region`) に分けた。`rig_group`・`create_frame`・`rig_sway` は
    `RigContext` を受ける。`rig_sway` と `mesh::find` の `#[allow(clippy::too_many_arguments)]` を外した。
  - `candidates` の検査の順を少し入れ替えた (自分の軸の検査を鎖の前に) が、どちらも副作用のない絞り込みなので結果は
    同じ。既存のテスト (ugoe-ai・ugoe-editor-core・ugoe-rigging) はそのまま通る。`just ci` は通った。
- やらなかったこと:
  - `candidates` は `ModuleLayout` を作る途中で呼ぶので、`RigContext` (layout を含む) ではなく `Survey` を受ける。
  - `crate::rig` の `Rig::pendulum` の `too_many_arguments` の許可は範囲 (module_rig) の外なので残した。
  - ランタイム層は触っていないので `just wasm-check` は流していない。
- 起票の候補:
  - `Rig::pendulum` (11 引数) の引数を振り子の設定の構造体にまとめる。
  - `ugoe-rigging` の `Layout` の `stride` による添字の計算 (`additive.rs`・`mirror.rs`) も `grid_index` にそろえるか
    検討する (性能の確認が要る)。
