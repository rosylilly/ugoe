---
id: T0221
title: "ヘッドレス描画の残り: ugoe render の GPU の待ちの上限と、render_png の大きなキャンバスの縮小"
status: in-progress
milestone: M9
crates: [ugoe-headless, ugoe-cli, ugoe-mcp, docs]
depends_on: []
assignee: claude-worker-T0221
---

# T0221: ヘッドレス描画の残り: ugoe render の GPU の待ちの上限と、render_png の大きなキャンバスの縮小

## 目的

- **GPU の待ちの上限 (T0218):** T0218 で GPU を待つ時間に上限 (既定 30 秒、`Gpu::set_wait_timeout`) を設けたが、`ugoe render` (CLI) からは変えられない。
  遅いソフトウェアの GPU (lavapipe) で大きなモデルを描く利用者のために、`--gpu-timeout <秒>` のような引数を出すかを決める。
- **大きなキャンバス (T0219):** MCP の `render_png` は、大きさを指定しないとキャンバスの大きさで描き、4096 を超えるとエラーになる。
  エージェントは大きさを指定しないことが多いので、4096 に収まるよう縮めて描き、縮めたことをログ (と結果) に書く。

## 受け入れ条件

- [ ] `ugoe render` の上限の引数を出すか決めた。出すなら引数・既定値・0 や負の値の扱いを決め、テストと `crates/ugoe-cli/README.md` を更新した
- [ ] `render_png` が大きさ未指定で 4096 を超えるキャンバスを縦横比を保って縮めて描き、縮めたことを返す。テストがある (GPU の要らない大きさの計算の部分)
- [ ] `ugoe render` の大きさ未指定のときの振る舞いを `render_png` と揃えるか決め、文書とスキーマ (T0219 で揃えたもの) を更新した
- [ ] `just ci` が通る

## 参考

- T0218・T0219 のメモ (起票の候補)
- crates/ugoe-headless/src/gpu.rs・args.rs、crates/ugoe-mcp/src/render.rs

## メモ

- T0019 で起票した (元の候補: T0218 の `--gpu-timeout`、T0219 の縮小)。
