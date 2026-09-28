---
id: T0102
title: モデル検証の完成
status: done
milestone: M1
crates: [ugoe-model]
depends_on: []
assignee: claude-worker-T0102
---

# T0102: モデル検証の完成

## 目的

`ugoe_model::validate` に、docs/spec/model.md#検証 で ★ が付いている検査項目を実装する。

## 受け入れ条件

- [x] ★ の項目がすべて実装され、項目ごとに失敗するケースのテストがある
- [x] エラーは最初の 1 件で止めずに、すべて返す
- [x] docs/spec/model.md の ★ を消す
- [x] `just ci` が通る

## 参考

- `docs/spec/model.md#検証`

## メモ

- `validate` に docs/spec/model.md#検証 の ★ 項目をすべて実装し、★ と注記を消した。エラーは全件を返す。
- ★ 以外に追加した検査 (仕様書にも追記済み): 種類ごとの ID 重複、キーが空のバインド (ランタイムの `locate` が panic するため)、同じ格子での同一パラメータの二重バインド、キーの NaN、パラメータの `mirror` / `group` の参照先。
- キーフォーム数・キー順序・バインドのパラメータ参照は、メッシュだけでなくパーツ・デフォーマ (ワープ / 回転)・グルーの全格子で検査する。
- 循環はパーツ・デフォーマそれぞれで循環ごとに 1 件 (`ParentCycle { cycle }`) 報告する。未定義の親は `UnknownReference` に任せ、循環検査では無視する。
- グルーの頂点インデックスは、参照先メッシュが存在する側だけ検査する。
- 既存のフィクスチャは変更していない。テストは `single_quad()` を変更して各検査の失敗ケースを作っている (`crates/ugoe-model/src/validate.rs`)。
- 後続候補:
  - マスクの自己参照・マスク間の循環、`uvs` の値域 (0..1)、不透明度などの値域、ブレンドシェイプ用パラメータの扱い (T0303) は未検査。
  - 物理・モーション・表情はモデル外のデータのため、パラメータ参照の検査は対象外。モデルと組み合わせて検査する関数は別タスク候補。
  - `ugoe-cli` と `ugoe-runtime` は `ValidationError` を `Debug` / `Display` でそのまま出すだけなので変更不要だった。
