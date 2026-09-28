---
id: T0203
title: GPU テストの CI (lavapipe)
status: todo
milestone: M1
crates: [.github/workflows, justfile]
depends_on: [T0201]
assignee:
---

# T0203: GPU テストの CI (lavapipe)

## 目的

GitHub Actions の Linux ランナーで Mesa の lavapipe を入れ、`just test-gpu` を実行するジョブを追加する。

## 受け入れ条件

- [ ] CI で GPU テストが実行される
- [ ] docs/testing.md を更新する
- [ ] `just ci` が通る

## 参考

- `docs/testing.md#描画テスト`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
