---
id: T0025
title: 一時ファイルを経た置き換えを 1 か所にまとめ、MCP の保存の書き込みをロックの外へ出す
status: in-progress
milestone: M9
crates: [ugoe-editor-core, ugoe-mcp, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0687]
assignee: claude-worker-T0025
---

# T0025: 一時ファイルを経た置き換えを 1 か所にまとめ、MCP の保存の書き込みをロックの外へ出す

## 目的

「同じフォルダーの一時ファイルに書いてから名前を変える」処理が、ビューア (`apps/viewer/src-tauri/src/atomic_write.rs`、T0728)・
エディタ (T0687)・スタジオ・`ugoe-mcp` (`StagedFile`、T0552)・`ugoe-editor-core` (`write_if_changed`) に別々にある (T0728 の候補)。
片方だけ直る (Windows の置き換え・権限・失敗時の後始末) のを避けるため、1 か所にまとめる。

あわせて、エディタの MCP の `save_project` は、ドキュメントのロックを握ったままファイルを書く (T0685 の候補。写しを取ってロックを放してから書き、
保存の印を付ける)。

**判断が要る:** 置き場所。候補は (a) `ugoe-editor-core` に置き、アプリと `ugoe-mcp` が使う (ビューア・スタジオが editor-core に依存してよいか)、
(b) ツール層の小さなクレートを新しく作る (`scripts/check-layers.mjs`・`ci-groups.mjs`・commitlint の SCOPES への登録が要る)、
(c) GPL のアプリで共有する場所を作る。ライセンス (MIT のクレートと GPL のアプリ) と依存方向から決め、決めた理由を docs に書く。

## 受け入れ条件

- [ ] 置き場所を決めた (上の判断)。利用者の判断が要るなら案と推奨をメモに書いて止まる
- [ ] 3 つのアプリ・`ugoe-mcp`・`ugoe-editor-core` の写しを、まとめた 1 つに置き換えた。失敗したときに前のファイルが残るテストと、利用者だけが読める権限で作るテストがまとめた側にある
- [ ] エディタの MCP の `save_project` が、ロックを放してからファイルを書く (書いている間にほかの読むだけのツールが答えるテスト)
- [ ] `just ci` が通る

## 参考

- T0728・T0552・T0685 のメモ (起票の候補)、T0687
- apps/viewer/src-tauri/src/atomic_write.rs

## メモ

- T0019 で起票した (元の候補: T0728 の 1 つにまとめる、T0552 の `EditorHost::save` (書き方そのものは T0687 の範囲)、T0685 の `save_project` のロック)。
- エディタの一時ファイル経由の書き方は T0687 が入れるので、その後に着手する。
