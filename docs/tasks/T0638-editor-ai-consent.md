---
id: T0638
title: エディタ: AI の API キーの保存と送信の同意
status: todo
milestone: M6
crates: [apps/editor, ugoe-editor-core, docs]
depends_on: [T0509]
assignee:
---

# T0638: エディタ: AI の API キーの保存と送信の同意

## 目的

T0509 の `ClaudeProvider` をエディタから安全に使えるようにする。API キーの保存と、画像を外部に送る前の同意の流れを作る。

## 受け入れ条件

- [ ] API キーを OS の資格情報の保存先に保存・削除でき、`ApiKey::new` で渡す。キーは画面・ログ・プロジェクトに残らない
- [ ] 送る前に `Provider::disclosure` (送り先・モデル・画像の枚数とバイト数) を示し、同意を求める。「今後は確かめない」の扱いを決める
- [ ] プロジェクトごとに「外部に送らない」を選べ、選んだプロジェクトでは AI の機能を無効にする
- [ ] ヘッドレスのブラウザで流れを確かめる
- [ ] `just ci`・`just gen`・エディタのビルドが通る

## 参考

- `docs/tasks/T0509-claude-provider.md` のメモ
- `docs/ai-integration.md#セキュリティとプライバシー`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
