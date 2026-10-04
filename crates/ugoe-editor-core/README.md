# ugoe-editor-core

**層:** T2 エディタ中核 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

ヘッドレスなエディタの中核。`Document`、`Command`、Undo/Redo、プロジェクトの読み書き。

## 守ること

- UI・Tauri・GPU に依存しない
- すべての編集は `Command` を通す。`Document::apply` は失敗時にモデルを変更しない
- `Command` のバリアントは末尾に追加する
- 画面に出すエラー・警告には `Coded` (`src/message_codes.rs`) で安定したコードと引数を付け、`Message` で渡す。**コードは名前を変えない**。足したら `docs/editor.md#エラーと警告のコード` の一覧と、見本 (`src/message_catalog.rs`。翻訳のカタログの検査に使う) にも書く ([ADR 0029](../../docs/adr/0029-shared-ui-and-i18n.md))
- `Command` やモデルの型を変えたら `just gen` で生成物 (TS の型・`schema/command.schema.json`) を作り直す ([型の生成](../../docs/editor.md#型の生成))

## 仕様

- [docs/editor.md](../../docs/editor.md)
- [docs/adr/0005-editor-command-model.md](../../docs/adr/0005-editor-command-model.md)

## 関連タスク

- T0601 (`docs/tasks/T0601-*.md`)
- T0603 (`docs/tasks/T0603-*.md`)
- T0604 (`docs/tasks/T0604-*.md`)
- T0611 (推定・ミラーのコマンドと `derived` のセルの自動の作り直し。`src/inference.rs`)
- T0614 (セルに記録した推定の方法での作り直しと、ミラー元が変わったときのミラーのやり直し)
- T0607 (デフォーマの作成・見た目を保つ付け替え・ギズモのセルの編集。`src/deformer_edit.rs`)
- T0630 (モーションと表情を `Document` に持ち、編集するコマンド。`src/motion_edit.rs`)
- T0114 (書き出し時のテクスチャのアトラス化。`src/atlas.rs`)
- T0648 (モジュールのプロジェクト。`src/module_project.rs`)
- T0650 (モジュールの設定の `SetModuleSettings`・本体の読み直しの `Document::rebase_module`・本体の指紋)
- T0651 (`SetModuleSettings` の `keep_look`・本体とスロットの付け替えの `Document::rebase_module_on`・プレビュー用の本体だけの `.ugoe` の `export_base_bundle`)
- T0653 (`keep_look` での衝突体の写し直し・本体の版 `ModuleContext::base_revision`・名前を替えた書き出し `export_module_as`)
- T0654 (今の編集と書き出した `.ugoemod` の違い `compare_with_module_file`。`src/module_compare.rs`・アトラス化を選べるモジュールの書き出し `export_module_with_settings`)
- T0666 (エラー・警告のコードと引数 `Message`・`Coded`。`src/message.rs`・`src/message_codes.rs`)
- T0014 (削除の後の参照の後始末を 1 か所にまとめた `src/cleanup.rs`・不正だが読み込めるデータへの編集のテスト `src/malformed_tests.rs`)
