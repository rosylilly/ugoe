---
id: T0963
title: "headless: CI の gpu (gl) で、読み戻しのコールバックが呼ばれないまま受け取ろうとして失敗する (readback callback was not called)"
status: done
milestone: M9
crates: [ugoe-headless, docs]
depends_on: []
assignee: claude-worker-T0963
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

- [x] 原因を確かめ、メモに書いた (読み戻しの待ち方の誤りか、テストの共有のしかたの誤りか)
- [x] ライブラリの誤りならライブラリを直す (コールバックが呼ばれるまで、上限のある回数で poll し直すなど。時間だけの待ちで隠さない)。テストの誤りならテストを直す
- [x] 並行に何度も流して (例: `--test-threads` を増やして 20 回。GL のアダプタが手元にあれば GL で) 落ちないことを確かめた
- [x] `just ci` が通る

## 参考

- crates/ugoe-headless (読み戻し)、crates/ugoe-headless/tests/resident_gpu.rs、T0220、docs/testing.md (GPU のテスト)

## メモ

- CI の失敗から起票した (2026-10-06)。
- **原因 (ライブラリの誤り):** wgpu-core 30 の `Device::poll` は、終わった提出の `map_async` のコールバックをキューのロックの中で集め、
  ロックを外してから呼ぶ (`device/resource.rs` の `maintain` → `UserClosures::fire`)。同じデバイスを 2 つのスレッドが poll すると、
  先のスレッドがこちらのコールバックを集めてまだ呼んでいないうちに、こちらの `poll(Wait)` が「キューが空」で `Ok` を返す。
  `Gpu::render` は待ち終わればコールバックは呼ばれていると決めつけて `try_recv` していたので、空のチャンネルで失敗した。
  `Resident::poll(Wait::All)` も同じ前提で、スロットが `PENDING` のまま `None` を返しえた (スタジオの仮想カメラも同じ口を使う)。
  テストの共有のしかたの誤りではなく、ホストが同じ `Gpu` を複数のスレッドから使えば起こる。
- **直したこと:** `Gpu::wait_until(done)` (`gpu.rs`)。poll が終わっても `done` (コールバックが呼ばれたか) が `None` なら、
  1 ms の間を置いて poll し直す。回数の上限は 1000 回 (`CALLBACK_RETRIES`) と待つ時間の上限 (`Gpu::wait_timeout`)。
  超えたら `Error::Gpu` ("readback callback was not called")。`Gpu::render` と `Resident::poll(Wait::All)` がこれで待つ。
- **テスト:** `tests/resident_gpu.rs` の `readbacks_from_many_threads_share_one_gpu` (8 スレッドが 1 枚の口と常駐の口で 16 回ずつ)。
  直す前は GL (llvmpipe) で 1 回目から同じエラーで落ちた。直した後は `WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1` で
  `--test-threads=16` を 20 回流してすべて通り、既定 (lavapipe の Vulkan) でも通った。`gpu.rs` の単体テストに、poll し直すことと上限で諦めることを足した。
- 文書: crates/ugoe-headless/README.md の「GPU」と「テスト」、docs/testing.md の「描画テスト」に、共有したデバイスの読み戻しの注意を書いた。
- `just ci` は通った。
- **起票の候補:** `crates/ugoe-render/tests/gpu.rs`・`crates/ugoe-render/tests/spec_render.rs`・`crates/ugoe-ai/tests/normal_render.rs` も
  1 つのデバイスをテストのスレッドで共有し、`poll(wait_indefinitely)` の直後に `get_mapped_range` するので、同じ競合で揺れうる
  (このタスクの `crates:` の外なので触っていない)。コールバックが呼ばれるまで poll し直すようにする。
