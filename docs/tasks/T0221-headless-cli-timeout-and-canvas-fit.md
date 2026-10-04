---
id: T0221
title: "ヘッドレス描画の残り: ugoe render の GPU の待ちの上限と、render_png の大きなキャンバスの縮小"
status: done
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

- [x] `ugoe render` の上限の引数を出すか決めた。出すなら引数・既定値・0 や負の値の扱いを決め、テストと `crates/ugoe-cli/README.md` を更新した
- [x] `render_png` が大きさ未指定で 4096 を超えるキャンバスを縦横比を保って縮めて描き、縮めたことを返す。テストがある (GPU の要らない大きさの計算の部分)
- [x] `ugoe render` の大きさ未指定のときの振る舞いを `render_png` と揃えるか決め、文書とスキーマ (T0219 で揃えたもの) を更新した
- [x] `just ci` が通る

## 参考

- T0218・T0219 のメモ (起票の候補)
- crates/ugoe-headless/src/gpu.rs・args.rs、crates/ugoe-mcp/src/render.rs

## メモ

- T0019 で起票した (元の候補: T0218 の `--gpu-timeout`、T0219 の縮小)。
- 決めたこと:
  - **`ugoe render --gpu-timeout <SECONDS>` を出した。** 既定 30 (`DEFAULT_WAIT_TIMEOUT`)。`0` は上限なし (`Duration::MAX`)。
    負・`inf`・`NaN`・`Duration` に収まらない値 (`1e300`) は引数のエラー。読むのは `ugoe_headless::args::parse_gpu_timeout` (MCP でも使える)。
    上限を超えたら `rendering did not finish within --gpu-timeout (raise it, or 0 waits without a limit)` を原因の上に付けて失敗する。
  - **大きさを指定しないときの縮小は CLI と `render_png` で揃えた。** `RenderOptions::shrink_to` (既定 `Some(MAX_SIZE)`) を足し、
    `size`・`width`・`height` がどれも無いとき、キャンバス × `scale` の長い辺がこれを超えるなら縦横比を保って収まるよう縮め
    (`view::shrink_scale`)、警告 `the canvas (WxH[ at scale S]) is larger than N pixels on a side; rendered at wxh instead (give a size to choose it)`
    を出す。CLI は 16384 (以前はエラー)、`render_png` は 4096 (以前は 16384 まで描いた)。`render_png` は警告を `log` に入れて返す。
    明示した大きさと、`--width`・`--height` の片方から決めた他方は縮めない (従来どおり上限を超えたらエラー)。`--scale` を明示しても縮める
    (エラーにするより、警告を出して描くほうが使いやすいと判断)。
- やったこと: crates/ugoe-cli/README.md (オプションの表・「上限」・「GPU を待つ時間」)、crates/ugoe-mcp/README.md (「描画」)、
  crates/ugoe-headless/README.md、`render_png` のスキーマの `width`・`height` に `description` (`render::SIZE_NOTE`。縮めることを書く)。
- テスト: `view::shrink_scale_fits_the_longest_side`・`tests::shrinks_a_large_canvas_when_no_size_is_given` (headless)、
  `args::tests::parses_gpu_timeouts`、CLI の `parses_the_gpu_timeout`・`render_shrinks_a_canvas_larger_than_the_limit`、
  MCP の `render::tests::shrinks_a_large_canvas_to_the_size_limit` (どれも GPU を使わない)。
  手で: 20000x5000 のキャンバスを `ugoe render` (llvmpipe) で描き 16384x4096 が出ること、`--gpu-timeout 0.000001` で上限のエラーと案内が出ることを確かめた。
- やらなかったこと: MCP サーバ (`ugoe-mcp`) の起動引数に GPU を待つ上限を出すこと。GPU テスト (`--features gpu-tests`) は流していない (変えたのは大きさの計算と引数)。
- `just ci` は通った (main に rebase した後)。
- 起票の候補:
  - `render_png` で `width` か `height` の片方だけを指定したとき、縦横比から決めた他方が 4096 を超えても描く (CLI の 16384 まで)。
    縦長のキャンバスで `width: 4096` なら高さが 4096 を超える。他方も 4096 に収めるか (両方を縮めるか、エラーにするか) を決める。
  - `ugoe mcp` の起動引数に `--gpu-timeout` を出すか (今は既定の 30 秒だけ)。
