---
id: T0615
title: M2 の後片付け (デフォーマ変換の API 化と型の統一)
status: in-progress
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

- [ ] `ugoe-runtime` が、評価後のデフォーマのワールド変換を公開する (例: 任意の局所座標の点をキャンバス座標に写す、ヤコビ行列を求める、ワープの制御点・回転デフォーマの原点と軸を返す)。追加のみで、`update()` のメモリ確保なし・決定性を保つ
- [ ] `mesh_edit`・`space.rs`・`gizmo.rs` を、この API を使う形に置き換え、探針用のメッシュを使う実装を取り除く。既存のテストがすべて通る
- [ ] `tree/model.ts` の型を、生成した型 (`lib/generated/types.ts`) から組み立てる形にし、`TODO(T0603)` を消す
- [ ] 他に残っている M2 のタスクの `TODO(T06xx)` を確認し、片付けるか後続タスクにする
- [ ] `docs/spec/evaluation.md` に公開した API を書く
- [ ] `just ci`・`just gen` (差分なし)・`just wasm-check`・エディタのビルドが通る

## 参考

- `docs/tasks/T0606-mesh-editing.md`、`T0607-deformer-editing.md`、`T0608-keyform-editing.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
