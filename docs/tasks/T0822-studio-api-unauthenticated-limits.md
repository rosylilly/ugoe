---
id: T0822
title: "スタジオの操作 API: 名乗らない接続と遅い要求の時間の上限"
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0814]
assignee:
---

# T0822: スタジオの操作 API: 名乗らない接続と遅い要求の時間の上限

## 目的

T0814 のレビューで見つけた。待ち受け (`apps/studio/src-tauri/src/api/server.rs`) は、

- `session.hello` をしないまま ping に pong を返し続ける接続を切らない (接続の上限 32 を埋められる)。
- 要求の頭を読む時間の上限 (`HEAD_TIMEOUT`) が 1 回の読み取りごとで、全体の上限が無い (1 バイトずつ送る相手を切らない)。
  接続ごとにスレッドを作るのは、頭を読み終えて経路が決まる前から。

どれもループバックの中のプロセスからだけ起きる (ADR 0028 の 6.) が、配信中に操作 API やブラウザソースが使えなくなる。

## 受け入れ条件

- [ ] 名乗る前の接続に時間の上限を設ける (例: 30 秒で `session.hello` が無ければ閉じる。承認待ちは除く)。閉じる符号を決めて
      docs/studio-api.md に書く
- [ ] 要求の頭を読み終えるまでの全体の時間に上限を設ける
- [ ] テスト (時刻は差し替えられる形にする。`server.rs` の `Keepalive` と同じ考え方)
- [ ] `just ci` が通る

## 参考

- ADR 0028 の 6.、docs/studio-api.md#close-codes

## メモ
