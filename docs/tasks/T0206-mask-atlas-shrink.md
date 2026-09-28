---
id: T0206
title: 参照レンダラ: マスクアトラスの縮小
status: todo
milestone: M2
crates: [ugoe-render]
depends_on: [T0205]
assignee:
---

# T0206: 参照レンダラ: マスクアトラスの縮小

## 目的

T0205 のマスクアトラスは必要に応じて大きくなるが、小さくならない。一度拡大表示すると 4096² (約 64MB) のまま残るので、使われなくなったら縮小・解放する。

## 受け入れ条件

- [ ] 一定フレーム使われなかったアトラスの領域を縮小・解放する (ヒステリシスを持たせ、毎フレームの再確保を避ける)
- [ ] 拡大 → 縮小を繰り返してもメモリが増え続けない GPU テストがある
- [ ] `docs/spec/rendering.md` の実装方式を更新する
- [ ] `just ci` と `just test-gpu` が通る

## 参考

- `docs/spec/rendering.md#マスク`
- `docs/tasks/T0205-render-mask-semantics.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
