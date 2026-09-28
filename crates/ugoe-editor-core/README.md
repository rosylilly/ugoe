# ugoe-editor-core

**層:** T2 エディタ中核 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

ヘッドレスなエディタの中核。`Document`、`Command`、Undo/Redo、プロジェクトの読み書き。

## 守ること

- UI・Tauri・GPU に依存しない
- すべての編集は `Command` を通す。`Document::apply` は失敗時にモデルを変更しない
- `Command` のバリアントは末尾に追加する
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
