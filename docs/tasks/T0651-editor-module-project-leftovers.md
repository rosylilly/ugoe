---
id: T0651
title: エディタ: モジュールのプロジェクトの残り (無効にしていない欄・見た目を保つ移動・本体の自動の読み直し)
status: in-progress
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0650]
assignee: claude-worker-T0651
---

# T0651: エディタ: モジュールのプロジェクトの残り

## 目的

T0650 の後続の候補をまとめる。

## 受け入れ条件

- [ ] まだ無効にしていない本体の欄を無効にする: Lighting の「Lit meshes」の本体のメッシュ、物理のリグの並べ替え (▲▼)、テンプレートの物理 (TemplateSection) で本体の要素を選べること
- [ ] スロットを取り付け先の違うスロットに移すとき、見た目を保って移す (取り付け先の座標に写し直す) 選択肢
- [ ] 本体の変化の自動の読み直し (外での書き換えの「Reload automatically」と同じ設定にするかを決める)。本体が見つからないときに、保存していない編集があっても本体を選び直せる
- [ ] 本体の問題のエラーの文で、ファイル名を文から見分けられるようにする (今は「The base model moved.ugoeproj no longer has slot…」と続けて読めてしまう)
- [ ] モジュールのプロジェクトのプレビューで、自分のスロットに今の編集を書き出したものを付けて、ほかのモジュールと差し替えて比べられる (今は付けない)
- [ ] ヘッドレスのブラウザで確かめる。マニュアルを更新する。`just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0650-editor-module-usability.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
