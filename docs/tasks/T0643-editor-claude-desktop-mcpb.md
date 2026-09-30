---
id: T0643
title: エディタ: Claude Desktop 用の MCP Bundle (.mcpb)
status: done
milestone: M6
crates: [apps/editor, docs, ugoe-mcp, Cargo.lock]
depends_on: [T0642]
assignee: claude-worker-T0643
---

# T0643: エディタ: Claude Desktop 用の MCP Bundle (.mcpb)

## 目的

Claude Desktop から、実行中のエディタにつなげるようにする (ADR 0023 の続き。利用者の要望)。
`ugoe-mcp --editor` の中継を MCP Bundle (`.mcpb`) に包み、エディタの「Connect agents」から Claude Desktop に入れられるようにする。

## 受け入れ条件

- [x] MCPB の最新の仕様 (manifest の版・`server` の `type: binary`・`entry_point`・`mcp_config`・`compatibility` の OS・`tools` の宣言・`user_config` など) を公式の文書で確かめ、`ugoe-mcp --editor` を包む `manifest.json` を作る。ツールの一覧は `ugoe-mcp` の `tools/list` から作り、手で写さない
- [x] `.mcpb` の作り方: エディタが同梱の `ugoe-mcp` から、その OS の `.mcpb` をその場で作る (インストーラには別に入れない。team-lead の承認で変更。理由はメモ)。公式の `mcpb` の CLI の validate・info を通す。開発モードでは `target/` の `ugoe-mcp` から作れる
- [x] エディタの「Connect agents」に Claude Desktop の手順: `.mcpb` を開いて Claude Desktop に入れるボタン (または保存)、入れた後の確かめ方。エディタが Windows で動くときは Windows 版の `ugoe-mcp.exe` を包む
- [x] 操作マニュアル (`docs/manual/mcp.md`) に Claude Desktop の節
- [x] `.mcpb` の中身 (manifest の検証と、中の `ugoe-mcp --editor` が起動して偽のエディタにつながること) を確かめるテスト
- [x] `just ci`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0023-editor-mcp-bridge.md`
- `docs/tasks/T0642-editor-mcp-bridge.md`・`docs/tasks/T0516-mcp-document-host.md` のメモ

## メモ

- **仕様 (2026-09-30 に確かめた):** [MANIFEST.md](https://github.com/modelcontextprotocol/mcpb/blob/main/MANIFEST.md)・
  [manifest 0.3 の JSON Schema](https://github.com/modelcontextprotocol/mcpb/blob/main/schemas/mcpb-manifest-v0.3.schema.json)・
  公式の CLI のソース (`src/node/validate.ts`・`src/cli/pack.ts`・`src/shared/config.ts`・`src/shared/constants.ts`)・
  [Build a desktop extension with MCPB](https://claude.com/docs/connectors/building/mcpb)・
  [Getting Started with Local MCP Servers on Claude Desktop](https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop)・
  [Install Claude Desktop](https://support.claude.com/en/articles/10065433-install-claude-desktop)。
  - `manifest_version` は 0.3 (CLI の既定。`latest` の schema も 0.3。0.4 は `uv` を足しただけ)。`server.type` `binary`、`entry_point` は
    束の中の実行ファイル (validate が存在と Unix の実行権を確かめる)、`mcp_config` の変数は `${__dirname}`・`${/}`・`${pathSeparator}`・
    `${user_config.*}` だけ (`${HOME}` は `user_config` の既定値でしか使えない)。`tools` の項目は `name`・`description` だけ。
    `compatibility.platforms` は `darwin`・`win32`・`linux`。公式の `pack` は Unix で作るときに mode を外部属性に入れ、`binary` の
    `entry_point` には必ず実行権を付ける (Claude Desktop がそのまま起動するため)。署名は PKCS#7 を zip の後ろに足す形 (していない)。
  - **Claude Desktop の OS:** macOS・Windows に加え、Linux もベータで公式にある (2026-06-30 から。Ubuntu 22.04+・Debian 12+、x64・arm64。
    ヘルプに「拡張機能と MCP サーバは入れられる」)。claude.com の MCPB の文書は「macOS と Windows」のままで古い。3 つとも束を作れる。
  - 入れ方: `.mcpb` のダブルクリック・ウィンドウへのドロップ・「Settings > Extensions > Advanced settings > Install Extension…」。
    確かめ方: 「Settings > Extensions」、チャットの「+ > Connectors」にツール。
- **やったこと**
  - `ugoe_mcp::mcpb` (MIT): `Bundle` (実行ファイル・OS・アイコン・版) が `manifest.json` を作り (`tools` は `Tools::list` (エディタの
    ブリッジと同じ既定の設定) の `name` と英語の `title`。手で写さない)、zip を書く。zip は自前の小さな書き手 (deflate は既存の
    `miniz_oxide`、CRC-32 は自前。時刻は固定で同じ入力から同じバイト列)。実行ファイルは 0755、Windows は `server/ugoe-mcp.exe` と
    `${__dirname}/server/ugoe-mcp.exe` (仕様の `platform_overrides` の例と同じく `.exe` まで書く)。`ugoe-mcp --mcpb <OUT> [--mcpb-icon <PNG>]`
    が自分を包む (開発者・リリースの作業・公式の CLI での確認用)。
  - エディタ: 「Connect agents」に「Claude Desktop」のタブ。「Install in Claude Desktop…」(同梱の `ugoe-mcp` をその場で包み、アプリの
    キャッシュに書いて tauri-plugin-opener で OS に開かせる)・「Save .mcpb…」(保存のダイアログ)・入れた後の確かめ方・更新したら入れ直す
    こと・`claude_desktop_config.json` に書く代わりの JSON。ブリッジが無効なら先に有効にするよう、エディタが WSL の中なら Windows の
    Claude Desktop から届かないと出す。IPC `open_claude_desktop_bundle`・`save_claude_desktop_bundle` (非同期。`spawn_blocking`)。
    アイコンはエディタの `icons/icon.png` (512×512)。
  - **束はインストーラに入れず、その場で作る** (受け入れ条件の「配布用のビルドで OS ごとの .mcpb を作って同梱する」から変えた。
    team-lead の承認): 同梱の `ugoe-mcp` を包むだけなので、インストーラに同じ実行ファイルを 2 つ入れずに済み、束の OS と版が
    エディタと必ずそろい、開発中も `target/` の `ugoe-mcp` から同じように作れる。`tauri.bundle.conf.json`・`sidecar.mjs` は変えていない。
    Linux の AppImage でも、中の `ugoe-mcp` を複製するので起動のたびに場所が変わる問題が起きない。
  - zip: 実行ファイルの項目は外部属性に Unix の mode (`S_IFREG | 0755`、作った OS は Unix)、ローカルと中央のヘッダ、データ記述子なし、
    名前は UTF-8 (汎用のフラグのビット 11)。テストはこれらと、展開した実行ファイルが実行できることを確かめる。
  - 署名: `.mcpb` に `mcpb sign` はしていない。中の `ugoe-mcp` はエディタの同梱のものの複製で、コード署名も無い (インストーラと同じ)。
    手元で作ったファイルには隔離属性・Mark of the Web が付かないが、配ったりダウンロードしたりすると macOS の Gatekeeper・Windows の
    SmartScreen やウイルス対策で止まりうることをマニュアルに書いた。
  - 文書: 操作マニュアル (`docs/manual/mcp.md#claude-desktop`、画像 `images/mcp-claude-desktop.png`)、`docs/editor.md#エージェントをつなぐ`、
    `docs/ai-integration.md`、`crates/ugoe-mcp/README.md` (`--mcpb` と束の形)。
