---
id: T0637
title: エディタ: AI の提案のプレビューと採用
status: in-progress
milestone: M6
crates: [ugoe-editor-core, apps/editor, docs/manual, docs/editor.md]
depends_on: [T0503, T0504]
assignee: claude-worker-T0637
---

# T0637: エディタ: AI の提案のプレビューと採用

## 目的

AI の `Proposal` (コマンドの列) を、エディタで差分として確かめ、採用 (1 回の Undo で戻せる) ・却下できるようにする。PSD の取り込みから「分類 → リギングの提案」までを画面から実行できるようにする。

## 受け入れ条件

- [ ] 提案のパネル: 提案の一覧 (題名・理由)、採用・却下・一部だけの採用 (コマンドの単位)
- [ ] プレビュー: 採用する前に、提案を適用した姿をビューポートで確かめられる (文書は変えない)。パラメータを動かして確かめられる
- [ ] 採用は `Batch` として 1 回の Undo で戻る。採用後に手で直したセルは `authored` になる
- [ ] PSD からの流れ: New from PSD の後に「AI でリギングを提案」を実行でき、進み具合と失敗の理由を出す。API キーがないときの案内
- [ ] ヘッドレスのブラウザで、録画した応答を使って確かめる。操作マニュアルに AI の章を足す
- [ ] `just ci`・`just gen`・エディタのビルドが通る

## 参考

- `docs/ai-integration.md`
- `docs/adr/0006-ai-proposals.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
