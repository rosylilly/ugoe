---
id: T0206
title: 参照レンダラ: マスクアトラスの縮小
status: done
milestone: M2
crates: [ugoe-render]
depends_on: [T0205]
assignee: claude-worker-T0206
---

# T0206: 参照レンダラ: マスクアトラスの縮小

## 目的

T0205 のマスクアトラスは必要に応じて大きくなるが、小さくならない。一度拡大表示すると 4096² (約 64MB) のまま残るので、使われなくなったら縮小・解放する。

## 受け入れ条件

- [x] 一定フレーム使われなかったアトラスの領域を縮小・解放する (ヒステリシスを持たせ、毎フレームの再確保を避ける)
- [x] 拡大 → 縮小を繰り返してもメモリが増え続けない GPU テストがある
- [x] `docs/spec/rendering.md` の実装方式を更新する
- [x] `just ci` と `just test-gpu` が通る

## 参考

- `docs/spec/rendering.md#マスク`
- `docs/tasks/T0205-render-mask-semantics.md` のメモ

## メモ

- 方針: `mask::AtlasPool` がアトラスの一辺と枚数をフレームをまたいで決める。大きくするのは即時、小さくするのは
  条件 (一辺: そのフレームだけで要る一辺が今の半分以下 / 枚数: 今より少なくて済む) が `shrink_after_frames` フレーム続いてから。
  縮めた後の値は待っていた間に要った値の最大値。マスクを使わないフレームが続いたらすべて解放する。
  そのフレームだけで要る一辺は `allocate(requests, 0, ..)` で求め、決めた一辺が違うときだけ詰め直す (CPU の棚詰めを 2 回するだけ)。
- 公開 API の追加 (追加のみ): `MaskAtlasOptions::shrink_after_frames` (既定 120、0 で縮小しない)、`FrameStats::mask_atlas_bytes`。
  どちらも構造体のフィールド追加なので、構造体リテラルで全フィールドを書いている利用者 (リポジトリ内では GPU テストだけ) は追従が要る。
- テスト: CPU の単体テスト (即時の拡大・遅延した縮小・揺れで作り直さない・溢れた分と未使用の解放・0 で無効) と、
  GPU テスト (拡大 → 縮小で N フレーム後にバイト数が減り、縮小前後でマスクが正しい / 拡大縮小の繰り返しで最大値を超えない /
  溢れたアトラスと画面外で未使用のアトラスの解放)。`just test-gpu` と `WGPU_BACKEND=gl just test-gpu` で確認した。
- GPU テストの `Scene::try_render` を、同じレンダラで複数フレーム描ける `Scene::prepare` + `render_frame` に分けた。
