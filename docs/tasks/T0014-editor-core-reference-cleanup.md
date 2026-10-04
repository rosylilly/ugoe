---
id: T0014
title: "editor-core: 要素の削除と ID の書き換えで、参照の後始末を 1 か所にまとめる"
status: in-progress
milestone: M9
crates: [ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T0014
---

# T0014: editor-core: 要素の削除と ID の書き換えで、参照の後始末を 1 か所にまとめる

## 目的

M9 のコードレビューの横断の所見 (T0668・T0137。まとめは docs/workflow/code-review.md#m9-の結果)。

- 要素を消すコマンドは、消した要素を指す参照 (ミラー元・対・物理・スロットなど) をそれぞれの場所で外している。`RemoveDeformer` と `UnwrapDeformer`、
  `RemoveMesh` と `RemoveGlue` のように同じ削除を別の経路で行うコマンドで外し忘れが起きた (T0667・T0668 で直した)。
- キーフォーム格子をたどるコードで、ブレンドシェイプの格子 (`blend_shapes`) の漏れがあった (T0137 の合成の付け替え)。格子の中の ID を書き換えるコード
  (改名・ミラー・複製) も、ブレンドシェイプ格子までたどっているかを確かめていない。

削除の後始末を `apply_with_report` の後段 (`physics_edit::forget_removed` と同じく、前後のモデルを比べて消えた ID から外す) にまとめると漏れにくい。

## 受け入れ条件

- [ ] 削除の後の参照の後始末を 1 か所にまとめ、各コマンドの個別の後始末を外した (既存のテストが通る)
- [ ] 改名・ミラー・複製など格子の中の ID を書き換えるコマンドが、ブレンドシェイプの格子も書き換えることをテストで確かめた
- [ ] 「不正だが読み込めるデータ」(空の列・未知の値・長さの合わない格子) で編集の関数がパニックしないことを確かめるテストを足した (T0668 の横断)
- [ ] docs/editor.md#削除の連鎖と拒否 を直した
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0137・T0668 のメモの「横断」

## メモ
