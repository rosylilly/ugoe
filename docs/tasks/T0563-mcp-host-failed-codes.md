---
id: T0563
title: "ugoe-mcp: 持ち主の失敗 (HostError::Failed) と、コードの無いツールのエラーにもコードを付ける (判断が要る)"
status: done
milestone: M9
crates: [ugoe-mcp, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0563
---

# T0563: ugoe-mcp: 持ち主の失敗 (HostError::Failed) と、コードの無いツールのエラーにもコードを付ける (判断が要る)

## 目的

T0557 で MCP のツールのエラーに構造 (`structuredContent` の `code`・`args`) を付けた。次はまだコードが無い (T0557 の候補)。

1. `HostError::Failed(String)` (`crates/ugoe-mcp/src/host.rs`) は文だけを持つので、エディタの持ち主の失敗 (保存の失敗 `project.*` など) のコードが MCP の結果に載らない。
   `Failed` にコード (`Message`) を持たせる。`HostError` は ugoe-mcp の公開の型で、エディタ (`apps/editor`) が持ち主を実装している。
2. **判断が要る:** ほかのコードの無いツールのエラー (引数の誤り・パスの誤り・`--root` の外・要素が無いなど) に MCP のコード (`mcp.invalid_arguments` など) を付けるか。
   付けるなら、コードの名前の付け方と ADR 0029 の訳の表 (`codes.json`) に入れるか。

## 受け入れ条件

- [x] `HostError::Failed` がコードと引数を持ち、エディタの持ち主の保存などの失敗のコードが MCP の結果の `structuredContent` に載る。そのテストがある
- [x] コードの無いツールのエラーにコードを付けるかの案と推奨をメモに書き、利用者の判断を得た。付けるなら、ツールのエラーの一覧と crates/ugoe-mcp/README.md・docs を揃えた
- [x] `just ci` が通る

## 参考

- T0557 のメモ (起票の候補)
- ADR 0029、crates/ugoe-mcp/README.md

## メモ

- T0032 で起票した (元の候補: T0557 (2 つ))。
- 判断 (2026-10-05、利用者の判断): コードの無いツールのエラーに MCP だけのコード (`mcp.invalid_arguments` など、`mcp.*`) を付け、ugoe-mcp の README の表に載せる。wasm の codes.json とエディタの訳の表には入れない。

### やったこと (T0563)

- **`HostError::Failed(Box<Message>)`:** 文だけでなくコード・引数を持つ (`From<Message>`・`HostError::failed(文, Message)`)。
  `From<String>` は外した (コードの無い失敗を作れないように)。持ち主の `.ugoe` の書き出しの失敗は、`FileHost` とエディタで
  共有する関数 (`host::export_failed`・`export_write_failed`・`export_replace_failed`) で作る (文は前と同じ)。
- **ツールのエラー:** `ToolError` を `Box<Message>` だけにし、`From<String>` を外した。すべてのツールのエラーが
  `structuredContent` にコードを持つ (`message` は `content` の文と同じ。文は変えていない)。
  - エディタのコードがあるものはそれを使う: プロジェクトの読み込み・保存 (`project.*`)、`.ugoe` の書き出し (`export.*`・`project.*`)、
    PSD のファイルを読めない (`psd.read_file_failed`。エディタと同じ `path`・`detail`)。
  - それ以外は MCP だけのコード `mcp.*` (判断どおり)。一覧と `args` は crates/ugoe-mcp/README.md の「ツールのエラー」。
    エディタの画面の既存の `mcp.*` (ブリッジ・`.mcpb`) と名前が重ならないようにした (`.ugoe` の書き出しは `mcp.export_*`。
    エディタの `mcp.bundle_write_failed` は `.mcpb` のもの)。wasm の `codes.json`・エディタの訳の表には入れていない
    (docs/editor.md の表の後に、入れないことと README への参照を書いた)。
  - `--root` の判定 (`root.rs`) は `Message` (`mcp.invalid_path`・`mcp.outside_root`) を返す。描画 (`RenderError`) と中継
    (`RelayError`) は `Coded` を実装した (書き出し・読み戻しの失敗は `cause` に `project.*`・`format.*`)。
  - `--editor` の WSL の中継は、エラーの `structuredContent` の `message` (と `cause`・`items` の文) のパスも `content` の文と同じく戻す
    (`message` と文が食い違わないように)。
- **エディタ (`apps/editor`):** 持ち主の失敗はエディタの `Message` をそのまま渡す (錠の壊れは `editor.lock_poisoned`)。
- テスト: `tests/stdio.rs` (エラーを返す呼び出しはすべて、`structuredContent` のコードがあり `message` が文と同じことを確かめる。
  `mcp.invalid_arguments`・`mcp.read_only`・`mcp.outside_root`・`mcp.unsaved_changes`・`mcp.export_write_failed`・`mcp.project_exists`・
  `mcp.element_not_found`・`mcp.unknown_kind`・`mcp.unknown_name`・`mcp.invalid_render`・自動リギングの `mcp.*`・`project.base_is_module`)、
  `host.rs` (`FileHost` の書き出しの失敗のコードと `args.path`、`src/` の `"mcp.…"` と README の表が一致すること)、`relay.rs`
  (中継のエラーのコード、WSL での `message` の戻し)、エディタの `mcp_tests.rs` (エディタの保存が書く途中で失敗すると
  `project.io` と `path`・`detail` が載る、`.ugoe` の書き出しの失敗の `mcp.export_write_failed`・`mcp.export_replace_failed`)。

### やらなかったこと

- MCP の `mcp.*` の英語の公開仕様 (`spec/`) への記載 (MCP サーバは公開仕様の範囲の外)。
- JSON-RPC のエラー (ツールの呼び出し以外・panic の `-32603`) はそのまま (ツールのエラーではないので)。

### 起票の候補

- `ToolError` の文が `(code: …)` を末尾に付ける PSD の上限のエラー (T0554) は、構造でコードを返すようになったので、文の末尾の
  `(code: …)` を外してよいか (文を変えるので利用者の判断)。

