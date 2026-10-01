---
id: T0722
title: ビューア: 待機モーションをモデルごとに覚える・優先をパラメータごとに・モーションの優先を C ABI と npm に出す
status: in-progress
milestone: M4
crates: [apps/viewer, ugoe-wasm, ugoe-ffi, bindings/web, docs]
depends_on: [T0721]
assignee: claude-worker-T0722
---

# T0722: ビューアのモーションの優先の残り

## 目的

T0721 の後続の候補をまとめる。

## 受け入れ条件

- [ ] 待機モーションの選択をモデルごとに覚え、覚えたものの一覧に出す (`modelMemory.ts`)
- [ ] トラッキング中のモーションの優先をパラメータごとに切り替えられる (頭だけ・目は除くなど)
- [ ] モーションの優先 (`set_motion_over_tracking`) を C ABI と npm パッケージ (`<ugoe-avatar>`) にも出す。ヘッダと README を揃える
- [ ] `just ci`・`just wasm-check`・`just conformance --only c,wasm`・ビューアのビルドが通る

## 参考

- `docs/tasks/T0721-viewer-motion-and-tracking.md` のメモ

## メモ
