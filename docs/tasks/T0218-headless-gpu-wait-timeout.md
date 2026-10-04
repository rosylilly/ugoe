---
id: T0218
title: ヘッドレス描画: GPU を待つときに時間の上限を設ける
status: in-progress
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

- [ ] 上限の値を決める (例: 1 枚の描画で 30 秒)。呼び出し側が変えられるようにするかを判断し、doc コメントに書く
- [ ] `Gpu::render` と `Resident::poll(Wait::All)` が上限を超えたら `Error::Gpu` を返す (止まり続けない)
- [ ] MCP サーバ (`crates/ugoe-mcp/src/render.rs`) が上限を超えた GPU を使い回さない
- [ ] crates/ugoe-headless/README.md の「GPU」に書く
- [ ] `just ci` が通る

## 参考

- crates/ugoe-headless/src/gpu.rs (`render_unscoped`)、crates/ugoe-headless/src/resident.rs (`poll`)
- crates/ugoe-mcp/src/render.rs (GPU の使い回しと panic の扱い)

## メモ
