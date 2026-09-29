---
id: T0503
title: レイヤーの意味分類
status: todo
milestone: M4
crates: [ugoe-ai]
depends_on: [T0501]
assignee:
---

# T0503: レイヤーの意味分類

## 目的

ラベルの語彙を定め、`Provider` を使ってレイヤーを分類する。`FakeProvider` でテストする。

## 受け入れ条件

- [ ] ラベルの語彙が定数として定義されている
- [ ] 出力の JSON Schema がある
- [ ] 録画した応答を使うテストがある
- [ ] `just ci` が通る

## 参考

- `docs/ai-integration.md`
- `assets/fixtures/psd-real/` (See-Through の PSD。英語のレイヤー名で、正解のラベルを付けやすい)

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
