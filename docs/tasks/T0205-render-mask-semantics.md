---
id: T0205
title: 参照レンダラ: マスクの意味論 (ADR 0009)
status: in-progress
milestone: M1
crates: [ugoe-render]
depends_on: [T0107]
assignee: claude-worker-T0205
---

# T0205: 参照レンダラ: マスクの意味論 (ADR 0009)

## 目的

T0202 のマスクアトラス方式を拡張し、[docs/spec/rendering.md#マスク](../spec/rendering.md#マスク) の意味論をすべて実装する。

## 受け入れ条件

- [ ] 和集合を `1 − Π(1 − c)` にする (例: マスクをクリアを 1 にしたチャンネルへ (Zero, OneMinusSrc) で描き、読むときに 1 から引く)
- [ ] 通常マスクと反転マスクを 1 つのメッシュで併用できる
- [ ] `mask_mode = intersection` を実装する
- [ ] 入れ子の継承: マスク用メッシュが自身のマスクを持つとき、その係数を掛けたカバレッジでマスクを描く (T0107 の依存順を使う)
- [ ] `mask_strength` を反映する
- [ ] `mask_only` のメッシュは色として描かれず、マスクとしては働く
- [ ] 既定の精度: 各マスクを画面上の大きさ以上の解像度で描く (アトラスのセルが足りなければアトラスを増やす)。解像度を下げるオプションを `RendererOptions` に持つ
- [ ] WebGL2 で使える機能だけで実装する (`just wasm-check`)
- [ ] 上の各項目に GPU テストがある (`just test-gpu`)
- [ ] `docs/spec/rendering.md` の「実装待ち」と、T0202 時点の実装方式の説明を更新する
- [ ] `just ci` が通る

## 参考

- `docs/adr/0009-clipping-mask-semantics.md`
- `docs/spec/rendering.md#マスク`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
