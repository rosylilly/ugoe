---
id: T0551
title: "MCP: --editor の中継で要求を並行に送り、取り消しを伝える"
status: done
milestone: M9
crates: [ugoe-mcp]
depends_on: [T0529]
assignee: claude-worker-T0551
---

# T0551: MCP: --editor の中継で要求を並行に送り、取り消しを伝える

## 目的

`ugoe-mcp --editor` の中継 (`src/relay.rs`) は標準入力を 1 行ずつ読み、エディタの答えを待ってから次の行を読む。
`apply_proposal`・`save_project` などはエディタの確認のダイアログを待つ (答えの時間は限らない) ので、その間は
同じクライアントのほかの要求 (`ping`・読むだけのツール) も、`notifications/cancelled` も届かない。MCP クライアントが
要求を並行に送る・時間切れで取り消す場合に、中継が止まって見える (T0529 のレビューで見つけた)。

stdio の MCP は要求を並行に送ってよく、応答の順も問わない。中継で要求ごとにエディタへの送信を並行にし、
`notifications/cancelled` を受けたら待っている確認を断れるようにする (エディタの側は `Approvals` で確認を拒否する口が要るか
を決める。HTTP の Streamable HTTP の取り消しの扱いも確かめる)。

## 受け入れ条件

- [x] 中継が、確認を待つ要求の間もほかの要求に答える (テスト: 偽のエディタで確認を待たせ、同じ中継で `ping` と `list_proposals` が返る)
- [x] 標準出力への書き込みは 1 行ずつ不可分 (並行の応答が混ざらない)
- [x] `notifications/cancelled` の扱いを決め、README に書いた (伝えるなら、確認を待つ適用が拒否で終わることのテスト)
- [x] `just ci` が通る

## 参考

- `crates/ugoe-mcp/src/relay.rs`・`crates/ugoe-mcp/src/host.rs` (`Approvals`)
- ADR 0023、`docs/ai-integration.md#実行中のエディタにつなぐ-adr-0023`
- MCP の仕様の取り消し (`notifications/cancelled`)

## メモ

- T0529 のレビューで起票した。
- やったこと:
  - 中継 (`src/relay.rs`): 要求ごとにスレッドでエディタへ送り、答えは錠の中で 1 行ずつ書く。エディタへは id を
    中継だけの id (`ugoe-relay-<乱数>-<番号>`) に変えて送り、答えで戻す (エディタの HTTP はセッションを持たず、
    別の中継も `1` などの同じ id を使うので)。`notifications/cancelled` は処理中の要求なら id を変えてエディタへ送り、
    その要求の答えは書かない。知らない id・終わった要求の取り消しは送らない。標準入力が閉じたら処理中の要求を
    すべて取り消してから終わる。`Relay` は `&self` で動く (`run` の出力は `Write + Send`。`main.rs` は `stdout()`)。
  - サーバ (`src/server.rs`): 処理中の要求の取り消しの印を id ごとに持つ。`notifications/cancelled` は、その id の
    処理中の要求が 1 つだけのときに取り消す (HTTP で複数のクライアントが同じ id なら無視)。
  - HTTP (`src/http.rs`): 答えを返す前にクライアントが接続を閉じたら取り消す (2026-07-28 の Streamable HTTP の
    取り消し。旧来の版は切断を取り消しとみなさないよう勧めるが、SSE で送り直さないので答えは届かない。README に理由)。
    接続は `peek` で覗く (確認を待つスレッドが 50ms ごと)。
  - `Approvals::request` (`src/host.rs`): 要求のスレッドの取り消しの印を見て、確認を拒否で終える (エディタの
    `EditorHost::confirm` は `request` から戻ったら画面の確認を閉じるので、エディタの変更は要らない)。
    `Approvals` に新しい公開の口は足していない。
  - テスト: `tests/editor.rs` に、同じ中継で確認を待つ間に `ping`・`list_proposals`・並行の 8 要求が返る (行が混ざらない)、
    `notifications/cancelled` で確認が閉じ適用されず答えも書かない、標準入力を閉じると確認が閉じて中継が終わる、
    HTTP の接続を閉じると確認が閉じる、を足した (いずれも変更前に失敗を確かめた)。`server.rs`・`host.rs` に単体テスト。
  - `crates:` の外: `docs/ai-integration.md` の中継の節と `docs/manual/mcp.md` の確認の節に 1〜2 行ずつ足した。
- やらなかったこと・制限:
  - 中継が要求を送った直後 (エディタに届く前) に取り消しが届くと、取り消しは伝わらない (確認は出る)。送る前なら送らない。
  - 取り消しで止まるのは確認だけ (ほかのツールはすぐ終わるので取り消さない。描画なども途中では止めない)。
  - 実際のエディタ (Tauri) と実際の MCP クライアント (Claude Code の時間切れ・Esc) では確かめていない (偽のエディタと
    子プロセスの中継で確かめた)。Windows の `peek` の振る舞いも未確認。
- 起票の候補:
  - エディタの確認のダイアログに「エージェントが取り消した」と出す (今は時間切れ・拒否と同じく消えるだけ)。
  - 長い描画 (`render_png`) や自動リギングの途中の取り消し (必要が出たら)。
