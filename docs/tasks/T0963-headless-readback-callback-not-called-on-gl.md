---
id: T0963
title: "headless: CI の gpu (gl) で、読み戻しのコールバックが呼ばれないまま受け取ろうとして失敗する (readback callback was not called)"
status: todo
milestone: M9
crates: [ugoe-headless, docs]
depends_on: []
assignee:
---

# T0963: headless: CI の gpu (gl) で、読み戻しのコールバックが呼ばれないまま受け取ろうとして失敗する (readback callback was not called)

## 目的

main の CI (https://github.com/rosylilly/ugoe/actions/runs/37447416182/job/112215479989 、`gpu (gl)`、llvmpipe の GL) で次が落ちた。

```
thread 'yuv_on_the_cpu_matches_the_gpu_path' panicked at crates/ugoe-headless/tests/resident_gpu.rs:143:48:
called `Result::unwrap()` on an `Err` value: Gpu { context: "readback callback was not called", detail: "receiving on an empty channel" }
```

143 行目は 1 回きりの描画 (`prepared.render(gpu())`) の読み戻し。ほかのテストと同じ GPU (デバイス) を並行して使っているので、
別のスレッドの `poll` が先にコールバックを消費する・待ちの `poll` が戻った時点でまだコールバックが呼ばれていない、などの競合が疑われる
(前の run 37429758787 の同じジョブは通っていた。揺れ)。テストだけでなく、ホスト (CLI・MCP サーバ・スタジオ) で同じデバイスを複数のスレッドから
使ったときにも起こりうるなら、ライブラリの側で直す。

## 受け入れ条件

- [ ] 原因を確かめ、メモに書いた (読み戻しの待ち方の誤りか、テストの共有のしかたの誤りか)
- [ ] ライブラリの誤りならライブラリを直す (コールバックが呼ばれるまで、上限のある回数で poll し直すなど。時間だけの待ちで隠さない)。テストの誤りならテストを直す
- [ ] 並行に何度も流して (例: `--test-threads` を増やして 20 回。GL のアダプタが手元にあれば GL で) 落ちないことを確かめた
- [ ] `just ci` が通る

## 参考

- crates/ugoe-headless (読み戻し)、crates/ugoe-headless/tests/resident_gpu.rs、T0220、docs/testing.md (GPU のテスト)

## メモ

- CI の失敗から起票した (2026-10-06)。
