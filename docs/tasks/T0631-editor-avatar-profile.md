---
id: T0631
title: エディタ: アバタープロファイルの編集
status: in-progress
milestone: M5
crates: [ugoe-editor-core, apps/editor, docs/manual]
depends_on: [T0110]
assignee: claude-worker-T0631
---

# T0631: エディタ: アバタープロファイルの編集

## 目的

エディタで `Model.avatar` (準拠レベル・メタデータ・利用許諾・トラッキングの上書き・視線・標準の当たり判定) を設定し、準拠の検査の結果を見られるようにする。

## 受け入れ条件

- [ ] `Model.avatar` を設定・変更する `Command` (例: `set_avatar_profile`、部分の更新) と、検証・Undo のテスト
- [ ] インスペクタかダイアログで、メタデータと利用許諾を編集できる。既定値は最も制限の強い値
- [ ] `check_profile` の結果 (名乗るレベル・満たすレベル・エラーと警告) を表示する
- [ ] T0702 の見本のジェネレータが、`Model.avatar` を直接書く代わりにこのコマンドを使う
- [ ] 操作マニュアルを更新し、`just ci`・`just gen` (差分なし)・エディタのビルドが通る

## 参考

- `docs/spec/avatar-profile.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
