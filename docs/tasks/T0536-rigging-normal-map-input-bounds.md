---
id: T0536
title: 法線マップの生成で、大きすぎる格子と長さの合わない入力を拒む
status: done
milestone: M9
crates: [ugoe-rigging, ugoe-ai, docs]
depends_on: [T0526]
assignee: claude-worker-T0536
---

# T0536: 法線マップの生成で、大きすぎる格子と長さの合わない入力を拒む

## 目的

T0526 (ugoe-rigging のコードレビュー) で見つかった、法線マップの生成 (`crates/ugoe-rigging/src/normal.rs`・`normal/group.rs`) の
入力の検査の穴を直す。どれもエラーの型 (`NormalMapError`) か公開の関数の形を変えるので、レビューの中では直さなかった。

1. **まとまりの格子の大きさに上限がない (中):** `GroupField::build` は、置いた画素の位置の外接矩形を、最も細かいテクスチャの
   `canvas_per_texel` の升目に分けて格子を作る。遠く離れた位置 (キャンバスの外に置いた隠しパーツの頂点など) や、極端に小さい
   `canvas_per_texel` があると `w × h` が巨大になり、確保に失敗して止まる (またはとても遅い)。`w as u32` / `h as u32` は
   切り捨てになりうる。升目の数に上限を設け、超えたらエラー (新しい `NormalMapError` の種類) にする。
2. **公開の型と関数が長さを検査しない (低):** `HeightField` は公開のフィールドを持ち、`smooth`・`to_normal_map` は
   `values`・`mask` の長さが `width × height` であることを前提に添字を引く (合わなければパニック)。`HeightField::add` は大きさだけを
   比べて `values` の長さを比べない。`fill_missing` と `NormalMap::extend_edges` も `mask` の長さを検査しない。
   コンストラクタを通す・`Result` を返す・長さが合わなければ何もしない、のいずれかに揃える (呼び出し側は `ugoe-ai::normal`)。

## 受け入れ条件

- [x] 巨大な格子になる入力 (離れた位置・極端に小さい `canvas_per_texel`) が、確保の前にエラーになるテストがある
- [x] 長さの合わない `HeightField` などでパニックしないテストがある
- [x] `ugoe-ai` の呼び出し側と `docs/ai-integration.md#法線マップの生成` を合わせて直した
- [x] `just ci` が通る

## 参考

- T0526 のメモ (所見の表)
- docs/ai-integration.md#法線マップの生成、docs/workflow/code-review.md

## メモ

- やったこと:
  - `GroupField::build` は升目の数を f64 で数え、`MAX_GROUP_CELLS` (2²⁵ = 8192 × 4096、公開の定数) を超える・無限大・NaN なら、
    確保の前に新しい `NormalMapError::GridTooLarge { width, height, max }` を返す。上限の内なので `w as u32` は切り捨てにならない。
    上限は、ふくらませる間の 1 升あたりおよそ 40 バイト (約 1.3 GB) と、大きな PSD (4000 × 6000 で 1 テクスチャ画素 = 1 キャンバスピクセル) を
    拒まないことの兼ね合いで決めた。テスト `far_apart_positions_are_rejected_before_allocating`・`tiny_canvas_per_texel_is_rejected`。
  - `HeightField` のフィールドを非公開にし、`HeightField::new` (長さを検査) と読むだけのアクセサ (`width`・`height`・`values`・`mask`) を足した。
    これで `smooth`・`to_normal_map`・`add` は長さが揃っている前提が型で守られる (シグネチャは変えない)。
  - `fill_missing` は `(values, mask, width: u32, height: u32) -> Result<Vec<f32>, NormalMapError>`、`NormalMap::extend_edges` は
    `Result<(), NormalMapError>` を返し、長さが合わなければ何も変えずに `SizeMismatch`。
  - `ugoe-ai::normal` の呼び出し側 (単独のふくらみ・深度からの方法) をテクスチャの名前を添えたエラーに直した。`docs/ai-integration.md` と
    ugoe-rigging の README に上限と検査を書いた。
- やらなかったこと: `NormalMap` は公開のフィールドのまま (`normal`・`downscale` は `get` で引くのでパニックしない。`extend_edges` だけ検査を足した)。
  単独のテクスチャの `bulge_height` の大きさの上限は扱っていない (画像の大きさはデコードの側で決まる)。
- 起票の候補: なし。

