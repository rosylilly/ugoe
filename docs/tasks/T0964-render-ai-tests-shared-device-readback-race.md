---
id: T0964
title: "render・ai の GPU テスト: 共有したデバイスで poll の直後に読み戻すと、コールバックが呼ばれる前で揺れる"
status: todo
milestone: M9
crates: [ugoe-render, ugoe-ai, docs]
depends_on: [T0963]
assignee:
---

# T0964: render・ai の GPU テスト: 共有したデバイスで poll の直後に読み戻すと、コールバックが呼ばれる前で揺れる

## 目的

T0963 で、wgpu の `Device::poll` は読み戻しのコールバックをロックを外してから呼ぶので、ほかのスレッドが同じデバイスを poll していると
`poll(Wait)` が戻ってもこちらのコールバックがまだ呼ばれていないことがある、と分かった (`ugoe-headless` は `Gpu::wait_until` で直した)。
`crates/ugoe-render/tests/gpu.rs`・`crates/ugoe-render/tests/spec_render.rs`・`crates/ugoe-ai/tests/normal_render.rs` も 1 つのデバイスを
テストのスレッドで共有し、`poll(wait_indefinitely)` の直後に `get_mapped_range` しているので、CI の `gpu (gl)`・`gpu (vulkan)` で同じ揺れが起こりうる (T0963 の候補)。

## 受け入れ条件

- [ ] 3 つのテストの読み戻しを、コールバックが呼ばれたことを確かめてから読む形に直した (上限のある回数で poll し直す。共通の補助にできるならまとめる。時間だけの待ちで隠さない)
- [ ] ライブラリの側 (`ugoe-render` などの本体) に同じ前提の読み戻しが無いか探し、あれば直した (無ければメモに書く)
- [ ] 並行に何度も流して (GL: `WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1`、`--test-threads` を増やして) 落ちないことを確かめた
- [ ] `just ci` が通る

## 参考

- T0963 (crates/ugoe-headless/src/gpu.rs の `Gpu::wait_until`)、docs/testing.md (描画テスト)

## メモ

- T0963 のワーカーの報告から起票した (2026-10-06)。
