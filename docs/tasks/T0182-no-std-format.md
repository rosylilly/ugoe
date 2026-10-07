---
id: T0182
title: ugoe-format を no_std + alloc にする (rmp-serde・ruzstd・std::io の扱い)
status: in-progress
milestone: M11
crates: [ugoe-format, Cargo.toml, Cargo.lock, scripts, docs]
depends_on: [T0179]
assignee: claude-worker-T0182
---

# T0182: ugoe-format を no_std + alloc にする (rmp-serde・ruzstd・std::io の扱い)

## 目的

`.ugoe` バンドルと JSON の読み書きを `no_std + alloc` でビルドできるようにする ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md))。
ゲーム機でもホストはバンドルのバイト列を渡すだけで読み込めるようにする。

## 受け入れ条件

- [ ] `rmp-serde` が no_std で使えるかを確かめた。使えなければ、MessagePack の読み書きを `rmp` などで書き直すか別のクレートに替え、
      書き出すバイト列が今と同じであることを、既存のバンドル (`assets/samples`・`assets/compat`) の読み書きで確かめた
- [ ] `ruzstd` を no_std で使い、`std::io::Read` に頼っている読み込み (`bundle.rs`・`image.rs`) をバイト列のスライスの読み込みにした
- [ ] PNG のデコード (`png`) と BCn のデコード (`bcdec_rs`) は `std` の feature の側に置いた (no_std では KTX2 の圧縮していない形と zstd だけを読む。
      デコードはもともとホストの役目)。`std` を外したときに使えない関数を README と doc コメントに書いた
- [ ] 読み込みの上限 (`ReadLimits`) が no_std でも同じに効く
- [ ] `ugoe-format` を `just nostd-check` の対象に足した
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、[ADR 0008](../adr/0008-bundle-payload-encoding.md)、[ADR 0030](../adr/0030-compressed-ktx2-decoding.md)、
  docs/spec/file-formats.md#読み込みの上限

## メモ

- 2026-10-06 起票。依存を替えるなら ADR 0031 の「見直すきっかけ」に当たるので、ADR に結果を書き足す。
