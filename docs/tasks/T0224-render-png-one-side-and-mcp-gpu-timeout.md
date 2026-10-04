---
id: T0224
title: render_png で片方だけ大きさを指定したときの上限と、ugoe mcp の --gpu-timeout (判断が要る)
status: todo
milestone: M9
crates: [ugoe-mcp, ugoe-headless, docs]
depends_on: []
assignee:
---

# T0224: render_png で片方だけ大きさを指定したときの上限と、ugoe mcp の --gpu-timeout (判断が要る)

## 目的

T0221 の残り (T0221 の候補)。

- MCP の `render_png` で `width` か `height` の片方だけを指定すると、縦横比から決めた他方が 4096 を超えても描く (CLI の上限 16384 まで)。
  縦長のキャンバスで `width: 4096` なら高さが 4096 を超える。
- `ugoe mcp` の起動引数に GPU を待つ上限 (`ugoe render --gpu-timeout` と同じもの。`ugoe_headless::args::parse_gpu_timeout`) が無い (今は既定の 30 秒だけ)。

**判断が要る:** 片方だけ指定したときに他方が上限を超えたら、両方を縮めて描くか (警告を `log` に入れる)、エラーにするか。

## 受け入れ条件

- [ ] 上の判断を決めた (理由を crates/ugoe-mcp/README.md の「描画」に書いた)
- [ ] 縦長・横長のキャンバスで片方だけ指定したときの振る舞いのテストがある (GPU を使わない大きさの計算で)
- [ ] `ugoe mcp --gpu-timeout <SECONDS>` があり、引数のエラー (負・NaN など) のテストがある。README とヘルプに書いた
- [ ] `just ci` が通る

## 参考

- T0221 のメモ (起票の候補)
- crates/ugoe-mcp/src/render.rs、crates/ugoe-headless/src/args.rs・view.rs

## メモ

- T0026 で起票した (元の候補: T0221 の 2 つ)。
