---
id: T0556
title: "ugoe-mcp: HTTP の誤りの答え (408・413・431) の後に接続をすぐ閉じず、Windows のクライアントが答えを失わないようにする"
status: done
milestone: M9
crates: [ugoe-mcp]
depends_on: []
assignee: claude-worker-T0556
---

# T0556: ugoe-mcp: HTTP の誤りの答え (408・413・431) の後に接続をすぐ閉じず、Windows のクライアントが答えを失わないようにする

## 目的

**優先度は高め。** `crates/ugoe-mcp/src/http.rs` は、誤りの答え (408・413・431) を書いた直後に、読み残しのある接続を閉じる。
読み残しがあるまま閉じると TCP の RST が送られ、Windows のクライアントは受け取った答えを捨てる (T0013 で Wine の上の `slow_requests_time_out_with_408` が失敗した。
`body_over_the_limit_is_413` なども同じ条件になりうる)。新しい `rust (windows)` の CI のジョブでも落ちる見込み (T0013 の候補)。

答えを書いた後に書く側を閉じ (`shutdown(Write)`)、短い上限の間だけ読み捨ててから閉じる (lingering close) 形にする。

## 受け入れ条件

- [x] 誤りの答えの後に書く側を閉じ、上限 (時間と量) 付きで読み捨ててから閉じる。上限があるので遅いクライアントで止まらない
- [x] 本文を送り終える前に答えを読むクライアントで、408・413・431 の答えを受け取れるテストがある
- [x] `rust (windows)` の CI のジョブ (か Wine) で `slow_requests_time_out_with_408`・`body_over_the_limit_is_413` が通ることを確かめた (確かめられなければメモに)
- [x] `just ci` が通る

## 参考

- T0013 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0013 の lingering close)。
- やったこと (claude-worker-T0556):
  - `crates/ugoe-mcp/src/http.rs`: 要求を読み終える前の誤りの答え (`ReadError::Timeout` の 408 と `ReadError::Bad` の
    413・431・400・501・505) を書けたら、`linger` で `shutdown(Write)` し、`Limits::linger_timeout` (既定 2 秒) と
    `Limits::max_linger_bytes` (既定 1 MiB) の上限まで読み捨ててから閉じる。相手が書く側を閉じれば (EOF) すぐ戻る。
    上限は内部の `Limits` に足しただけで、コマンド行のオプションは足していない。README の「上限」と http.rs の冒頭の doc に書いた。
  - テスト: `error_responses_reach_clients_that_are_still_sending` (408・413・431 を、残りを送る前に読み、その後も送れる)、
    `lingering_close_is_bounded` (送り続ける相手でも時間の上限で閉じる・時間が長くても量の上限で閉じる)。修正前は Linux でも
    2 つとも失敗することを確かめた。
  - `slow_requests_time_out_with_408` の 2 つ目の場面は、送り終えてから読むクライアントだと、閉じた後の書き込みで RST が返り
    Windows では答えを失う (サーバ側ではどうにもならない) ので、送るのを別のスレッドにして送りながら読む形に直した。
  - Wine (docs/testing.md の手順。ただし sandbox では `/tmp/ugoe-wine` が使えず、WINEPREFIX を別の場所にした) で
    `cargo test -p ugoe-mcp --target x86_64-pc-windows-gnu` を 3 回流し、すべて通った (`slow_requests_time_out_with_408`・
    `body_over_the_limit_is_413` を含む)。本物の Windows (`rust (windows)` の CI) では確かめていない。
  - Windows でのビルドで `tools.rs` のテストのモジュールが `unused import: super::*` の警告を出していた (中のテストが `cfg(unix)`
    だけ)。clippy の `-D warnings` で落ちるので、モジュールごと `cfg(all(test, unix))` にした。windows-gnu で clippy
    (`--all-targets -D warnings`) が通ることを確かめた。
- やらなかったこと: 接続の数の上限を超えたときの `503` は受け付けのスレッドで書いてすぐ閉じるので、読み残しがあれば同じく RST に
  なりうる。受け付けのスレッドで読み捨てると受け付けが止まるので、手を付けなかった (相手は要求をまだ送っていないことが多い)。
- 起票の候補: `503` (接続の数の上限) の答えも RST で失われないようにする (枠の外の短い読み捨て用のスレッドなど)。
