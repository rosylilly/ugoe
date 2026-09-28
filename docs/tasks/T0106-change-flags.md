---
id: T0106
title: 評価結果の変更フラグ
status: done
milestone: M1
crates: [ugoe-runtime]
depends_on: [T0104]
assignee: claude-worker-T0106
---

# T0106: 評価結果の変更フラグ

## 目的

描画可能要素ごとに、前フレームから頂点・不透明度・描画順・可視性が変わったかを示すフラグを出力する (エンジン統合での転送量を減らすため)。

## 受け入れ条件

- [x] フラグの API とテストがある
- [x] docs/spec/evaluation.md#8-出力 を更新する
- [x] `just ci` が通る

## 参考

- `docs/spec/evaluation.md#8-出力`

## メモ

- API: `DrawableChanges` (手書きの `u8` ビット集合。`POSITIONS` `OPACITY` `DRAW_ORDER` `VISIBILITY` `MULTIPLY_COLOR` `SCREEN_COLOR` `MASK_STRENGTH` `ALL` `NONE`、`contains` / `intersects` / `bits` など)、`ModelInstance::changes()` (`drawables()` と同じ並び)、`render_order_changed()`、`mark_all_changed()`。既存の公開 API は変えていない。
- 比較はビット列の一致 (許容誤差なし・決定的)。`-0.0` と `0.0` は別、同じビット列の NaN は同じ。`mask_strength` はクランプ後の値で比べる。
- 最初の報告: `new` の直後と、ホストが最初に呼ぶ `update()` の両方で `ALL` (new の直後に update してから読むホストが最初の転送を取りこぼさないため)。
- 実装: 前回の出力の控え (`previous`、`previous_render_order`) を `new` で確保し、`update()` の最後に比べる。頂点位置は変わったときだけコピーする。ヒープ確保なし (`update_does_not_reallocate_buffers` に控えのバッファも加えた)。
- 後続: `ugoe-render` は今も毎フレーム全頂点を転送している。`changes()` の `POSITIONS` を見て転送を省く変更は、レンダラを拡張する T0205 で行うのがよい (本タスクでは触っていない)。`ugoe-ffi` / `ugoe-wasm` / `ugoe` (Puppet) への公開も未着手 (C ABI では `bits()` の `u8` 配列として渡す想定)。
