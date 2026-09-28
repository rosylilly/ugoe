---
id: T0307
title: ブレンドシェイプ
status: done
milestone: M3
crates: [ugoe-model, ugoe-runtime, ugoe-editor-core]
depends_on: []
assignee: claude-worker-T0307
---

# T0307: ブレンドシェイプ

## 目的

`kind = blend_shape` のパラメータを軸に持つキーフォーム格子を、通常の補間結果に差分として加算する (`docs/spec/evaluation.md#6-ブレンドシェイプ`)。
ARKit のブレンドシェイプのように、互いに独立した表情の差分を重ねられるようにする。

## 受け入れ条件

- [x] 差分の持ち方を決め、`docs/spec/model.md` の `> 未決:` を解消する (既存の `MeshKeyform` の流用か、専用の型か。ファイル形式を変えるなら `docs/spec/file-formats.md` と `SCHEMA_VERSION` も)
- [x] `validate` がブレンドシェイプ格子の不正 (通常のパラメータとの混在の規則など、決めたもの) を検査する
- [x] ランタイムが仕様の式で加算する。メッシュとデフォーマ (ワープの格子点・回転の角度など、決めた範囲) のテスト、既定値では変化がないテスト、複数の格子の加算の順のテスト
- [x] エディタのコマンドが、ブレンドシェイプのパラメータを作ってキーを置けるか確かめる (必要なら直す。UI は範囲外)
- [x] `just ci`・`just wasm-check` が通る

## 参考

- `docs/spec/evaluation.md#6-ブレンドシェイプ`
- `docs/spec/model.md#ブレンドシェイプ-m3`

## メモ

- 差分の持ち方: `KeyformGrid<T>` に `blend_shapes: Vec<KeyformGrid<T>>` を足した (同じキーフォームの型の格子の列)。
  セルは差分ではなく形そのものを持ち、ランタイムが既定値での形との差を取る。省略でき、空なら書き出さない (`SCHEMA_VERSION` は 0 のまま)。
  専用の型にしなかった理由は `docs/spec/model.md#ブレンドシェイプ` に書いた。
- 検証: 通常の格子に `blend_shape` の軸、ブレンドシェイプ格子に `normal` の軸 (`ParameterKindMismatch`)、軸なし・入れ子・パーツとグルーの `blend_shapes` (`InvalidBlendShape`)、
  同じパラメータが 2 つのブレンドシェイプ格子の軸 (`DuplicateBinding`)。ブレンドシェイプ格子にも格子・頂点数・制御点数の検査を行う。未知の種類のパラメータはどちらにも置ける。
- ランタイム: メッシュ (全数値フィールド)・ワープ (制御点・不透明度)・回転 (原点・角度・拡大率・不透明度) に宣言順に 1 つずつ加算。
  既定値での重みはコンパイル時に求め、現在の重みが同じ格子は加算しない (既定値ではビット単位で変化なし)。`mesh_local_positions` も差分を含む。
  変更フラグは出力の比較なので追加の対応は不要 (テストあり)。フィクスチャ `fixtures::blend_shapes` を追加。
- エディタ: `add_keyform_key` / `remove_keyform_key` / `set_keyform_cell` / `set_mesh_keyform` がパラメータの種類でブレンドシェイプ格子に振り分ける。
  新しい格子は基準の形 (通常の格子の既定値での補間) で埋める。頂点数・座標系を変える編集、`set_mesh_draw_order`、`remove_parameter` もブレンドシェイプ格子を扱う。
  推定 (加算合成・ミラー・深度からの回転) はブレンドシェイプ格子を扱わない (`docs/spec/keyform-inference.md`)。
- 範囲外の小さな変更: `ugoe-format` の機能の一覧に基準の機能 `deform.blend_shape` を追加 (仕様書の表と一致させるテストがあるため)。
  `just gen` で生成物 (`apps/editor/src/lib/generated/`・`command.schema.json`) を更新し、型検査のサンプルに `blend_shapes` のフィクスチャを加えた。
- T0305 (2 段階の評価) との衝突: `instance.rs` の `update_deformers` / `update_meshes` に差分の加算を数行ずつ入れた。差分の計算は `ugoe-runtime/src/blend_shape.rs` にまとめてある。
- 後続タスクの候補:
  - パーツの不透明度・グルーの強さのブレンドシェイプ (今は検証で拒否。ARKit の `tongueOut` で舌のパーツを出すなど)。
  - エディタの画面 (インスペクタでのブレンドシェイプ格子の表示と編集、ブレンドシェイプのパラメータの作成 UI)。
  - 2 つのブレンドシェイプの組み合わせの補正格子 (軸 2 本) を作るコマンド (今はデータとしては作れるが、`add_keyform_key` は 1 軸の格子しか作らない)。
