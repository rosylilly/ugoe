---
id: T0520
title: AI: すべてのテクスチャの法線マップを AI タブから提案する・眼鏡などの扱い
status: in-progress
milestone: M6
crates: [ugoe-ai, apps/editor, docs]
depends_on: [T0518]
assignee: claude-worker-T0520
---

# T0520: AI: 法線マップの提案の画面

## 目的

T0518 の後続の候補をまとめる。

## 受け入れ条件

- [ ] AI タブから、すべてのテクスチャの法線マップをまとまりごとに提案し、プレビューして採用できる
- [ ] 眼鏡など、ふくらませず向きだけを受けるまとまりを足す (意味ラベルの扱い)
- [ ] マニュアル (lighting.md・ai.md) の画像を撮り直す
- [ ] 録画した応答でテスト。`just ci`・`just test-gpu`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0518-ai-normal-map-groups.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
