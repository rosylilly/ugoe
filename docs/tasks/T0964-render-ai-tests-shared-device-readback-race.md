---
id: T0964
title: "render・ai の GPU テスト: 共有したデバイスで poll の直後に読み戻すと、コールバックが呼ばれる前で揺れる"
status: done
milestone: M9
crates: [ugoe-render, ugoe-ai, docs]
depends_on: [T0963]
assignee: claude-worker-T0964
---

# T0964: render・ai の GPU テスト: 共有したデバイスで poll の直後に読み戻すと、コールバックが呼ばれる前で揺れる

## 目的

T0963 で、wgpu の `Device::poll` は読み戻しのコールバックをロックを外してから呼ぶので、ほかのスレッドが同じデバイスを poll していると
`poll(Wait)` が戻ってもこちらのコールバックがまだ呼ばれていないことがある、と分かった (`ugoe-headless` は `Gpu::wait_until` で直した)。
`crates/ugoe-render/tests/gpu.rs`・`crates/ugoe-render/tests/spec_render.rs`・`crates/ugoe-ai/tests/normal_render.rs` も 1 つのデバイスを
テストのスレッドで共有し、`poll(wait_indefinitely)` の直後に `get_mapped_range` しているので、CI の `gpu (gl)`・`gpu (vulkan)` で同じ揺れが起こりうる (T0963 の候補)。

## 受け入れ条件

- [x] 3 つのテストの読み戻しを、コールバックが呼ばれたことを確かめてから読む形に直した (上限のある回数で poll し直す。共通の補助にできるならまとめる。時間だけの待ちで隠さない)
- [x] ライブラリの側 (`ugoe-render` などの本体) に同じ前提の読み戻しが無いか探し、あれば直した (無ければメモに書く)
- [x] 並行に何度も流して (GL: `WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1`、`--test-threads` を増やして) 落ちないことを確かめた
- [x] `just ci` が通る

## 参考

- T0963 (crates/ugoe-headless/src/gpu.rs の `Gpu::wait_until`)、docs/testing.md (描画テスト)

## メモ

- T0963 のワーカーの報告から起票した (2026-10-06)。
- **確かめたこと:** 補助に一時的な出力を入れて GL (llvmpipe) と Vulkan (lavapipe) で `--test-threads=16` で流すと、
  `poll(wait_indefinitely)` が戻った時点でコールバックがまだ呼ばれていない場面は毎回のように起きていた。ただしそのときも
  `get_mapped_range` は成功した。wgpu-core 30 は `Queue::maintain` の中で、ライフトラッカーのロックを持ったままバッファの状態を
  `Active` にし (`LifetimeTracker::handle_mapping` → `Buffer::map`)、ロックを外してからコールバックを呼ぶので、
  ほかのスレッドが「キューが空」を見た時点でもうマップは済んでいる。このため直す前のテストが実際に落ちることはなかった
  (直す前の形でも 15 回 × `--test-threads=16` で通った)。残っていた問題は、(1) 文書にない実装の都合に頼っていること、
  (2) コールバックの `r.expect("map readback")` が、マップに失敗したときにほかのテストのスレッドの `poll` の中で panic しうること。
- **直したこと:** `crates/ugoe-render/tests/common/mod.rs` に `map_read(device, buffer)` を置き、3 つのテストの読み戻しをこれにした。
  コールバックの結果をチャンネルで受け取り、受け取れなければ 1 ms の間を置いて poll し直す (上限 1000 回、超えたら panic)。
  マップの失敗はそのテストのスレッドで panic する。ugoe-ai の `tests/normal_render.rs` は `#[path]` でこのファイルを取り込む
  (補助は wgpu だけに依存させてある)。ランタイム層 (ugoe-render) はツール層 (ugoe-headless) に依存できないので、`Gpu::wait_until` は使っていない。
- **テスト:** `tests/gpu.rs` に `readbacks_from_many_threads_share_one_device` (8 スレッドが 16 回ずつ描いて読む) を足した。
  上のとおり直す前でも通るので、失敗するテストを先に書くことはできなかった (揺れの検出ではなく、並行の読み戻しの回帰の検査)。
- **並行の確認:** `ugoe-render` の `gpu`・`spec_render` と `ugoe-ai` の `normal_render` を、GL (`WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1`)
  と Vulkan (lavapipe) でそれぞれ 10 回、`--test-threads=16` で流してすべて通った。
- **ライブラリの側:** `map_async` を使うのは ugoe-headless (T0963 で直した) と `ugoe-wasm` の `web/readback.rs` だけ。
  後者はコールバックが結果を置くまで `poll(Poll)` と待ちを回数の上限付きで繰り返しているので、同じ前提は無い。ugoe-render の本体には読み戻しが無い。
- 文書: docs/testing.md の「描画テスト」と crates/ugoe-render/README.md の「テスト」に、`map_read` と wgpu-core の振る舞いを書いた。
- `just ci` は通った。ライブラリのコードは変えていないので `just wasm-check` は流していない。
- 起票の候補: なし。
