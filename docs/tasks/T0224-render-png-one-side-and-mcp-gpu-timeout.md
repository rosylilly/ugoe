---
id: T0224
title: render_png で片方だけ大きさを指定したときの上限と、ugoe mcp の --gpu-timeout (判断が要る)
status: done
milestone: M9
crates: [ugoe-mcp, ugoe-headless, docs]
depends_on: []
assignee: claude-worker-T0224
---

# T0224: render_png で片方だけ大きさを指定したときの上限と、ugoe mcp の --gpu-timeout (判断が要る)

## 目的

T0221 の残り (T0221 の候補)。

- MCP の `render_png` で `width` か `height` の片方だけを指定すると、縦横比から決めた他方が 4096 を超えても描く (CLI の上限 16384 まで)。
  縦長のキャンバスで `width: 4096` なら高さが 4096 を超える。
- `ugoe mcp` の起動引数に GPU を待つ上限 (`ugoe render --gpu-timeout` と同じもの。`ugoe_headless::args::parse_gpu_timeout`) が無い (今は既定の 30 秒だけ)。

**判断が要る:** 片方だけ指定したときに他方が上限を超えたら、両方を縮めて描くか (警告を `log` に入れる)、エラーにするか。

## 受け入れ条件

- [x] 上の判断を決めた (理由を crates/ugoe-mcp/README.md の「描画」に書いた)
- [x] 縦長・横長のキャンバスで片方だけ指定したときの振る舞いのテストがある (GPU を使わない大きさの計算で)
- [x] `ugoe mcp --gpu-timeout <SECONDS>` があり、引数のエラー (負・NaN など) のテストがある。README とヘルプに書いた
- [x] `just ci` が通る

## 参考

- T0221 のメモ (起票の候補)
- crates/ugoe-mcp/src/render.rs、crates/ugoe-headless/src/args.rs・view.rs

## メモ

- T0026 で起票した (元の候補: T0221 の 2 つ)。
- 決めたこと: **片方だけ指定して他方が 4096 を超えたらエラーにする** (縮めない)。理由は crates/ugoe-mcp/README.md の「描画」:
  明示した大きさを縮めない `ugoe render` の規則 (T0221) と、両方を明示して超えたときのエラーに揃える。縮めると `width: 4096` に
  2048 幅の画が返り、エージェントは `log` の警告を見落としやすい。エラーには収まる最大の値を書く
  (`width 4096 gives a height of 8192 for the 3000x6000 canvas, larger than 4096; give a width of at most 2048, or the height instead`。
  1 でも収まらない縦横比では `give the height instead`)。検査は `render::check_derived_size` で、GPU を使う前に行う。
  丸めは `ugoe_headless::output_size` と同じ式・順で計算する。
- `ugoe-mcp --gpu-timeout <SECONDS>` を出した (`ugoe_headless::args::parse_gpu_timeout` で読む。既定 30、`0` は上限なし)。
  `Options::gpu_timeout: Option<Duration>` (None は既定) を足し、`Renderer::new(backend, wait_timeout)` が GPU を作るときに
  `Gpu::set_wait_timeout` する。`--editor`・`--mcpb` とは一緒に使えない (clap の衝突)。上限を超えたときのエラーに
  `(the server's --gpu-timeout sets the limit)` を足した。
- テスト (GPU を使わない): `render::tests::rejects_a_derived_side_over_the_size_limit` (縦長で width・横長で height・丸めの境目・
  1 でも収まらない縦横比)、`render::tests::checks_the_derived_side_before_touching_the_gpu`、main.rs の `tests::parses_the_gpu_timeout`
  (`-1`・`NaN`・`inf`・`1e300`・`abc`・空、`--editor`・`--mcpb` との衝突)。
- 文書: crates/ugoe-mcp/README.md (オプションの表・「描画」)、crates/ugoe-headless/README.md (1 語)、`render_png` のスキーマの
  `SIZE_NOTE`。範囲外の小さな変更: crates/ugoe-cli/README.md の「上限」の MCP の行に、片方から決めた他方も 4096 までと足した。
- やらなかったこと: GPU テスト (`--features gpu-tests`) は流していない (変えたのは大きさの検査と引数。GPU の待ちの上限は
  `Gpu::set_wait_timeout` をそのまま呼ぶだけ)。実際の MCP クライアントからつないで確かめてはいない。
- `just ci` は通った (main 9d39c65f の上)。
- 起票の候補: なし。
