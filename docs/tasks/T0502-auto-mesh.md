---
id: T0502
title: 自動メッシュ生成
status: done
milestone: M2
crates: [ugoe-mesher]
depends_on: []
assignee: claude-worker-T0502
---

# T0502: 自動メッシュ生成

## 目的

docs/ai-integration.md#自動メッシュ のパイプラインを実装し、暫定の外接矩形の実装を置き換える。

## 受け入れ条件

- [x] 穴のある形状、複数の島がある形状を扱える
- [x] すべての不透明な画素がメッシュに覆われていることを検証するテストがある
- [x] 頂点数が u16 の範囲に収まる
- [x] `just ci` が通る

## 参考

- `docs/ai-integration.md#自動メッシュ`

## メモ

- パイプラインと保証はクレートの README に書いた。膨張は輪郭のオフセットではなく距離変換で行う
  (島の合流・穴の閉塞が自然に扱える)。輪郭は marching squares、簡略化は Douglas–Peucker、
  内部点は六角格子、三角形分割は `spade` (MIT OR Apache-2.0) の制約付きドロネー。
- 被覆の保証: 簡略化の許容誤差を `margin` から求めた余裕の半分未満に抑えるので、通常は輪郭だけで
  すべての不透明画素の中心が内側に入る。安全策として、残っていない三角形に画素の中心が入っていれば
  その三角形を残す (テストの形状では一度も働かなかった)。
- `MeshOptions` に `min_island_area` (既定 0 = 捨てない)、`MeshError` に `InvalidOptions` と
  `TooComplex` を追加した。フィールドの追加なので、構造体リテラルで作る呼び出し側は
  `..Default::default()` が要る。
- 頂点数の上限は 65535 (インデックス 0xFFFF をプリミティブリスタートと衝突させないため)。
- 性能: 1024x1024 のレイヤーで約 70ms (リリースビルド、既定のオプション、453 頂点)。
  `cargo test -p ugoe-mesher --release -- --ignored --nocapture` で測れる。
- 後続の候補: ラベル別プリセット (`ugoe-ai`)、曲率に応じた輪郭の間隔、既存メッシュの再生成時に
  頂点をなるべく保つ (キーフォームの引き継ぎ) 機能。
