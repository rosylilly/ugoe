---
id: T0822
title: "スタジオの操作 API: 名乗らない接続と遅い要求の時間の上限"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0814]
assignee: claude-worker-T0822
---

# T0822: スタジオの操作 API: 名乗らない接続と遅い要求の時間の上限

## 目的

T0814 のレビューで見つけた。待ち受け (`apps/studio/src-tauri/src/api/server.rs`) は、

- `session.hello` をしないまま ping に pong を返し続ける接続を切らない (接続の上限 32 を埋められる)。
- 要求の頭を読む時間の上限 (`HEAD_TIMEOUT`) が 1 回の読み取りごとで、全体の上限が無い (1 バイトずつ送る相手を切らない)。
  接続ごとにスレッドを作るのは、頭を読み終えて経路が決まる前から。

どれもループバックの中のプロセスからだけ起きる (ADR 0028 の 6.) が、配信中に操作 API やブラウザソースが使えなくなる。

## 受け入れ条件

- [x] 名乗る前の接続に時間の上限を設ける (例: 30 秒で `session.hello` が無ければ閉じる。承認待ちは除く)。閉じる符号を決めて
      docs/studio-api.md に書く
- [x] 要求の頭を読み終えるまでの全体の時間に上限を設ける
- [x] テスト (時刻は差し替えられる形にする。`server.rs` の `Keepalive` と同じ考え方)
- [x] `just ci` が通る

## 参考

- ADR 0028 の 6.、docs/studio-api.md#close-codes

## メモ

- やったこと (`apps/studio/src-tauri/src/api/server.rs`・`mod.rs`):
  - 名乗るまでの期限 `HelloDeadline` (30 秒)。期限を過ぎて接続がまだ `Session::New` なら close `4008` (`CLOSE_HELLO_TIMEOUT`、HTTP の 408 に倣う) で閉じる。
    承認待ち (`Pending`) と `Ready` は名乗ったとみなし、以後は確かめない (名簿の錠は期限を過ぎたときだけ取る)。`Studio::api_introduced` を足した。
  - 要求の頭を読む全体の期限 (接続してから 10 秒)。`Deadlined` で読み取りのたびにソケットの読み取りの上限を期限までの残りにし、過ぎたら `TimedOut`。
  - 頭を読んでいる途中の接続を 64 まで (`HeadSlot`)。超えた接続はスレッドを作らずに閉じる (受け入れ条件の外だが「目的」の 3 つめへの対処として小さく入れた)。
  - テスト: `HelloDeadline` は時刻を引数で渡す単体テスト、全体の期限は実際のソケットで 1 バイトずつ送る相手を 300 ms の期限で切るテスト、
    `HeadSlot` の上限、`api_introduced` (`tests.rs` の最初の接続のテストに追加)。
  - docs: `docs/studio-api.md` (Connecting と Close codes に `4008`、頭の 10 秒)、`docs/studio.md` (外部 API の節)。
- やらなかったこと: 30 秒・10 秒を実際に待つ結合テスト (時間が掛かるため。期限の判断は単体テストで確かめた)。e2e は流していない (フロントエンドは変えていない)。
- `just ci` は通った。
- 起票の候補: なし。
