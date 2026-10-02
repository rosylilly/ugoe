---
id: T0666
title: エディタ: バックエンドのエラーと警告をコードと引数で返し、画面で訳す
status: done
milestone: M8
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0900]
assignee: claude-worker-T0666
---

# T0666: エディタ: バックエンドのエラーと警告をコードと引数で返し、画面で訳す

## 目的

Tauri のコマンドと editor-core が返すエラーや警告は英語の文で、画面で訳せない。コードと引数で返す形にそろえる。

## 受け入れ条件

- [x] editor-core と Tauri のコマンドのエラー・警告・問題 (ModuleProblem など) に、安定したコードと引数を持たせる (文は英語のまま残し、ログや MCP で使う)
- [x] ipc.ts と生成した型を更新し (`just gen`)、画面はコードから文言を引く形にできるようにする (訳の付け替えは T0663〜T0665)
- [x] MCP・CLI の文言は英語のまま変えない
- [x] `just ci`・`cargo check -p ugoe-editor` が通る

## 参考

- ADR 0029

## メモ

- **形:** `ugoe_editor_core::Message` (`code`・`args`・英語の `message`・省略できる `cause`) と、コードを返す
  `Coded` (`src/message.rs`・`src/message_codes.rs`)。`cause` は ADR 0029 の形に足したもの (包むエラー
  `proposal.rejected`・`edit.invalid_cell`・`project.base_unreadable` などで原因の `Message` を持つ。省略すれば ADR の形と同じ)。
  数の引数は JSON の数、NaN・無限大は文字列。
- **editor-core:** `EditError`・`CellError`・`ProposalError`・`ProjectError`・`ModuleError`・`ExportError`・`AtlasError`・
  `OverflowError`・`LightingError`・`SpaceError`・`PsdImportError`・`FormatError`・`CompileError`・`ProfileIssue` に `Coded`。
  警告は `PsdImportWarning`・`RederiveWarning` に `code`・`args` のフィールドを足し、`AvatarCheck` の `errors`・`warnings` と
  `ModuleExport::warnings` を `Vec<Message>` にした。英語の文はすべて前と同じ。
- **Tauri:** コマンドの戻り値を `Result<T, String>` から `IpcResult<T>` (`Result<T, Message>`) にした。バックエンドだけで
  起きるエラーは `src-tauri/src/error.rs` の `EditorError` (thiserror。`editor.*`・`ai.*`・`mcp.*`・`proposal.*`・`lighting.*`・
  `psd.read_file_failed`)。`AiError` は種類ごとのコード。`ModuleProblem` は `Message` を `#[serde(flatten)]` で含む (`message` は
  そのまま)。`McpBridgeStatus.error` と `ModuleExportReport.warnings` も `Message`。`thiserror` を `apps/editor/src-tauri` の
  依存に足した (ワークスペースにあるもの)。
- **MCP・CLI:** 文言は変えていない。エディタの MCP の持ち主 (`mcp.rs` の `EditorHost`) は `Message` の英語の文を
  `HostError::Failed` に渡す。ただし `ugoe-mcp` の `autorig` が返す `warnings` (`PsdImportWarning`) と `ApplyReport` の
  `warnings` (`RederiveWarning`) の JSON には `code`・`args` のフィールドが増えた (文は同じ)。
- **画面:** 訳の付け替えはしていない。`src/lib/errors.ts` の `errorText` (いまは `message`) と `errorCode` を足し、
  `String(e)` で IPC のエラーを文字列にしていたところ (22 ファイル) を `errorText` にした。外での書き換えの判定は
  文の部分一致からコード `editor.external_change` にした。`ipc.ts` に `IpcError` (= `Message`) と reject の説明を足した。
- **一覧:** `docs/editor.md#エラーと警告のコード` に全コードと引数を書いた。editor-core と src-tauri のテスト
  (`every_code_is_documented`) が、ソースのコードがすべて一覧にあることを確かめる。
- **確かめたこと:** `just ci`・`cargo check -p ugoe-editor`・`just gen` (差分なし)。画面はビルドした画面をヘッドレス Chrome で
  開き、IPC を偽物 (本物の Document へのブリッジではなく、`open_project` が `Message` で reject するだけのもの) に差し替えて、
  開くのに失敗したときに前と同じ英語の文が出て `[object Object]` が出ないことを確かめた
  (scratchpad の `T0666/shots/03-open-failed.png`。ほかの赤い文は偽物が `null` を返すため)。
- **確かめていないこと:** 本物の Tauri のウィンドウでの表示。本物の Document へのブリッジでのすべてのエラーの経路。
- **後続の候補:** 自由な文の引数 (`reason`・`detail`・`details`) の元のエラー (`ValidationError`・`ComposeError`・
  `InferError`・`MeshError`・`PsdError`・`AiError` の中の文、`EditError` の `reason` を作る箇所) にもコードを付ける。
  `ModuleDifference` の 1 行ずつの英語 (違いの説明) と、wasm の警告 (ビューア・スタジオ) を同じ形にする。
  MCP のツールの結果にもコードを載せるか (文言は変えずに)。
