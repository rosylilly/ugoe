---
id: T0112
title: 法線マップとライティングのデータ
status: done
milestone: M4
crates: [ugoe-model, ugoe-format, ugoe-editor-core, ugoe-cli, ugoe-runtime, ugoe-rigging, ugoe-editor]
depends_on: [T0109, T0111]
assignee: claude-worker-T0112
---

# T0112: 法線マップとライティングのデータ

## 目的

`TextureRef::normal_map`、`Model::lighting`、`Mesh::lit` と、バンドルの `NRML` チャンクを追加する。

## 受け入れ条件

- [x] 型とシリアライズ、`NRML` チャンクの読み書きがある
- [x] 機能 `lighting.normal_map` を宣言する
- [x] 仕様書の「実装待ち」を更新する
- [x] `just ci` が通る

## 参考

- `docs/adr/0015-lighting.md`
- `docs/spec/rendering.md#ライティング`

## メモ

- 範囲外の変更 (team-lead 承認済み): 構造体リテラルに既定値を足す機械的な修正 (ugoe-editor-core の atlas.rs・document.rs・import.rs・project.rs、ugoe-cli の main.rs、ugoe-runtime/examples/compose_timing.rs、ugoe-rigging/examples/depth_face.rs)、`just gen` の生成物 (`apps/editor/src/lib/generated/types.ts`・`command.schema.json`)。
- `Bundle` に `normal_maps` を足したので、editor-core の `export_bundle` と CLI の `build` は `TextureRef::normal_map` の名前でプロジェクトのテクスチャから法線マップの画像を引いて `NRML` に入れる。
- editor-core のアトラス化は、法線マップを持つテクスチャを詰めずに残す (`KeptReason::HasNormalMap`)。法線マップの画像もそのまま出力に残す。
- `lighting.normal_map` は基準の機能なので `features::detect` には足さない (宣言表には既にある)。
- 残した課題: プロジェクト (`.ugoeproj`) の保存・読み込みは、まだ法線マップの画像を扱わない (`project.rs` の TODO)。後続タスク T0640 (team-lead が起票する)。
