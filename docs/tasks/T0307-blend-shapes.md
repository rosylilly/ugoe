---
id: T0307
title: ブレンドシェイプ
status: todo
milestone: M3
crates: [ugoe-model, ugoe-runtime, ugoe-editor-core]
depends_on: []
assignee:
---

# T0307: ブレンドシェイプ

## 目的

`kind = blend_shape` のパラメータを軸に持つキーフォーム格子を、通常の補間結果に差分として加算する (`docs/spec/evaluation.md#6-ブレンドシェイプ`)。
ARKit のブレンドシェイプのように、互いに独立した表情の差分を重ねられるようにする。

## 受け入れ条件

- [ ] 差分の持ち方を決め、`docs/spec/model.md` の `> 未決:` を解消する (既存の `MeshKeyform` の流用か、専用の型か。ファイル形式を変えるなら `docs/spec/file-formats.md` と `SCHEMA_VERSION` も)
- [ ] `validate` がブレンドシェイプ格子の不正 (通常のパラメータとの混在の規則など、決めたもの) を検査する
- [ ] ランタイムが仕様の式で加算する。メッシュとデフォーマ (ワープの格子点・回転の角度など、決めた範囲) のテスト、既定値では変化がないテスト、複数の格子の加算の順のテスト
- [ ] エディタのコマンドが、ブレンドシェイプのパラメータを作ってキーを置けるか確かめる (必要なら直す。UI は範囲外)
- [ ] `just ci`・`just wasm-check` が通る

## 参考

- `docs/spec/evaluation.md#6-ブレンドシェイプ`
- `docs/spec/model.md#ブレンドシェイプ-m3`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
