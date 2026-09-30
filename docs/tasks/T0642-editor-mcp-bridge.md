---
id: T0642
title: エディタ: エディタの中の MCP サーバと、エージェントをつなぐ画面
status: todo
milestone: M6
crates: [apps/editor, ugoe-editor-core, docs]
depends_on: [T0516, T0641]
assignee:
---

# T0642: エディタ: エディタの中の MCP サーバと、エージェントをつなぐ画面

## 目的

ADR 0023 の 1・2・4・6 のエディタ側。実行中のエディタに外部のエージェントがつながり、その提案をエディタの画面で確かめて採用できるようにする。

## 受け入れ条件

- [ ] エディタが開いている `Document` を持ち主として、T0516 の API で MCP を待ち受ける (設定で有効にする。既定は無効。ループバック・トークン必須)。発見用のファイルを書き、終了時に消す
- [ ] ブリッジ経由の提案が AI タブの提案の一覧に出る (エージェントからと分かる表示)。プレビュー・一部の採用・Undo は既存の流れ (T0637) のまま
- [ ] エージェントの `apply_proposal`・`save_project` などは、エディタが確認のダイアログを出し、利用者が承認したときだけ行う。拒否・時間切れはエージェントにエラーで返る。プロジェクトの開閉などはブリッジからはできない
- [ ] 設定に「エージェントをつなぐ」の画面: ブリッジの有効化・状態 (つながっているクライアント)・トークンの作り直し、Claude Code・OpenCode・ChatGPT (トンネル)・WSL からの登録のコマンドをコピーできる
- [ ] `ugoe-mcp` をサイドカーとしてエディタに同梱する (Windows では `ugoe-mcp.exe`)。WSL から `ugoe-mcp.exe --editor` を登録する手順が画面とマニュアルにある
- [ ] 操作マニュアルに章 (`docs/manual/mcp.md`): ファイルを直接扱う形とエディタにつなぐ形の違い、各クライアントの登録、頼み方の例、安全のための設定、Windows と WSL での使い方
- [ ] ヘッドレスのブラウザと、テストのクライアント (中継を通す) で、提案 → エディタに出る → 承認 → 適用 → Undo を確かめる
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0023-editor-mcp-bridge.md`
- `docs/tasks/T0516-mcp-document-host.md`・`docs/tasks/T0637-editor-ai-proposals.md`・`docs/tasks/T0638-editor-ai-consent.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
