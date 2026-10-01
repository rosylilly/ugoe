---
id: T0661
title: エディタ: グルーの頂点の組をビューポートで編集・強さの軸・着せ替えのモジュールのポーズをプレビューで切り替え
status: todo
milestone: M4
crates: [ugoe-wasm, ugoe-editor-core, apps/editor, docs]
depends_on: [T0659, T0660]
assignee:
---

# T0661: グルーとモジュールのポーズの残り

## 目的

T0659・T0660 の後続の候補のうち、エディタの画面で閉じるものをまとめる。

## 受け入れ条件

- [ ] Glue タブで選んだグルーの頂点の組をビューポートに重ねて表示し、ビューポートで組を足す・外す (Command を通す)
- [ ] グルーの強さのキーフォームに軸を足す画面 (今は `add_keyform_key` を送る AI・エージェントだけが足せる)
- [ ] 着せ替えの違いの強調で、除いた要素は今の編集にメッシュが残っていても「removed」と分かる知らせにする
- [ ] プレビューの Pose の節に、着せ替えで付けたモジュールのポーズのグループも出す (WebPuppet にポーズのグループを返す口が要る。ugoe-wasm)
- [ ] 物理の無い本体で開けないモジュールを、衝突体の参照を外して開く選択肢 (外すことを明示して確認する。T0655 の「黙って消さない」を守る)
- [ ] `just ci`・`just gen`・`just wasm-check`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0659-editor-module-diff-poses.md`・`docs/tasks/T0660-editor-pose-and-sample-leftovers.md` のメモ

## メモ