- **確かめたこと**
  - `just ci` が通る。`cargo xwin clippy --target x86_64-pc-windows-msvc -p ugoe-mcp --tests -- -D warnings` が通る。
  - テスト: `crates/ugoe-mcp/tests/editor.rs` の `the_mcp_bundle_relays_to_a_running_editor` (`--mcpb` で作り、展開して manifest を
    0.3 の schema と validate の規則で確かめ、実行ファイルが同じバイト列で 0755、manifest のコマンドを `${__dirname}` を置き換えて起動し、
    偽のエディタ (`FakeEditor`) につながって、`tools/list` が manifest の `tools` と同じ・提案が届く)。`src/mcpb.rs` の単体テスト (CRC・
    OS ごとの manifest・ツールの一覧・PNG でないアイコン)。エディタの `mcp_tests.rs` (同梱の `ugoe-mcp` から作る・無いときの案内)。Vitest。
  - 公式の CLI (`npx @anthropic-ai/mcpb@2.1.2`) の `info`・`unpack`・`validate` が、Linux の束と Windows の束 (cargo-xwin で作った
    `ugoe-mcp.exe` を WSL の相互運用で動かして `--mcpb`) の両方を通す。Windows の束から展開した `ugoe-mcp.exe --editor` を相互運用で
    起動し、WSL の中のエディタ (下のブリッジ) につながることも確かめた (ミラーのネットワーク)。
  - ヘッドレス Chrome (ビルドした画面 + 本物の `AppState`・`McpBridge` の使い捨ての HTTP ブリッジ。束を作る IPC はブリッジが
    `ugoe_mcp::mcpb` で書くだけ): 無効のときの案内 → 有効 → 「Install in Claude Desktop…」(作っている間の表示 → 開いた後の案内) →
    「Save .mcpb…」(ダイアログの既定の名前と拡張子) → 保存した束を展開して manifest のコマンドを起動し、`claude-ai` として握手すると
    「Connected agents」に出る。
- **確かめていないこと**
  - **実際の Claude Desktop に入れること** (開発した環境が WSL で、Claude Desktop が無い)。インストールの画面の見え方・署名の無い束の
    扱い・Claude Desktop が `.exe` の付いた `command` をそのまま使うか (仕様の例に合わせた)・渡す環境変数 (Linux で `XDG_STATE_HOME` を
    変えている利用者では、エディタと中継が別の場所を見るおそれ)・Claude Desktop のクライアントの名前 (`claude-ai` は仮定)。
  - 実際の Tauri のウィンドウでの `open_path` (OS が `.mcpb` を Claude Desktop に渡すか。Linux で zip として別のアプリが開くときの案内は
    書いた)、macOS の署名・公証の付いたサイドカーを束に写したときの Gatekeeper の扱い。
- **範囲外の変更 (team-lead の承認。crates: に足した):** 束を作るコードは MIT の `ugoe-mcp` に置く (エディタと `--mcpb` の両方で使い、
  テストで中の中継を偽のエディタにつなぐため)。 `crates/ugoe-mcp` (`src/mcpb.rs`・`--mcpb`・テスト・README) と `Cargo.lock` (ugoe-mcp が
  `miniz_oxide` を使う 1 行)。ルートの `Cargo.toml` は変えていない (新しい外部の依存は無い)。
- **後続の候補**
  - 実機の Claude Desktop (macOS・Windows・Linux) で入れて確かめる。必要なら `compatibility.claude_desktop` の下限を書く。
  - 束の署名 (`mcpb sign`。リリースの証明書がそろったら)。
  - エディタを更新したときに、入れた束が古いことを知らせる (中継が版の違いを警告するだけなので)。
