---
id: T0669
title: コードレビュー: エディタのバックエンド (src-tauri)
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T0669
---

# T0669: コードレビュー: エディタのバックエンド (src-tauri)

## 目的

エディタのバックエンド (src-tauri) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/editor/src-tauri` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - Tauri コマンドに編集のロジックが無いか (`ugoe_editor_core::Command` を通す)
  - ファイルの許可の範囲・パスの扱い、エラーをコードで返しているか (T0666)
  - IPC の型と `apps/editor/src/lib/ipc.ts` の一致、MCP のブリッジ (T0642) の後片付け
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0685〜T0688 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

範囲は `apps/editor/src-tauri` のすべて (`src/` の 22 ファイルとテスト、`Cargo.toml`・`tauri.conf.json`・`tauri.bundle.conf.json`・
`capabilities/`・`build.rs`・`examples/gen.rs`・plist)。IPC の 76 コマンドの名前と引数を `apps/editor/src/lib/ipc.ts` とスクリプトで
突き合わせた。`cargo clippy -p ugoe-editor -- -W clippy::pedantic` は Tauri のコマンドの値渡しと doc の書き方だけ (直していない)。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `src/proposals.rs` (`place`・`propose_normal_maps_in`・`classify_guided_in`) | 提案を作る間 (AI の応答待ち) に別のプロジェクトを開いても、ドキュメントの状態の番号 (開くたびに 0 から数える) だけで比べるので、同じモデルの写しなどでは前のプロジェクトの提案が新しいプロジェクトの一覧に置かれる。分類は何も比べずに新しいプロジェクトに残る | 直した (`ProjectSession::id` を足し、`proposals::Base` でセッションと状態を比べる。テスト `proposals_made_for_another_project_are_not_placed`) |
| 中 | `src/project.rs` (`save_project_as`)・`src/dress_up.rs` | 「Save As…」で別のフォルダーに保存すると、`editor.dress_up` に保存先からの相対パスで残したモジュールが新しい保存先から解かれ、別の (無い) ファイルを指す | 直した (`dress_up::move_to` で新しい保存先から書き直す。失敗したら未保存の印も戻す。テスト `save_as_keeps_the_dress_up_modules`、docs/editor.md) |
| 中 | `src/tracking.rs` (`write_json_file`)・`src/mcp.rs` (`save_claude_desktop_bundle`) | 相対パスを受け付け、プロセスの作業ディレクトリに書く (ほかのパスの引数は絶対パスだけ。docs/editor.md#プロジェクト、T0619) | 直した (`absolute_path`。テストを追加) |
| 中 | `src/project.rs`・`module.rs`・`psd.rs`・`lighting.rs`・`proposals.rs` ほか | 同期のコマンドはメインのスレッドで動く。開く・保存・PSD の取り込み・書き出し・`get_model_bundle`・一定の間隔の `check_external_change` などが重い処理をメインのスレッドで行う (T0814 と同じ形) | T0685 を起票 |
| 中 | `src/lighting.rs` (`generate_texture_normal_map`) | `whole_group` でまとまりのすべての法線マップを、文書のロックを握ったまま作る (提案の `propose_normal_maps` は複製で作ってロックを外している) | T0685 に含めた |
| 低 | `tauri.conf.json`・`build.rs` | CSP が `null`、アプリのコマンドを明示的に許可していない (T0814 と同じ形)。ウィンドウは `main` だけなので、ウィンドウごとの絞り込みは今は効かない | T0686 を起票 (多層の防御) |
| 低 | `src/project.rs` (`export_bundle`)・`module.rs` (`export_module`)・`mcp.rs` (`write_private`)・`ai_provider.rs` | 書き出しと設定のファイルを直接上書きする。途中で失敗すると前の正しいファイルが壊れる (`mcp-bridge.json` ならトークンが消える) | T0687 を起票 |
| 低 | `src/sample.rs` (`open_sample_project`) | 書いたプロジェクトの一覧を添字 `written[0]` で取る | 直した |
| 低 | `src/ai_provider.rs` (`ProviderStore::set`) | ファイルを値の錠の外で書くので、同時に 2 回呼ぶとファイルとメモリの設定が食い違いうる (`spawn_blocking` で動く) | 見送り (画面からは 1 つずつ呼ぶ。T0687 で書き方を変えるときに一緒に直せる) |
| 低 | `src/mcp.rs` (`McpBridge::load`)・`ai_provider.rs` (`ProviderStore::load`) | 設定の JSON が読めないときも `editor.io` のエラー | 見送り (起動時のログに出すだけで、画面には出ない) |
| 低 | `src/module.rs` (`get_preview_base_bundle`) | 覚えた `Arc<[u8]>` を呼ぶたびに `to_vec` で複製する | 見送り (`Response` は `Vec<u8>` を取る。呼ぶのは本体の版が変わったときだけ) |
| 低 | `src/dress_up.rs` (`read_modules`) | `project.json` の相対パスに `..` があっても、そのまま保存先の外を指す | 見送り (絶対パスも書ける形式で、読むのは拡張子 `.ugoemod` だけ、中身は webview の `WebPuppet` が検査する) |
| 確認 | `src/*.rs` | 編集はすべて `Command` を通る (`apply_command`・提案の採用・`generate_texture_normal_map`・`assign_texture_normal_map` は `Command`/`Batch` を `Document::apply` に渡す)。エラーはすべて `Message` (コード付き。`every_code_is_documented` のテストあり) | 問題なし |
| 確認 | `src/lib.rs`・`src/mcp.rs` (MCP のブリッジ、T0642) | 終了時 (`RunEvent::Exit`) に止め、待っている確認を拒否し、発見用のファイルを消す。無効化・ポート・トークンの変更で始め直す。画面の確認の一覧は状態の取り直しで消える。錠の順は `document` → `project` で、`running` の錠と交差しない | 問題なし (強制終了で残る発見用のファイルは ugoe-mcp の側) |
| 確認 | `src/lib/ipc.ts` | 76 コマンドの名前・引数名・戻り値の型が一致 | 問題なし |

### 横断 (T0008 向け)

- **Tauri の同期コマンドはメインのスレッドで動く。** エディタ (T0685)・スタジオ (T0819) の両方にあった。ビューアも確かめるとよい。
- **CSP が `null`** はエディタ・ビューア・スタジオの 3 つのアプリすべて (`apps/*/src-tauri/tauri.conf.json`)。エディタは T0686、スタジオは T0820。ビューアは未起票。
- **開くたびに 0 から数える番号を「変わっていないか」の目印に使う**と、別の文書と取り違える (`Document::state_id`)。時間のかかる処理の結果を戻すところでは、文書やセッションの同一性も比べる。
- **ファイルを直接上書きする**形はエディタのほか `ugoe-mcp` の `host.rs` (`save` の `bundle`) にもある (T0687 の受け入れ条件で editor-core を確かめる)。
- **絶対パスの検査が IPC コマンドごとに手で書かれ**、2 か所で漏れていた。絶対パスだけを受ける型 (Deserialize で検査する) にすれば漏れない。
- テストの一時ディレクトリが `temp_dir()` と pid の手書きで、失敗すると残る (`mcp_tests.rs` の `TempDir` と不揃い)。

