---
id: T0615
title: M2 の後片付け (デフォーマ変換の API 化と型の統一)
status: done
milestone: M2
crates: [ugoe-runtime, ugoe-editor-core, ugoe-wasm, apps/editor]
depends_on: [T0606, T0607, T0608]
assignee: claude-worker-T0615
---

# T0615: M2 の後片付け (デフォーマ変換の API 化と型の統一)

## 目的

M2 の並行作業で重複した実装と、残った `TODO` を片付ける。

- デフォーマのワールド変換を取り出すために、3 か所 (T0606 の `mesh_edit`、T0608 の `space.rs`、T0607 の `ugoe-wasm` の `gizmo.rs`) が、それぞれ「探針用のメッシュを足したモデルを評価する」方法を独自に実装している。
- ツリー UI の型 (`apps/editor/src/lib/panels/tree/model.ts`) が手書きのまま (`TODO(T0603)`)。

## 受け入れ条件

- [x] `ugoe-runtime` が、評価後のデフォーマのワールド変換を公開する (例: 任意の局所座標の点をキャンバス座標に写す、ヤコビ行列を求める、ワープの制御点・回転デフォーマの原点と軸を返す)。追加のみで、`update()` のメモリ確保なし・決定性を保つ
- [x] `mesh_edit`・`space.rs`・`gizmo.rs` を、この API を使う形に置き換え、探針用のメッシュを使う実装を取り除く。既存のテストがすべて通る
- [x] `tree/model.ts` の型を、生成した型 (`lib/generated/types.ts`) から組み立てる形にし、`TODO(T0603)` を消す
- [x] 他に残っている M2 のタスクの `TODO(T06xx)` を確認し、片付けるか後続タスクにする
- [x] `docs/spec/evaluation.md` に公開した API を書く
- [x] `just ci`・`just gen` (差分なし)・`just wasm-check`・エディタのビルドが通る

## 参考

- `docs/tasks/T0606-mesh-editing.md`、`T0607-deformer-editing.md`、`T0608-keyform-editing.md` のメモ

## メモ

- **ランタイムの API** (`docs/spec/evaluation.md#デフォーマのワールド変換を読む`): `ModelInstance` に
  `deformer_world(d)` / `deformer_parent_world(d)` / `mesh_parent_world(m)` (どれも `WorldTransform` を返す。
  `point(p)`・解析的な `jacobian(p)`・`rotation_child(keyform)`・`is_identity()`)、`deformer_world_point` /
  `deformer_world_jacobian` (略記)、`warp_world_points(d)`、`rotation_world_frame(d)` (`Affine2`)、
  `mesh_local_positions(m)` (問い合わせ用に補間し直す。確保あり) を足した。`CompiledModel::deformer_index(id)` /
  `mesh_index(id)` で ID から添字を引く。まだモデルにないデフォーマの写像は `WorldTransform::IDENTITY` /
  `from_affine` / `from_warp_points` で作れる (T0607 の作成で必要)。どれも `update()` が残した状態を借りるだけで、
  `update()` は変えていない (確保なし・決定的のまま。`update_does_not_reallocate_buffers` も通る)。
- **テスト** (`crates/ugoe-runtime/src/instance.rs`): 全フィクスチャ × 5 通りのパラメータ値で、
  `mesh_parent_world(m).point(mesh_local_positions(m))` が `drawables` とビット単位で一致すること (グルーで
  動くメッシュを除く)、ヤコビ行列が中心差分と一致すること (ワープは格子の外を含む)、制御点・回転の枠が写像と
  一致すること、作った写像が評価したものと同じになること。`deform.rs` にワープのヤコビ行列の単体テスト。
