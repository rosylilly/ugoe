---
id: T0915
title: UI 品質: 基準の見直しと、繰り返せる監査の道具
status: todo
milestone: M10
crates: [docs, scripts, tests, justfile, packages, apps/editor, apps/viewer, apps/studio, package.json, pnpm-lock.yaml]
depends_on: []
assignee:
---

# T0915: UI 品質: 基準の見直しと、繰り返せる監査の道具

## 目的

M10 の各タスクが同じ物差しで画面を確かめられるように、UI の品質の基準を見直し、T0907 でスクラッチ領域に置いた監査の道具をリポジトリに入れる。
基準は docs/ui.md (T0915 で足す「UI の品質の基準」を含む)。直せないもの・大きいものは後続のタスクにする (milestone は M10)。

## 受け入れ条件

- [ ] docs/ui.md に「UI の品質の基準」の節を足した: 配置 (情報の優先度・揃え・余白の段・グループの見せ方)、レイアウト (幅と高さの変化・送り・分割)、デザイン (トークンの使い方・アイコン・状態の見せ方・空の状態・読み込み中・エラー)、アクセシビリティ (既存の基準に、スクリーンリーダー・200% の拡大・reduced motion の確かめ方を足す)
- [ ] 3 つのアプリをビルドしてヘッドレスの Chrome で開き (偽のホスト)、ライト・ダーク × 日英 × 幅 × devicePixelRatio で撮って、コントラスト・横のはみ出し・24px 未満の押せるもの・名前の無い操作できる部品・Tab の順を報告する `just ui-audit` がある (T0907 の shots.mjs・audit.mjs を元にする。`just ui-shots` との関係を整理する)
- [ ] axe-core などの検査の道具を入れるかを決めた (入れるなら npm のライセンスの検査を通し、PR に理由を書く)
- [ ] いまの 3 つのアプリの監査の結果を基準線としてメモに書き、M10 の各タスクに振り分けた
- [ ] `just ci` には重いので含めず、docs/testing.md に回し方を書いた

## 参考

- docs/ui.md
- docs/ui.md の「アクセシビリティの基準」と「3 つのアプリの見直し (T0907)」
- T0907 のメモ (道具の作り)
- T0006 (実機の e2e。WebKitGTK で撮れるかの参考)

## メモ
