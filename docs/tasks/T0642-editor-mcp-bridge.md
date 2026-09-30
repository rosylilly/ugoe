---
id: T0642
title: エディタ: エディタの中の MCP サーバと、エージェントをつなぐ画面
status: done
milestone: M6
crates: [apps/editor, ugoe-editor-core, docs, Cargo.toml, scripts, justfile, ugoe-mcp]
depends_on: [T0516, T0641]
assignee: claude-worker-T0642
---

# T0642: エディタ: エディタの中の MCP サーバと、エージェントをつなぐ画面

## 目的

ADR 0023 の 1・2・4・6 のエディタ側。実行中のエディタに外部のエージェントがつながり、その提案をエディタの画面で確かめて採用できるようにする。

## 受け入れ条件

- [x] エディタが開いている `Document` を持ち主として、T0516 の API で MCP を待ち受ける (設定で有効にする。既定は無効。ループバック・トークン必須)。発見用のファイルを書き、終了時に消す
- [x] ブリッジ経由の提案が AI タブの提案の一覧に出る (エージェントからと分かる表示)。プレビュー・一部の採用・Undo は既存の流れ (T0637) のまま
- [x] エージェントの `apply_proposal`・`save_project` などは、エディタが確認のダイアログを出し、利用者が承認したときだけ行う。拒否・時間切れはエージェントにエラーで返る。プロジェクトの開閉などはブリッジからはできない
- [x] 設定に「エージェントをつなぐ」の画面: ブリッジの有効化・状態 (つながっているクライアント)・トークンの作り直し、Claude Code・OpenCode・ChatGPT (トンネル)・WSL からの登録のコマンドをコピーできる
- [x] `ugoe-mcp` をサイドカーとしてエディタに同梱する (Windows では `ugoe-mcp.exe`)。WSL から `ugoe-mcp.exe --editor` を登録する手順が画面とマニュアルにある
- [x] 操作マニュアルに章 (`docs/manual/mcp.md`): ファイルを直接扱う形とエディタにつなぐ形の違い、各クライアントの登録、頼み方の例、安全のための設定、Windows と WSL での使い方
- [x] ヘッドレスのブラウザと、テストのクライアント (中継を通す) で、提案 → エディタに出る → 承認 → 適用 → Undo を確かめる
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0023-editor-mcp-bridge.md`
- `docs/tasks/T0516-mcp-document-host.md`・`docs/tasks/T0637-editor-ai-proposals.md`・`docs/tasks/T0638-editor-ai-consent.md` のメモ

## メモ

- やったこと
  - バックエンド `apps/editor/src-tauri/src/mcp.rs`: 持ち主 `EditorHost` (`AppState` の文書と `ProposalSession` の `Proposals` を
    `DocumentHost` として貸す。錠は document → project)、`McpBridge` (設定 `mcp-bridge.json` の有効・ポート・トークン。既定は無効。
    Unix は 0600。起動時に有効なら始め、`RunEvent::Exit` で止めて発見用のファイルを消す)、IPC 6 つ (`mcp_bridge_status`・
    `set_mcp_bridge_enabled`・`set_mcp_bridge_port`・`regenerate_mcp_bridge_token`・`get_mcp_bridge_token`・`answer_mcp_approval`)、
    画面へのイベント `mcp-bridge` (`BridgeEvent`)。
    - 指せるのは開いているプロジェクト (正規化したパスで比べる) だけ。未保存なら「Save As」を促すエラー。作る・`import_psd`・閉じるは
      断り、`open_project` は開いているものなら状態を返すだけ。`instructions` に開いているパス。
    - 適用・Undo・Redo・保存は `Approvals` で確認 (120 秒で時間切れ)。画面から採用するときは確認しない。保存は `ProjectSession::save_to`
      (指紋を取り直す。外で書き換えられていたらエージェントの保存は断る)、書き出しはプロジェクトの書き出しの設定で。
    - 提案の出どころ: `Proposals::set_source`・`ProposalView::source` を ugoe-editor-core に足した (MCP の結果の JSON は変えていない)。
  - 画面 `apps/editor/src/lib/agents/`: 設定の「Connect agents」タブ (有効化・待ち受けの URL・エージェントが使うパス・握手した
    クライアント・登録の手順・ポートとトークン)、確認のダイアログ (モーダルにせず画面の下に。「Show in AI tab」でエージェントが選んだ
    コマンドのままプレビュー・残りの秒数)、ヘッダーの「● Agents」。AI タブの提案に「from <名前>」、タブを開いていなければ「•」。
    登録の手順 (`setup.ts`、Vitest) は同梱の `ugoe-mcp` のパスを埋める: Claude Code・OpenCode・ChatGPT (`tunnel-client` の
    `--mcp-command "<ugoe-mcp> --editor"`。エディタを公開せずに済む)・Windows では WSL (`ugoe-mcp.exe --editor --wsl-distro`、
    ミラーなら Linux 版)・HTTP (トークン)。
  - サイドカー: `src-tauri/tauri.bundle.conf.json` (`externalBin` と `beforeBuildCommand` に `pnpm sidecar`) を `tauri build --config`
    で重ねる。`tauri.conf.json` に書かないのは、tauri-build がファイルの無い `externalBin` で `cargo check`・`tauri dev` も落とすため。
    `scripts/sidecar.mjs` はターゲット (`TAURI_ENV_TARGET_TRIPLE`) 向けに `ugoe-mcp` をリリースでビルドし `binaries/` に置く
    (Linux から Windows は cargo-xwin、universal-apple-darwin は lipo)。
    `just editor-build` と `scripts/editor-windows.mjs` が `--config src-tauri/tauri.bundle.conf.json` を渡す (team-lead の承認)。
    `binaries/` は `src-tauri/.gitignore` に入れた。`just editor`・`just ci`・`cargo check` はサイドカー無しで今までどおり動く。
    開発中は、画面が同じ `target/` の `debug/`・`release/` の `ugoe-mcp` を探し、無ければ `cargo build --release -p ugoe-mcp` を案内する。
  - 文書: `docs/editor.md#エージェントをつなぐ`、操作マニュアル `docs/manual/mcp.md` (画像 2 枚。目次・`screen.md` から辿れる)、
    `docs/ai-integration.md`、`apps/editor/README.md`。
