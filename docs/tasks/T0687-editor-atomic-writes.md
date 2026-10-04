---
id: T0687
title: "エディタ: 書き出しと設定のファイルを一時ファイル経由で書く"
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0669]
assignee:
---

# T0687: エディタ: 書き出しと設定のファイルを一時ファイル経由で書く

## 目的

T0669 のレビューで見つけた。エディタのバックエンドは、書き出し (`export_bundle`・`export_module`・`write_png_file`・
`write_json_file`・MCP の `save_project` の `bundle`) と設定 (`ai-provider.json`・`mcp-bridge.json`) を `std::fs::write` や
`truncate(true)` で直接書く。書いている途中で失敗する (ディスクがいっぱい・強制終了) と、前に書き出した正しいファイルが
途中までのファイルに置き換わる。`mcp-bridge.json` ならトークンと有効の設定が消え、つないだクライアントの設定が使えなくなる。

## 受け入れ条件

- [ ] 同じフォルダーの一時ファイルに書いてから名前を変える (Windows の置き換えを含む)。`mcp-bridge.json` は一時ファイルも
      利用者だけが読める権限で作る
- [ ] 失敗したときに前のファイルが残るテスト
- [ ] `.ugoeproj` の保存 (`ugoe_editor_core::Project::save`) が同じ形かを確かめ、違えば editor-core の側を起票する
- [ ] `just ci` が通る

## 参考

- `apps/editor/src-tauri/src/project.rs` (`export_bundle`)・`module.rs` (`export_module`)・`mcp.rs` (`write_private`)・
  `ai_provider.rs` (`ProviderStore::set`)

## メモ

- T0008: 3 つのアプリに同じ所見がある (同期のコマンド: T0685・T0726・T0819、CSP とコマンドの許可: T0686・T0727・T0820、一時ファイル経由の保存: T0687・T0728)。
  先に終えたタスクの形 (非同期にする関数・CSP の中身・一時ファイルの書き方) に後のタスクが揃える (docs/workflow/code-review.md#m9-の結果)。
