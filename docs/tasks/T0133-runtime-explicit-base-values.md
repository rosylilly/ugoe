---
id: T0133
title: ランタイム: 受け取ったパラメータの値を基準値として確実に書く口 (スタジオの受け手のため)
status: todo
milestone: M8
crates: [ugoe-runtime, ugoe, ugoe-ffi, ugoe-wasm, bindings/web, docs]
depends_on: [T0800]
assignee:
---

# T0133: 受け取った値を基準値として書く口

## 目的

ADR 0028 の受け手 (出力ウィンドウ・ブラウザソース・仮想カメラ) は、指揮役から届いた値を基準値として書き、物理だけを自分で回す。
いまの Puppet の口では、モーション・表情・トラッキングの層と混ざらずに基準値を書けることが保証されていない (T0800 のメモ)。

## 受け入れ条件

- [ ] Puppet に、届いた値を基準値として書き、自分のモーション・表情・トラッキングの層を通さずに物理だけを進める口を足す (既定の挙動は変えない)
- [ ] C ABI と wasm に出す。WebPuppet にはパラメータを Float32Array でまとめて書く口も足す (T0808 の負荷を下げる)
- [ ] 同じ {dt, values} の列を与えた 2 つの Puppet (ネイティブと wasm) の頂点が許容誤差内で一致するテスト
- [ ] `just ci`・`just wasm-check`・`just ffi-check`・`just conformance --only c,wasm` が通る

## 参考

- docs/adr/0028-studio.md、docs/studio.md

## メモ
