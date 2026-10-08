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
| [0008](0008-bundle-payload-encoding.md) | バンドルのペイロードは MessagePack (フィールド名付き) と zstd | Accepted |
| [0009](0009-clipping-mask-semantics.md) | クリッピングマスクの意味論 | Accepted |
| [0010](0010-keyform-inference.md) | キーフォームの推定 (組み合わせ爆発の解消) | Accepted |
| [0011](0011-dynamic-physics.md) | 揺れ用パラメータのいらない物理 (動的物理) | Accepted |
| [0012](0012-avatar-profile.md) | 2D アバターの標準プロファイル | Accepted |
| [0013](0013-forward-compatibility.md) | 前方互換と段階的な劣化 | Accepted |
| [0014](0014-composable-modules.md) | 着せ替え (実行時のモジュール合成) | Accepted |
| [0015](0015-lighting.md) | ライティング (法線マップ) | Accepted |
| [0016](0016-draw-batching.md) | 描画コストの自動最適化 | Accepted |
| [0017](0017-single-runtime-distribution.md) | 単一ランタイムと 1 行導入 | Accepted |
| [0018](0018-licensing-and-1.0-scope.md) | ライセンスと 1.0 の範囲 | 一部 Superseded (1.0 の範囲は 0020) |
| [0019](0019-viewer-license.md) | ビューア (apps/viewer) のライセンスは GPL-3.0-only | Accepted |
| [0020](0020-milestone-order-and-ai-in-1.0.md) | マイルストーンの順序と、AI 支援を 1.0 に含めること | Accepted |
| [0021](0021-public-specification.md) | 公開のモデル仕様 (英語版を正とし、CC BY 4.0) | Accepted |
| [0022](0022-multiple-ai-providers-and-agents.md) | 複数の AI プロバイダと、エージェントに判断を任せる経路 | Accepted |
| [0023](0023-editor-mcp-bridge.md) | 実行中のエディタと MCP をつなぐ (エディタの中の MCP サーバと、stdio の中継) | Accepted |
| [0024](0024-module-authoring.md) | 着せ替えのモジュールは、本体を参照する別のプロジェクトとして作る | Accepted |
| [0025](0025-web-runtime-package.md) | Web のランタイムのパッケージ (@ugoe/runtime) は MIT で bindings/web に置く | Accepted |
| [0026](0026-godot-binding.md) | Godot のバインディング (godot-rust) をワークスペースの外に置く | Accepted |
| [0027](0027-crates-io-publishing.md) | crates.io に公開するクレートの範囲 (`ugoe` とその依存) と公開の方法 | Accepted |
| [0028](0028-studio.md) | 配信用のアプリ「スタジオ」の範囲・構成・描画の場所・外部 API の安全性 | 一部 Superseded (プロファイルのパスは 0033、`unsafe` の例外は 0034) |
| [0029](0029-shared-ui-and-i18n.md) | 共有の UI 部品と翻訳 (@ugoe/ui・@ugoe/i18n・@ugoe/player)。翻訳は i18next、部品の土台は Bits UI | Accepted |
| [0030](0030-compressed-ktx2-decoding.md) | 参照の実装と wasm で圧縮した KTX2 をデコードする (BCn を bcdec_rs で。ETC2・ASTC・Basis は後で) | Accepted |
| [0031](0031-no-std-runtime-and-host-neutral-c-abi.md) | ランタイム層を no_std + alloc で使えるようにし (数学関数は常に libm)、C ABI のエラーをスレッドローカルから外し、毎フレームの状態はまとめて渡す | Accepted |
| [0032](0032-fuzzing-with-cargo-fuzz.md) | 外から来る入力を cargo-fuzz (libFuzzer、nightly) で続けて fuzz する。ワークスペースの外に置き、`just ci` に入れない | Accepted |
| [0033](0033-studio-profile-relative-paths.md) | スタジオのプロファイルは、プロファイルのフォルダの下のファイルを相対パスで持つ (`..` を含む相対パスは読まない) | Accepted |
| [0034](0034-studio-gpu-priority.md) | スタジオの GPU の優先度を、`unsafe` を許す専用のクレート (`ugoe-studio-gpu-priority`) で Windows でだけ下げる | Accepted |
