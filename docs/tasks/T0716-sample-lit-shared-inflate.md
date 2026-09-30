---
id: T0716
title: 見本: 法線マップ付きの見本を共通の関数 (group_bulge_normal_maps) で作り直す
status: in-progress
milestone: M4
crates: [ugoe-cli, assets, docs]
depends_on: [T0518]
assignee: claude-worker-T0716
---

# T0716: 見本: avatar-lit を共通の関数で作り直す

## 目的

T0518 の後続の候補。T0518 で、部位のまとまりでふくらませる方法を `ugoe_rigging::group_bulge_normal_maps` に一般化した。
見本の生成 (`crates/ugoe-cli/src/sample/lit.rs`) は、独自の同じ処理 (と半画素のずれ) を持ったままになっている。共通の関数に寄せると輪郭の画素が変わる
(T0518 の見積もり: 輪郭で最大 110°、形の中の平均 0.02〜11°) ので、見本を作り直す。

## 受け入れ条件

- [ ] lit.rs を `group_bulge_normal_maps` と `NormalMap::downscale` に置き換え、独自の `extend_edges` と `downscale` を消す
- [ ] `assets/samples/avatar-lit.ugoe` と `avatar-lit.jpg` を作り直し、前後の絵を並べて目で確かめる。輪郭の違いの大きさをメモに書く
- [ ] avatar.ugoe と着せ替えの見本は変えない。ファイルの大きさが大きく増えない。最新であることのテストは今の許容で通す
- [ ] `just ci`・`just test-gpu` が通る

## 参考

- `docs/tasks/T0518-ai-normal-map-groups.md`・`docs/tasks/T0707-lit-sample-avatar.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
