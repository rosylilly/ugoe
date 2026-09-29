---
id: T0208
title: 参照レンダラ: 描画の自動統合
status: done
milestone: M4
crates: [ugoe-render]
depends_on: [T0205]
assignee: claude-worker-T0208
---

# T0208: 参照レンダラ: 描画の自動統合

## 目的

描画順で隣り合い、描画の状態が同じ要素を 1 回の描画にまとめる。要素ごとの値は頂点属性またはインスタンスのデータで渡す。

## 受け入れ条件

- [x] 描画順を入れ替えない
- [x] 統合した結果が、要素ごとに描いた結果と一致する GPU テストがある
- [x] `FrameStats` に描画回数・状態の切り替え回数・転送バイト数を出す
- [x] WebGL2 で動く (`just wasm-check`、GL の GPU テスト)
- [x] `docs/spec/rendering.md#描画の最適化` を更新する
- [x] `just ci` が通る

## 参考

- `docs/adr/0016-draw-batching.md`
- `docs/spec/rendering.md#描画の最適化`

## メモ

- 方式: 要素ごとの値 (乗算色・不透明度・スクリーン色・マスクの読み方と `mask_strength`) を `Rgba32Float` のテクスチャ
  (要素 1 つ = 5 texel、1 行に最大 64 要素) に置き、静的な頂点属性 (要素の番号、`u32`) で頂点シェーダが `textureLoad` して flat で渡す。
  WebGL2 で使えないストレージバッファと `base_vertex` を避けるため。
  - 本描画のインデックスは `u32` の全体の頂点番号で、描く要素の分を描画順に並べて転送する (控えと違う範囲だけ)。まとめた描画はその連続した範囲を 1 回で描く。
  - マスクパスは従来どおりマスク用メッシュを 1 つずつ、動的オフセットのユニフォームで描く (宣言順の静的な `u32` インデックスを使い、スライスの束縛はやめた)。
  - 描画の状態 = パイプライン (合成モード・カリング) + テクスチャ + 読むマスクアトラス。マスクを読まない要素はどのアトラスとも両立するとした
    (仕様の「マスクの内容 (アトラスとセル)」から緩めた。セルは要素ごとの値なので状態にしなくてよい)。
- 公開 API の追加 (追加のみ): `FrameStats::{draw_calls, state_changes, index_bytes, uploaded_bytes}`、`RendererOptions::batch_draws` (既定 `true`)。
  `RendererOptions` は `Default` を手書きにした (構造体リテラルで作るコードは `..RendererOptions::default()` が要る。GPU テストの `shrinking_options` を直した)。
  `uniform_bytes` は要素ごとの値 (テクスチャへの転送) を含む。
- GPU テストの追加: 統合あり・なしで画素が完全に一致すること (重なる半透明の要素、マスクあり・なしの混在、入れ子・共通部分・複数アトラス、80 要素)、
  状態が変わる所で描画が分かれること、描画順が変わったときだけインデックスを転送すること。lavapipe (Vulkan) と llvmpipe (GL) で通した。
  既存のテストの期待値は変えていない (`static_frames_upload_nothing_after_the_first` に新しいフィールドの 0 を足しただけ)。
- `state_changes` は T0114 (アトラス化) が効果の確認に使う (worker-T0114 に伝えた)。
- 見送り・後続の候補:
  - マスクパスの描画の統合 (同じ内容のマスク用メッシュをまとめる)。今は要素ごとに描く。
  - ADR 0016 の「ベンチマークを CI で記録する」は未着手。
  - 要素の数が `64 × max_texture_dimension_2d` (WebGL2 の既定の上限で 131072) を超えるモデルは扱えない (実用上は問題ない想定)。
