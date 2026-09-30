---
id: T0521
title: エディタと AI: ポーズを手で編集する画面・直接写す形の作り直しの知らせ・差分の判定
status: in-progress
milestone: M6
crates: [ugoe-ai, ugoe-editor-core, apps/editor, docs]
depends_on: [T0519]
assignee: claude-worker-T0521
---

# T0521: ポーズの手での編集と、モジュールの自動リギングの残り

## 目的

T0519 の後続の候補をまとめる。

## 受け入れ条件

- [ ] ポーズのグループを手で編集する画面 (`Command::SetPose` を使う。モジュールのプロジェクトでは本体のグループは読み取り専用)
- [ ] 本体の変形を衣装のメッシュに直接写す形で組んだ後に衣装のメッシュを編集したら、提案し直すよう知らせる
- [ ] 本体のワープと本体のメッシュの動きを合わせて写す (今はどちらか一方)
- [ ] 衣装の差分 (A/B) を、名前の末尾以外 (絵の重なり・意味ラベル) でも判定する
- [ ] 録画した応答でテスト。`just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0519-ai-module-rig-followups.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