- 確かめたこと
  - `just ci`・`just gen` が通る。`cargo xwin clippy --target x86_64-pc-windows-msvc -p ugoe-editor --all-targets -- -D warnings` が通る。
  - Rust のテスト (`mcp_tests.rs`): 本物の `AppState` でブリッジを始め、`ugoe_mcp::relay::Relay` (中継と同じコード) から、握手・
    提案がエージェントの名前付きで一覧に出る・拒否・時間切れ・承認して適用・エディタの Undo で提案が戻る・Redo・保存と書き出し
    (自分の保存を外の変更と取り違えない)・外での書き換えの後の保存を断る・止めると発見用のファイルが消える・ほかのプロジェクト /
    作る / 閉じる / import_psd を断る・未保存・設定とトークン (0600・作り直し)。
  - サイドカー: `pnpm sidecar` で `binaries/ugoe-mcp-x86_64-unknown-linux-gnu` ができ、`TAURI_CONFIG` に重ねる設定を渡した
    `cargo check -p ugoe-editor` で tauri-build が受け入れて実行ファイルの隣に写すことを確かめた。
  - **実際に束ねた (Linux だけ):** `pnpm tauri build --config src-tauri/tauri.bundle.conf.json --bundles deb` (just editor-build と同じ
    設定) で、重ねた `beforeBuildCommand` が `pnpm sidecar` を実行し (Tauri が渡すターゲットの三つ組で)、`.deb` に
    `usr/bin/ugoe-editor` と `usr/bin/ugoe-mcp` が入ることを確かめた。Windows (NSIS・MSI)・macOS (universal の lipo) は束ねていない。
  - ヘッドレス Chrome (ビルドした画面 + scratchpad の HTTP ブリッジ。エディタのバックエンドの `ai`・`ai_provider`・`mcp`・`project`・
    `proposals`・`psd` のモジュールを `#[path]` でそのまま取り込み、本物の `AppState`・`McpBridge`・`EditorBridge` を動かす。Tauri の
    イベントはブリッジから取りに行って配る) と、本物の `ugoe-mcp --editor` (子プロセス) を stdio で動かすテストのクライアントで:
    無効のときの「not running」→ Connect agents で有効 → 握手でクライアントが出る → `propose_edit` で AI タブに「AI (1) •」と
    「from claude-code」→ `apply_proposal` で確認 → Decline でエラー → もう一度 → 「Show in AI tab」でプレビュー → Apply で
    適用 (パラメータとデフォーマが増える) → ヘッダーの Undo で提案が一覧に戻る → Redo → `save_project` の確認 → Save で保存
    (未保存の印が消える) → 作るは断られる → 無効にすると「not running」。Windows のエディタのときの WSL の手順の表示も
    (状態を差し替えて) 確かめた。
- 確かめていないこと: 実際の Tauri のウィンドウ (イベントの届き方・`navigator.clipboard`・終了時の片付け)、Windows と macOS の束ねと
  インストール後のサイドカーの場所 (NSIS・.app。`.deb` の中身は確かめた)、Claude Code・OpenCode・ChatGPT (tunnel-client)
  からの実際の登録と接続、Windows での動作と WSL からの `ugoe-mcp.exe` (T0516 で中継だけは実機で確かめ済み)。
- 範囲の外だったもの (team-lead の承認済み。crates: に足した): ルートの `Cargo.toml` (`ugoe-mcp` を workspace の依存に)・`Cargo.lock`・
  `scripts/check-layers.mjs` (ugoe-editor → ugoe-mcp を許す。ADR 0023 でエディタが組み込むため)・`justfile` の `editor-build` と
  `scripts/editor-windows.mjs` (サイドカーの設定を重ねる)・`crates/ugoe-mcp/README.md` (WSL の例のインストール先と「エディタに組み込む」を
  実装に合わせた。README だけ)・`docs/architecture.md` の層の表。
- T0516 からの申し送り (保存はエディタの `save_to` を通す) は満たしている (`EditorHost::save`。テストで指紋が更新されることを確かめた)。
- 後続の候補
  - 中継が `instructions` の中のパスも WSL のパスに変える (今は Windows のパスのまま伝わる。そのまま渡しても動く)。
  - クライアントの区別 (セッションか中継のヘッダ)。未保存のプロジェクトを指す仮のパス (T0516 の候補と同じ)。
  - 確認のダイアログに差分 (追加・変更される要素) を出す。エージェントの Undo のときに取り消す編集の名前を出す (Undo の履歴のラベル)。
  - AppImage では同梱の `ugoe-mcp` の場所が起動のたびに変わるので、登録のコマンドにそう出す・PATH へ入れる手順。
