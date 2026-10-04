---
id: T1605
title: "エディタ: Undo の上限の設定と、戻れない・進めないときの Undo/Redo のボタン"
status: done
milestone: M9
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T1605
---

# T1605: エディタ: Undo の上限の設定と、戻れない・進めないときの Undo/Redo のボタン

## 目的

T0680 で Undo の履歴に上限 (既定 100) を設け、`Document::set_undo_limit` を用意したが、エディタの設定には出していない
(大きなモデルでは 1 状態が数 MB になり、上限を下げたい利用者がいる)。また、画面の Undo / Redo のボタンは常に押せ、戻れないときは何もしない。

## 受け入れ条件

- [x] `Document` に `can_undo`・`can_redo` (と必要なら履歴の数) があり、テストがある
- [x] 編集・Undo・Redo の後に、その状態が画面へ届き、ボタン (とメニューの項目) が押せなくなる
- [x] エディタの設定に Undo の上限があり、変えると `set_undo_limit` が呼ばれ、次の起動でも覚えている (範囲と既定値を決めた)
- [x] `apps/editor/src/lib/ipc.ts` と `docs/editor.md` を更新した
- [x] `just ci` が通る

## 参考

- T0680 のメモ (起票の候補)
- crates/ugoe-editor-core/src/document.rs (`set_undo_limit`)

## メモ

- T0019 で起票した (元の候補: T0680 の 2 つ)。

### やったこと

- editor-core: `Document::can_undo`・`can_redo`・`undo_count`・`redo_count` を足した (`History` の数を返すだけ)。テスト `undo_tests::can_undo_and_can_redo_follow_the_history`
  (編集・Undo・Redo・拒否・上限での切り捨て・Undo の後の編集・上限を 0 に)。
- エディタのバックエンド: `src-tauri/src/history.rs` に `HistoryStatus` (`can_undo`・`can_redo`・`undo_count`・`redo_count`・`undo_limit`)・
  IPC の `history_status`・`set_undo_limit(limit)` (async。ファイルを書くので)・上限の置き場 `UndoLimitStore` を置いた。
  上限はアプリの設定のディレクトリの `editor-settings.json` (`{"undo_limit": N}`) に書き、起動時に読んで開いているドキュメントに当てる。
  範囲の外は新しいコード `editor.invalid_undo_limit` (`min`・`max`)。`acl.rs`・`capabilities/default.json`・`ipc.ts`・`just gen` の生成物・翻訳 (en/ja) も直した。
- プロジェクトを開く・作る・開き直す 7 か所 (`project.rs`・`module.rs`・`sample.rs`) を `project::replace_document` にまとめ、前のドキュメントの上限を引き継ぐ
  (これが無いと別のプロジェクトを開くと既定の 100 に戻る)。テスト `project::tests::replacing_the_document_keeps_the_undo_limit`。
- フロントエンド: `EditorDocument.history` を `refresh` で `history_status` とともに読み直し (編集・Undo・Redo・エージェントの変更の後)、`canUndo`・`canRedo` を足した。
  ヘッダーの「Undo」「Redo」は押せないときに無効にする。上限の変更 `setUndoLimit` は編集の列に並べる。テストは `document.test.ts` の `EditorDocument.history`。
- 設定の「General」に「Undo history」(`src/lib/UndoLimitSetting.svelte`。NumberField で 1〜1000、確定するとすぐに当てる) を足した。
- `docs/editor.md#undo` に上限の設定・範囲と既定値・保存先・引き継ぎ・ボタンの扱いを、Tauri コマンドの表とエラーのコードの一覧にも書いた。

### 判断したこと

- **範囲は 1〜1000、既定は 100 (`DEFAULT_UNDO_LIMIT`)。** 0 (履歴なし) は誤操作を戻せなくなるので選ばせない (API の `set_undo_limit(0)` はそのまま)。
  上は 1 状態が数 MB のモデルで数 GB にならないよう 1000 で止めた。
- **保存先はバックエンドのファイル (localStorage ではない)。** AI のプロバイダの設定 (`ai-provider.json`) と同じ置き方で、画面が立ち上がる前
  (エージェントの Undo など) から効き、Rust でテストできるため。エディタ全体の設定で、プロジェクトには残さない。
- **「メニューの項目」:** エディタに Undo / Redo のメニューの項目は無い (ネイティブのメニューも、ファイルメニューの項目も無い) ので、ヘッダーのボタンだけを無効にした。
  ショートカットは今のまま (押せないときに押してもバックエンドが `false` を返すだけ)。

### やらなかったこと・確かめていないこと

- 実際にエディタを起動して画面で確かめてはいない (svelte-check・vitest・Rust のテストまで)。
- MCP の `undo`・`redo` のツールの結果に戻れる・進めるかを載せること (範囲外)。

### 起票の候補

- MCP のツール (`ugoe-mcp`) の状態に `can_undo`・`can_redo` を載せ、エージェントが戻れないのに Undo を求めて確認を出させないようにする。
