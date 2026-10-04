---
id: T0556
title: "ugoe-mcp: HTTP の誤りの答え (408・413・431) の後に接続をすぐ閉じず、Windows のクライアントが答えを失わないようにする"
status: in-progress
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

- [ ] 誤りの答えの後に書く側を閉じ、上限 (時間と量) 付きで読み捨ててから閉じる。上限があるので遅いクライアントで止まらない
- [ ] 本文を送り終える前に答えを読むクライアントで、408・413・431 の答えを受け取れるテストがある
- [ ] `rust (windows)` の CI のジョブ (か Wine) で `slow_requests_time_out_with_408`・`body_over_the_limit_is_413` が通ることを確かめた (確かめられなければメモに)
- [ ] `just ci` が通る

## 参考

- T0013 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0013 の lingering close)。
