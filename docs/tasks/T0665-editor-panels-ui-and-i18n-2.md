---
id: T0665
title: エディタ: パネルの移行 2 (物理・ライティング・スロット・ポーズ・グルー・検証・AI・エージェント・書き出し・プレビュー)
status: in-progress
milestone: M8
crates: [apps/editor, docs]
depends_on: [T0663]
assignee: claude-worker-T0665
---

# T0665: エディタ: パネルの移行 2 (物理・ライティング・スロット・ポーズ・グルー・検証・AI・エージェント・書き出し・プレビュー)

## 目的

T0663 の続き。残りのパネルを共有部品と翻訳に移す。T0664 と並行できるよう、触るディレクトリを分ける。

## 受け入れ条件

- [ ] panels/physics・lighting・slots・poses・glues・verify、ai・agents、ExportPanel、preview を共有部品に置き換え、文言をカタログに移す
- [ ] T0908 の検査で、エディタの許す一覧が空になる (T0664 と後から入る方が空にする)
- [ ] `just ci` が通る

## 参考

- T0663 のメモ

## メモ
