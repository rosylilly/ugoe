---
id: T0501
title: PSD 取り込み
status: done
milestone: M2
crates: [ugoe-psd]
depends_on: []
assignee: claude-worker-T0501
---

# T0501: PSD 取り込み

## 目的

PSD から `LayerTree` を作る。外部クレートを使うか自前で書くかを決める。

## 受け入れ条件

- [x] グループ・表示状態・不透明度・位置が正しく読める
- [x] テスト用の小さな PSD を `assets/fixtures/` に置き、出所を README に書く
- [x] 未対応の機能を含む PSD では警告を返す
- [x] `just ci` が通る

## 参考

- `docs/ai-integration.md#psd-取り込み`

## メモ

- **パーサは自前** (`crates/ugoe-psd/src/parse.rs`、非公開)。`psd` クレート (0.3.5) は境界検査なしのスライスで
  切れた入力にパニックし、画素をキャンバス全体の大きさで返し、`iOpa`・`lyid`・未知のキーを出さないため採らなかった。
  理由の詳細はクレートの README。
- API の追加: `LayerTree::warnings` (`ImportWarning { path, kind: WarningKind }`)、`Layer` に `id` `fill_opacity`
  `clipping`、`LayerNode::Group` に `id` `opacity` `blend_mode` `clipping`。並び順は下から上 (描画順) と doc に明記した。
- 対応: 8 ビット RGB、無圧縮 / PackBits。PSB・16/32 ビット・RGB 以外は `Unsupported`。ZIP 圧縮のチャンネルは
  警告して飛ばす (8 ビットの Photoshop の出力には現れない)。レイヤー情報のない PSD は合成画像を 1 枚のレイヤーにする。
- 警告: 調整・塗りつぶしレイヤー、画素のないスマートオブジェクト (飛ばす)、レイヤー効果・レイヤーマスク
  (無視して画素を取り込む)。レイヤー効果は `lfx2` などのキーの有無だけで判定するので、効果がすべて無効でも警告が出る。
- フィクスチャ `assets/fixtures/layers.psd` はテスト用ライター (`tests/common/mod.rs`) で生成し、
  コミット済みのファイルと一致することをテストする。ImageMagick 6.9 でも読めることを確かめた
  (ImageMagick が書いた 8 ビットの PSD も取り込めた)。
- 後続の候補: レイヤーマスクの適用 (または `Layer` にマスク画像を持たせる)、ZIP 圧縮、グレースケール、
  スマートオブジェクトの埋め込み画像。T0605 では `clipping` をマスクに、`fill_opacity` を `opacity` との積にする。
