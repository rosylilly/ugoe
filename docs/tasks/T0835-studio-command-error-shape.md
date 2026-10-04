---
id: T0835
title: "スタジオ: コマンドのエラーを { code, args, message } の形にそろえる"
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee:
---

# T0835: スタジオ: コマンドのエラーを { code, args, message } の形にそろえる

## 目的

エディタとビューア (T0729) の Tauri のコマンドは、失敗を `{ code, args, message }` (ADR 0029) で返し、画面がコードから訳す。
スタジオのコマンドのエラーは、引数が平らに並び (`args` にまとまっていない)、`message` の無いものがある (T0729 の候補)。
同じ形にそろえ、操作ウィンドウと出力ウィンドウの訳し方を共通にする。

## 受け入れ条件

- [ ] スタジオのコマンドのエラーがすべて `{ code, args, message }` で返る。形を固定するテストがある
- [ ] 画面がコードと `args` から訳す (知らないコードは `message`)。翻訳のキーの検査が通る
- [ ] 外部 API (`docs/studio-api.md`) のエラーの形との関係を確かめ、違うなら理由を書いた
- [ ] `just ci` が通る

## 参考

- T0729 のメモ (起票の候補)、ADR 0029
- apps/viewer/src-tauri/src/error.rs (T0729 の形)

## メモ

- T0019 で起票した (元の候補: T0729 のスタジオ)。
