# ADR (Architecture Decision Records)

設計上の重要な判断と、その理由を記録します。

- ファイル名は `NNNN-kebab-case-title.md`。番号は連番で、既存の最大番号 + 1 を使います。
- 雛形は [0000-template.md](0000-template.md) です。
- 状態: `Proposed` → `Accepted` → (`Superseded by NNNN` | `Deprecated`)
- `Accepted` になった ADR の本文は書き換えません (誤字の修正は可)。判断を変えるときは新しい ADR を書きます。
- 並行して作業している複数のエージェントが同じ番号を取った場合は、後からマージする側が番号を振り直します。

| 番号 | タイトル | 状態 |
| --- | --- | --- |
| [0001](0001-technology-stack.md) | 技術スタック (Rust / wgpu / Tauri / Svelte) | Accepted |
| [0002](0002-cpu-deformation-gpu-rendering.md) | 変形は CPU、描画は GPU | Accepted |
| [0003](0003-editor-viewport-in-webview.md) | エディタのビューポートは WebView 内の wasm で描画する | Accepted |
| [0004](0004-file-formats.md) | 独自ファイル形式 (プロジェクト JSON とバンドル `.ugoe`) | Accepted |
| [0005](0005-editor-command-model.md) | 編集操作はすべてシリアライズ可能な Command を通す | Accepted |
| [0006](0006-ai-proposals.md) | AI は提案だけを行い、適用は人間が承認する | Accepted |
| [0007](0007-multi-agent-workflow.md) | ファイルベースのタスクボードと worktree による並行開発 | Accepted |
| [0009](0009-clipping-mask-semantics.md) | クリッピングマスクの意味論 | Accepted |
| [0008](0008-bundle-payload-encoding.md) | バンドルのペイロードは MessagePack (フィールド名付き) と zstd | Accepted |
