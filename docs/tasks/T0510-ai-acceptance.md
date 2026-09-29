---
id: T0510
title: M6 (AI 支援) の受け入れテスト
status: todo
milestone: M6
crates: [ugoe-ai, ugoe-editor-core, ugoe-cli, docs]
depends_on: [T0503, T0504, T0505, T0508, T0509, T0637]
assignee:
---

# T0510: M6 (AI 支援) の受け入れテスト

## 目的

`docs/roadmap.md` の M6 の完了の条件 (PSD から、顔の角度・目・口・呼吸・髪揺れが動くモデルを、提案を採用するだけで作れる) を、録画した応答による自動のテストと、実際のモデルでの確認で示す。

## 受け入れ条件

- [ ] `assets/fixtures/psd-real/` の PSD (バストアップを含む) から、分類 → リギングの提案 → 採用で、顔の角度・目・口・呼吸・髪揺れが動くモデルができるテスト (録画した応答)
- [ ] できたモデルがアバタープロファイルの検査を通る (名乗るレベルを決めて書く)
- [ ] 実際の Claude API で同じ流れを 1 回行い、結果の絵と気づいたことを記録する (費用と時間も)
- [ ] `docs/roadmap.md` の M6 を完了にする
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0310-m3-acceptance.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
