---
id: T0205
title: 参照レンダラ: マスクの意味論 (ADR 0009)
status: done
milestone: M1
crates: [ugoe-render]
depends_on: [T0107]
assignee: claude-worker-T0205
---

# T0205: 参照レンダラ: マスクの意味論 (ADR 0009)

## 目的

T0202 のマスクアトラス方式を拡張し、[docs/spec/rendering.md#マスク](../spec/rendering.md#マスク) の意味論をすべて実装する。

## 受け入れ条件

- [x] 和集合を `1 − Π(1 − c)` にする (例: マスクをクリアを 1 にしたチャンネルへ (Zero, OneMinusSrc) で描き、読むときに 1 から引く)
- [x] 通常マスクと反転マスクを 1 つのメッシュで併用できる
- [x] `mask_mode = intersection` を実装する
- [x] 入れ子の継承: マスク用メッシュが自身のマスクを持つとき、その係数を掛けたカバレッジでマスクを描く (T0107 の依存順を使う)
- [x] `mask_strength` を反映する
- [x] `mask_only` のメッシュは色として描かれず、マスクとしては働く
- [x] 既定の精度: 各マスクを画面上の大きさ以上の解像度で描く (アトラスのセルが足りなければアトラスを増やす)。解像度を下げるオプションを `RendererOptions` に持つ
- [x] WebGL2 で使える機能だけで実装する (`just wasm-check`)
- [x] 上の各項目に GPU テストがある (`just test-gpu`)
- [x] `docs/spec/rendering.md` の「実装待ち」と、T0202 時点の実装方式の説明を更新する
- [x] `just ci` が通る

## 参考

- `docs/adr/0009-clipping-mask-semantics.md`
- `docs/spec/rendering.md#マスク`

## メモ

- 方式: マスクアトラス (RGBA8) を 1 でクリアし、乗算合成 (dst × src) で `Π(1 − c)` を作る。R・G・B を内容に、A を作業領域に使う。
  - 和集合は `Product(P)` を 1 から引いて読む。反転マスクだけの要素は `Product(N)` をそのまま読むので、同じ集合なら同じセルを共有する。
  - 通常と反転の併用・2 つ以上の共通部分は `Clip` (係数 `m` そのもの)。作業領域に `Π(1 − c)` (共通部分はマスクごとに `1 − c`) を作り、
    全面描画 (合成 `dst × (1 − dst.a)`、チャンネルごとの書き込みマスク) でチャンネルに掛ける。メッシュの外では 0 が掛かるので共通部分が正しくなる。
  - 入れ子: マスク用メッシュのカバレッジに自身の係数 `k` (mask_strength を含む) を掛ける。内容のレベル (マスク用メッシュの `mask_depth` の最大値) ごとに
    別のアトラスに描き、小さいレベルから描く。外接矩形はレベルの大きい順に伝播する。
  - 精度: 外接矩形を画素の境界に広げ、セルの内側を `画素数 / downscale` texel にする (既定では texel = 画素)。棚詰めで詰め、
    アトラスの一辺は 2 の冪で大きくするだけ (上限 `max_size`、既定 4096 とデバイス上限の小さい方)。
- 公開 API の変更: `MaskAtlasOptions { size, min_cell_size }` を `{ max_size, downscale }` に置き換えた。`FrameStats` の意味は
  「集合」→「内容」になったが、フィールドは同じ。`Renderer::new` / `with_options` / `set_texture` / `encode` / `View` は変えていない。
- GPU テストの期待値の変更: `overlapping_masks_saturate_at_full_coverage` を `overlapping_masks_combine_as_union` にし、
  0.75 + 0.75 の結果を 1 (飽和加算) から `1 − 0.25²` に変えた (ADR 0009 の和集合の式)。`mask_sets_overflow_into_more_atlases` は
  新しいオプション (`max_size: 16`) で溢れさせるようにした。
- `encode` はマスクの解像度を `target.texture().size()` から決める。
- 見送り: `DrawableChanges::POSITIONS` を使った頂点位置の差分アップロード。1 つの `Renderer` で同じモデルの複数のインスタンスを描く場合や、
  `update` を 2 回呼んでから描く場合に、バッファの中身とインスタンスの変更フラグがずれるため。インスタンスを識別する仕組みと合わせて後続タスクで扱う。
- 後続の候補: アトラスが大きくなったまま縮まない (一時的に大きく表示したモデルは最大 4096² を持ち続ける)。必要ならフレーム数を数えて縮める。
