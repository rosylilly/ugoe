---
id: T0014
title: "editor-core: 要素の削除と ID の書き換えで、参照の後始末を 1 か所にまとめる"
status: done
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T0014
---

# T0014: editor-core: 要素の削除と ID の書き換えで、参照の後始末を 1 か所にまとめる

## 目的

M9 のコードレビューの横断の所見 (T0668・T0137。まとめは docs/workflow/code-review.md#m9-の結果)。

- 要素を消すコマンドは、消した要素を指す参照 (ミラー元・対・物理・スロットなど) をそれぞれの場所で外している。`RemoveDeformer` と `UnwrapDeformer`、
  `RemoveMesh` と `RemoveGlue` のように同じ削除を別の経路で行うコマンドで外し忘れが起きた (T0667・T0668 で直した)。
- キーフォーム格子をたどるコードで、ブレンドシェイプの格子 (`blend_shapes`) の漏れがあった (T0137 の合成の付け替え)。格子の中の ID を書き換えるコード
  (改名・ミラー・複製) も、ブレンドシェイプ格子までたどっているかを確かめていない。

削除の後始末を `apply_with_report` の後段 (`physics_edit::forget_removed` と同じく、前後のモデルを比べて消えた ID から外す) にまとめると漏れにくい。

## 受け入れ条件

- [x] 削除の後の参照の後始末を 1 か所にまとめ、各コマンドの個別の後始末を外した (既存のテストが通る)
- [x] 改名・ミラー・複製など格子の中の ID を書き換えるコマンドが、ブレンドシェイプの格子も書き換えることをテストで確かめた
- [x] 「不正だが読み込めるデータ」(空の列・未知の値・長さの合わない格子) で編集の関数がパニックしないことを確かめるテストを足した (T0668 の横断)
- [x] docs/editor.md#削除の連鎖と拒否 を直した
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0137・T0668 のメモの「横断」

## メモ

### やったこと

- `src/cleanup.rs` の `forget_removed` を `Document::apply_with_report` の `apply_to` の直後 (`derived` の作り直しと検証の前) で 1 回呼ぶ。
  適用の前後のモデルと物理を比べ、前にあって後にない ID (パラメータ・パーツ・デフォーマ・メッシュ・グルー・当たり判定・スロット・物理の衝突体) への参照を外す。
  - 各コマンドの個別の後始末 (`forget_missing_mirror_sources` の 5 か所の呼び出し、`remove_parameter`・`remove_part`・`remove_deformer`・`remove_mesh` の参照の除去、
    `avatar::forget_removed_hit_areas`、`slot_edit::remove` の `avatar.slots`、`slot_edit::forget_removed_colliders`、`RemovePhysicsCollider` のリグの `colliders`) を外した。
    物理の参照を外す `physics_edit::forget_removed` (基準の姿での衝突体の写し直しを含む) は残し、`cleanup` から呼ぶ。
  - コマンド自身に残したもの: `remove_parameter` の格子の軸の畳み込み (値を選ぶ編集で、`batch` の後続のコマンドが畳んだ格子を前提にしうる)、
    `remove_mesh` のグルーと当たり判定の連鎖の削除 (要素そのものの削除)。消えたグルー・当たり判定への参照は `cleanup` が外す。
- **見つけた漏れ (直した):** ブレンドシェイプ格子のミラー元 (`blend_shapes[i].mirror_source`) を、元の要素を消しても外していなかった
  (`forget_missing_mirror_sources` が通常の格子しか見ない)。検証 (`UnknownReference`) で削除・`unwrap_deformer` が拒否されていた。
  先に失敗するテスト `cleanup_tests::removing_an_element_forgets_it_as_a_blend_shape_mirror_source`・`unwrapping_a_deformer_forgets_it_as_a_blend_shape_mirror_source`。
- **振る舞いの変化:** 前後の比較にしたので、(1) 読み込んだ時点で既に存在しない要素を指していたミラー元・当たり判定の対応は外さなくなった
  (前は削除のたびに外していた。問題を増やさない編集はできる、の考え方に揃えた。`references_broken_before_the_edit_are_kept`)、
  (2) `batch` の中で消して同じ ID で足し直した要素への参照は残る (`an_element_removed_and_added_back_in_a_batch_keeps_its_references`)、
  (3) 消して空になったポーズのグループだけを消す (前は空のグループをすべて消していた)。
- ブレンドシェイプ格子のテスト: モジュールの合成と分割でのミラー元の付け替え (`module_project_tests::blend_shape_mirror_sources_are_renamed_both_ways`)、
  描画用の複製の合わせ直し (`draw_clone_tests::sync_rewrites_blend_shape_cells_of_the_clone`)、ミラーがブレンドシェイプ格子を変えないこと
  (`inference_tests::mirror_keeps_blend_shape_grids`。仕様どおり)、デフォーマの作成・付け替え・外す・回転の中心の移動での写し直し
  (`deformer_edit_tests::deformer_edits_keep_the_look_of_blend_shape_cells`)。どれも既に正しく、漏れはなかった。
  エディタの中で ID を書き換えるのはモジュールの分割 (`Splitter::grid`) とスロットの改名 (格子は持たない) だけで、ID を変える改名のコマンドはない。
- `malformed_tests.rs`: 6 つのフィクスチャを 6 通りに壊し (セルがない・セルの数が合わない・キーのない軸・由来の数とミラー元の不正・未知の値・
  軸やセルのないブレンドシェイプ格子。メッシュの頂点・三角形・深度、ワープの制御点、グルーのペアの不正も加える)、要素ごとの格子・形・階層の
  コマンド (約 3500 回の適用) がパニックしないこと、拒否したらモデルが変わらないことを確かめた。パニックは見つからなかった。
- `docs/editor.md#削除の連鎖と拒否` を「コマンド自身がすること」と「参照の後始末」の 2 つの表に書き直した。`remove_physics_collider` の行にスロットを足した。

### やらなかったこと

- アバタープロファイルのトラッキングの上書き・視線のパラメータは、パラメータを消しても書き換えない (前と同じ。準拠の検査が知らせる)。文書に書いた。
- モーション・表情・物理のテンプレートなど、モデルの外のデータへの不正なデータのテストは足していない (モーションは T0668 で確かめた)。

### 起票の候補

- 前後の比較は `Document::apply_with_report` を通る編集だけが対象。`set_module_settings`・`rebase_module` のように別の経路でモデルを組み直す
  処理では要素が消えうるので、同じ後始末を通すか確かめる (今はモジュールの本体を読み直して合成し直すので参照は合成側で揃う)。

### just ci

- `git rebase main` の後に `just ci` が通った。

