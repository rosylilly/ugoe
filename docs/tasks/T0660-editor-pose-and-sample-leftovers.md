---
id: T0660
title: エディタ: プレビューでのポーズの切り替え・名前で見つけた差分の追従・物理の無い本体の衝突体の案内・見本の肩の衝突体
status: in-progress
milestone: M4
crates: [ugoe-ai, ugoe-editor-core, ugoe-mcp, ugoe-cli, assets, apps/editor, docs]
depends_on: [T0523, T0655]
assignee: claude-worker-T0660
---

# T0660: ポーズと見本の残り

## 目的

T0523・T0655 の後続の候補をまとめる。

## 受け入れ条件

- [ ] プレビューモードのパネルで、ポーズの表示する要素を切り替えられる
- [ ] 名前で見つけた衣装の差分も、見えている / 隠してあるに応じて本体の差分に追従させる
- [ ] ugoe-mcp の set_layer_labels で variant_sets を受ける
- [ ] エディタで、物理の無い本体が公開する衝突体をモジュールのリグが参照しているときの案内 (今はエラーになる)
- [ ] 見本の本体に肩の衝突体を入れ直し、スロット Hair で公開する (見本の作り直し)。T0655 の規則で、物理を渡さずに作った Puppet でもモジュールを付けられることを確かめる
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0523-ai-pose-and-rig-notices.md`・`docs/tasks/T0655-sample-projects-and-slot-exposure.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
