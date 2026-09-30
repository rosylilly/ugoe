---
id: T0514
title: MCP: HTTP のトランスポート
status: done
milestone: M6
crates: [ugoe-mcp, docs, Cargo.toml]
depends_on: [T0513]
assignee: claude-worker-T0514
---

# T0514: MCP: HTTP のトランスポート

## 目的

ChatGPT のように HTTP で MCP サーバにつなぐクライアントのために、`ugoe-mcp` に stdio に加えて HTTP (MCP の Streamable HTTP) のトランスポートを足す (ADR 0022)。

## 受け入れ条件

- [x] `ugoe-mcp --http <addr>` で HTTP のトランスポートで待ち受ける。既定はループバックだけに束ねる
- [x] 認証 (トークンなど) と、外部に公開するときの注意 (トンネルを使う場合など) を README に書く。認証なしで外部に公開できないようにする
- [x] 新しい外部の依存を入れるなら、ライセンス (`just licenses`) と重さを確かめ、理由を書く
- [x] HTTP で一通りのツールを呼ぶテスト。ChatGPT から使う手順を書く (実際の接続の確認は利用者に残してよい)
- [x] `just ci` が通る

## 参考

- `docs/adr/0022-multiple-ai-providers-and-agents.md`
- https://modelcontextprotocol.io/

## メモ

- やったこと
  - `ugoe-mcp --http <addr>` (`src/http.rs`)。MCP の Streamable HTTP のエンドポイント `/mcp` で、要求を 1 つずつ POST で
    受け、応答は JSON 1 つで返す (SSE は使わない。ツールは途中の通知を送らないので)。通知は `202`、GET・DELETE は `405`、
    セッションは発行しない (状態はプロセスに 1 つで共有。stdio と同じ `Server` をそのまま使う。stdio の挙動は変えていない)。
  - 版: 要求ごとに `_meta` で版を示す版 (2026-07-28) では `MCP-Protocol-Version`・`Mcp-Method`・`Mcp-Name` と本文の一致を
    確かめる (違えば `400` と -32020。`=?base64?..?=` も解く)。知らないメソッドは `404`。握手をする版 (2025-11-25 以前) も
    受け、ヘッダが無ければ 2025-03-26 とみなす。仕様は modelcontextprotocol.io の draft と 2025-11-25 の transports を見た。
  - 守り: 既定はループバック (`--http 8765` は `127.0.0.1:8765`)。ループバックの外に束ねるにはトークンが要る (無ければ起動
    しない)。トークン (`--token` か `UGOE_MCP_TOKEN`。16 文字以上) は `Authorization: Bearer` か、ChatGPT のようにヘッダを
    送れないクライアントのためにパス `/mcp/<token>` で受ける (定数時間の比較)。トークンが無ければ、`Host` がループバックの
    名前で転送ヘッダ (`X-Forwarded-For` など) の無い要求だけを受ける (ngrok などで認証なしに公開されないように)。
    `Origin` はループバックか `--allow-origin` のものだけ (DNS リバインディング対策。違えば `403`)。
  - 依存: HTTP/1.1 は std の `TcpListener` の上に書いた (接続ごとのスレッド・keep-alive・`Content-Length` と `chunked`・
    `Expect: 100-continue`)。承認された httparse の案よりさらに小さく、HTTP の解析のクレートも使っていない。
    トークンの生成に `getrandom` 0.3 (MIT/Apache-2.0、ring などで推移的に入っていたもの) をルートの `Cargo.toml` の
    `[workspace.dependencies]` に足した (team-lead の承認。このため crates: に Cargo.toml を足した)。
    テストのクライアントに、ワークスペースの `ureq` を dev-dependency で使う。
  - 上限 (team-lead の条件): 本文 32 MiB (`413`)・ヘッダの合計 64 KiB と 100 個 (`431`)・1 つの要求を読み終えるまで 60 秒
    (keep-alive の待ちを含む。slowloris も切る。`408`)・同時の接続 32 (`503`)。書き込みにも同じ時間の上限。
    それぞれ上限を小さくしたサーバをソケットで叩く単体テストがある。
  - トークン: 32 文字 (16 進数で 128 ビット) 以上を求める。`--generate-token` で OS の乱数から 256 ビットを作って標準エラーに
    書く。比較は定数時間。URL に入れる形はログに残りやすいことを README に書いた。
  - テスト: `src/http.rs` の単体テスト (ヘッダの検査・守り・要求の読み取り) と `tests/http.rs` (サーバを `--http` で起動し、
    initialize → tools/list → tools/call の提案・適用・保存、版を示すクライアントとの状態の共有、生の HTTP での keep-alive・
    405・Host/Origin/転送ヘッダ、トークン、ループバックの外での起動の拒否)。curl でも手で確かめた。
  - 文書: `crates/ugoe-mcp/README.md` に HTTP の動かし方・守り・ChatGPT から使う手順 (OpenAI の Secure MCP Tunnel を勧める。
    公開のトンネルではトークンを URL のパスに入れる)、`docs/ai-integration.md` に設計の要点。
- 確かめていないこと
  - 実際の ChatGPT (開発者モード・Secure MCP Tunnel・ngrok) からつなぐこと。ChatGPT が `Origin` を付けるか、tunnel-client が
    転送ヘッダを付けるかは分からない (付けるなら `--allow-origin`・トークンで回避する手順を README に書いた)。
  - `tunnel-client` を HTTP のサーバにつなぐときの `--sample` の名前 (OpenAI の文書に stdio の例しかない)。
- 後続の候補
  - ツールが読み書きできるパスを 1 つのディレクトリの下に限る (`--root`。T0515 で起票済み)。HTTP で外に出すと、利用者の権限で任意のパスに
    `save_project` できるため。今は README で `--read-only` を勧めるに留めた。
  - OAuth (ChatGPT のコネクタの認証)。今はトークンだけ。
  - 長い処理 (PSD の取り込みなど) の進捗を SSE の `notifications/progress` で送る。