- **置き換え**: 探針用のメッシュを使う実装は 3 か所ではなく **4 か所** あった (T0607 の
  `deformer_edit.rs` の `Rest::new` も同じ方法)。4 か所とも置き換えて探針のコードを消した。
  - `mesh_edit.rs`: 基準の姿は `mesh_local_positions` と `drawables`、ヤコビ行列は `mesh_parent_world().jacobian()`
    (中心差分から解析的な微分に変わった。セルの境目の頂点では片側の微分になるが、既存のテストは変更なしで通る)。
  - `space.rs`: 親の祖先だけのモデルを 1 回評価し、`deformer_world` で写す。ニュートン法は editor-core に残し、
    解析的なヤコビ行列を使う (反復ごとのコンパイルがなくなった)。
  - `deformer_edit.rs`: `Frame` は作ろうとしているデフォーマの分も持つため制御点を所有したままにし、写像・微分・
    回転デフォーマの子の変換は `WorldTransform` に任せた (ワープの式・ε の重複を消した)。逆写像のニュートン法だけが残る。
    未知の種類のデフォーマの下への付け替えのテストを足した。
  - `ugoe-wasm/src/gizmo.rs`: 2 つ目のインスタンスを持たず、表示中のインスタンスを評価して読む関数にした
    (`WebPuppet` の `gizmo` / `gizmo_stale` を削除)。テストの期待値は同じ。探針特有の「テクスチャなしでも
    作れる」テストは、存在しない ID のテストに置き換えた (見た目の変更のみ)。
- **ツリーの型**: `tree/model.ts` を生成した型の `Pick` で組み立てた (`TODO(T0603)` を削除)。テスト用の小さな
  モデルが書けるように、格子の `bindings` とメッシュの `mask_only` / `masks` は省略可にしている。
- **付け替えとミラー (T0607 × T0614、`inference_tests.rs` の末尾)**: ミラーのセルを持つ格子の
  `reparent_preserving_look` をテストした。
  - ミラー先 (`EyeR`、ミラー元は `EyeL`) を付け替えると、検証を通り、`mirror_source` と由来はそのまま、
    ミラーのセルは見た目を保って写し直される (ミラー元が変わっていないのでミラーし直さない。警告なし)。
  - ただし対の片側だけを付け替えると、ミラーは元の親座標系で反転して先へ書くので座標系が食い違い、
    その後にミラー元を編集するとミラーし直せない (頂点が対応しない)。ミラーのセルは前の値のまま残り、
    `RederiveWarning` を返す (格子は壊れない)。もう片側も付け替えて座標系をそろえると、ミラーし直しは
    元どおり働き、付け替えずに編集した結果をキャンバス座標へ写したものと一致する。
  - 自分自身がミラー元の格子 (`Nose`) は、付け替えで格子が変わるので新しい座標系でミラーし直され
    (警告なし)、写し直した値と一致する (対称軸の既定値はワープの 0.5 → キャンバスの原点 128 で、同じ直線)。
- **計測** (release、一時的なテストで計測してコミットしていない。フィクスチャのメッシュを 300 倍に複製したモデル):
  `MoveMeshVertices` 200 回 198ms → 133ms (探針のメッシュを足したモデルの 2 回のコンパイルが 1 回になった分。
  残りはモデルの複製とコンパイル)、`local_to_canvas` + `canvas_to_local` (50 点) 100 回 25.4ms → 21.6ms
  (祖先だけのモデルなので元々軽い)。
- **`TODO(T06xx)`**: コード中に残っていたのは `TODO(T0603)` (ツリーの型) だけで、片付けた。
- **後続の候補**:
  - インスペクタのキーフォーム (`inspector/keyforms/keyforms.ts`) にも手書きの `KeyformBinding` / 格子の型がある。
    T0613 が編集中のため触っていない。生成した型に寄せる小さなタスクにできる。
  - `ugoe-rigging/src/space.rs` の座標系の単位 (ワープは制御点の外接矩形で近似) を、ランタイムの
    `WorldTransform::jacobian` で求める形にできる (T0612 の「親がワープの要素の深度からの回転」とあわせて)。
  - 対の片側だけの付け替えでミラーが切れることを、付け替えの時点で知らせる (警告を `ApplyReport` に
    足す)、またはミラーを「元と先の親座標系が違えばキャンバス座標を経由して反転する」ように広げる。
  - エディタの操作ごとにモデル全体を複製・コンパイルしている (`mesh_edit` / `deformer_edit` の基準の姿)。
    `Document` が既定値のインスタンスを持ち回れば、さらに速くなる。
