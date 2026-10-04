---
id: T0218
title: ヘッドレス描画: GPU を待つときに時間の上限を設ける
status: done
milestone: M9
crates: [ugoe-headless, ugoe-mcp, docs]
depends_on: [T0212]
assignee: claude-worker-T0218
---

# T0218: ヘッドレス描画: GPU を待つときに時間の上限を設ける

## 目的

T0212 (ugoe-headless のコードレビュー) の所見。`Gpu::render` と `Resident::poll(Wait::All)` は
`wgpu::PollType::wait_indefinitely()` で読み戻しを待つので、ドライバが止まると呼び出したスレッドが戻らない。
MCP サーバの `render_png` は描画の錠を持ったまま止まり、以後の描画がすべて待ちになる (panic と違って `catch_unwind` でも受けられない)。

wgpu の `PollType::Wait` の時間の上限を使い、上限を超えたら `Error::Gpu` を返す。上限を超えた GPU は MCP サーバでは
panic のときと同じく捨てて (破棄せずに手放して) 作り直す。

## 受け入れ条件

- [x] 上限の値を決める (例: 1 枚の描画で 30 秒)。呼び出し側が変えられるようにするかを判断し、doc コメントに書く
- [x] `Gpu::render` と `Resident::poll(Wait::All)` が上限を超えたら `Error::Gpu` を返す (止まり続けない)
- [x] MCP サーバ (`crates/ugoe-mcp/src/render.rs`) が上限を超えた GPU を使い回さない
- [x] crates/ugoe-headless/README.md の「GPU」に書く
- [x] `just ci` が通る

## 参考

- crates/ugoe-headless/src/gpu.rs (`render_unscoped`)、crates/ugoe-headless/src/resident.rs (`poll`)
- crates/ugoe-mcp/src/render.rs (GPU の使い回しと panic の扱い)

## メモ

- やったこと:
  - 上限は `ugoe_headless::DEFAULT_WAIT_TIMEOUT` = 30 秒 (1 回の呼び出しで待つ合計)。呼び出し側は `Gpu::set_wait_timeout` で変えられる
    (`Duration::MAX` で上限なし)。`Gpu::wait_timeout` の doc コメントと README の「GPU」に書いた。CLI・MCP サーバの引数には出していない。
  - `Gpu::render` と `Resident::poll(Wait::All)` は `Gpu::wait` (中は `wait_with`) で待ち、超えたら `Error::Gpu`
    (context `the GPU did not finish in time`) を返す。見分けるのは `Error::is_gpu_wait_timeout`、作るのは `Error::gpu_wait_timeout`。
  - **GL のバックエンドの既存の問題も直した:** wgpu 30 の GL は 1 回の待ちを約 2.1 秒 (`i32::MAX` ns) で打ち切るので、
    `wait_indefinitely` でも 2.1 秒を超える描画 (llvmpipe で大きな画) は `waiting for the GPU: ... timed out` で失敗していた。
    1 秒ずつ待ち直して上限まで待つので、どのバックエンドでも上限の意味が同じになる。
  - `Gpu::render` の読み戻しのコールバックは `recv` (待つ) ではなく `try_recv` で受ける (呼ばれなかったときに止まらないように)。
  - MCP サーバ (`render.rs`) は上限を超えたら `RenderError::TimedOut` (GPU を捨てて次で作り直すと書く) を返し、GPU を手放す
    (panic のときと同じく破棄はしない)。crates/ugoe-mcp/README.md にも書いた。
  - テスト: `wait_with` の単体テスト (上限を超えたらエラー・刻みは 1 秒以下・刻みで打ち切られても待ち直す・ほかのエラーは上限のエラーにしない)。
    `tests/wait_timeout_gpu.rs` (gpu-tests): 上限 0 で `Gpu::render`・`Resident::poll(Wait::All)` が上限のエラーか成功で戻り、
    上限を戻せば同じ GPU で描き続けられる (lavapipe では 2 回のうち 1 回が実際に上限のエラーになった)。MCP サーバの単体テスト
    (エラーの文・gpu-tests で GPU を捨てて作り直す)。
- 確かめていないこと: 実際に止まるドライバ (本当のハング) での振る舞い。wgpu の待ちが上限を守ることは Vulkan (lavapipe) で確かめた。
  DX12・Metal は hal の実装を読んだだけ (どちらも上限を受け取る)。
- `just ci`: 1 回目は packages/player の vitest の 1 件 (`rememberedModels.test.ts`) が負荷 (load average 24) で 5 秒の時間切れ。
  この変更と無関係で、単独で流すと 117 件すべて通った。`just web-check i18n app-build-check` (失敗以降の段) を流し直して通った。
  それより前の段 (fmt-check・lint・test・layers・groups・licenses・publish-check・spec) は 1 回目で通っている。
- 起票の候補:
  - スタジオ (`apps/studio/src-tauri/src/virtual_camera.rs`) は止めるときの `poll(Wait::All)` の結果を捨てている。毎フレームの `submit`/`poll(Wait::No)` は
    待たないので止まらないが、GPU が止まると読み戻しが来なくなる。上限を超えたかを見て受け手の GPU を作り直すかは、スタジオの側で判断する。
  - `ugoe render` (CLI) に `--gpu-timeout` のような上限の引数を出すか (今は既定の 30 秒だけ)。

