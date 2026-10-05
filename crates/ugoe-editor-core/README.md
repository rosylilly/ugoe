# ugoe-editor-core

**層:** T2 エディタ中核 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

ヘッドレスなエディタの中核。`Document`、`Command`、Undo/Redo、プロジェクトの読み書き。

## 守ること

- UI・Tauri・GPU に依存しない
- すべての編集は `Command` を通す。`Document::apply` は失敗時にモデルを変更しない
- `Command` のバリアントは末尾に追加する
- 画面に出すエラー・警告には `Coded` (`src/message_codes.rs`) で安定したコードと引数を付け、`Message` で渡す。**コードは名前を変えない**。足したら `docs/editor.md#エラーと警告のコード` の一覧と、見本 (`src/message_catalog.rs`。翻訳のカタログの検査に使う) にも書く ([ADR 0029](../../docs/adr/0029-shared-ui-and-i18n.md))
- エラー型 (`EditError`・`ModuleError` など) の ID は `String` のまま、データとして返す型の ID は `Id<T>` で持つ (理由は [docs/spec/model.md#ID](../../docs/spec/model.md#id)。T1600)
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
- T0677 (1000 行を超えたファイルを分けた。`src/document/` (エラー・削除の連鎖・テクスチャのコマンド・左右の対)・`src/module_compare/` (アトラスを戻す部分・要素ごとの中身の違い)・`src/project/` (モジュールのプロジェクトの読み書き・パス・ファイルの補助)・`src/module_project/split.rs` (`Splitter`))
- T0680 (Undo の履歴の上限と、状態ごとの検証の結果の記憶。`src/document/history.rs`・テスト `src/undo_tests.rs`)
- T0162 (テクスチャの PNG のデコードを `ugoe_format::image` に寄せた。`src/texture.rs` の `decode_png`。上限を超える画像は `*.texture_too_large` で拒む)
- T0560 (テクスチャと法線マップの追加・差し替えのコマンドが、一辺が上限を超える PNG を置く前に `edit.texture_too_large` で拒む。`src/document/texture_edit.rs` の `png_dimensions`)
- T1605 (`Document::can_undo`・`can_redo`・`undo_count`・`redo_count`。エディタの Undo のボタンと上限の設定が使う)
- T1600 (エラー型の ID は文字列のまま、`ColliderReference` の ID を型付きにした)
- T1607 (プロジェクト・本体・指紋が、通常のファイルだけを大きさの上限付きで読む。`src/project/read.rs` の `ProjectReadLimits` (既定は JSON 256 MiB・画像 512 MiB・本体の `.ugoe` 512 MiB。値と理由は [docs/editor.md#読むファイルの種類と大きさ](../../docs/editor.md#読むファイルの種類と大きさ))。拒んだら `project.not_regular_file`・`project.file_too_large`)
- T0167 (合成のエラーのコード `compose.*` の後ろの部分を ugoe-runtime の `ComposeError::code` に揃え、テスト `source_error_codes_follow_the_crate_codes` で一致を確かめる)
- T1618 (`Coded::code` を `Cow<'static, str>` にし、`validation.*`・`infer.*`・`world.*`・`normal_map.*`・`compose.*` を元のクレートのコードから組み立てる。手で並べた一覧をなくした)
- T1609 (`SetModuleSettings` の `keep_look` の作り直しの警告を `ApplyReport` で返す)
- T1615 (PSD のレイヤーの分類の直し `Command::SetLayerCorrection`。`src/classification.rs`。直しは `Document` のスナップショットに持ち、`project.json` の `editor.classification` に残す)
- T0567 (PSD のレイヤーの分類の答えのキャッシュ `.ugoe-cache/classification.json`。`src/classification_cache.rs`。`project.json` は変えず、Undo の対象にしない。分類した PSD の大きさと FNV-1a のハッシュを添え、使う側 (エディタ・MCP サーバ) が今の PSD と比べる。[docs/spec/file-formats.md#分類のキャッシュ](../../docs/spec/file-formats.md#分類のキャッシュ))
