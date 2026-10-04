---
id: T0156
title: ugoe クレートの参照レンダラを feature で外せるようにする
status: in-progress
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, ugoe-headless, docs]
depends_on: [T0139]
assignee: claude-worker-T0156
---

# T0156: ugoe クレートの参照レンダラを feature で外せるようにする

## 目的

`ugoe` (crates.io に出すランタイム SDK) は `ugoe-render` (wgpu・naga) に無条件に依存し、`ugoe::render` として再エクスポートする。
エンジン独自のレンダラで描く組み込み先 (Unity・Godot・自前のエンジン) も wgpu をビルドすることになる。クレートの doc コメントは
「`render` feature 相当」と書いていたが、そうした feature は無い (T0139 で文言だけ直した)。

`render` feature (既定で有効) にして外せるようにするかを決め、するなら実装する。公開 API の形 (`ugoe::render` の有無) が変わるので、
ADR 0027 (crates.io への公開) との関係も確かめる。

## 受け入れ条件

- [ ] `render` を feature にするかを決め、理由をこのタスクのメモか ADR に書いた
- [ ] するなら: `default-features = false` の `ugoe` が wgpu に依存しないこと (`cargo tree`) と、ffi (`ugoe::render::MAX_LIGHTS` を使う)・wasm・headless がビルドできることを確かめた
- [ ] crates/ugoe/README.md と docs/portability.md を更新した
- [ ] `just ci` が通る

## 参考

- crates/ugoe/src/lib.rs・Cargo.toml
- docs/adr/0027-crates-io-publishing.md
- docs/portability.md

## メモ
