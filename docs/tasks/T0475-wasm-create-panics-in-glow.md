---
id: T0475
title: "wasm: WebPuppet の作成が WebGL のコンテキストを取れないときに glow の中で panic する"
status: todo
milestone: M9
crates: [ugoe-wasm, ugoe-render, packages/player, bindings/web, docs]
depends_on: []
assignee:
---

# T0475: wasm: WebPuppet の作成が WebGL のコンテキストを取れないときに glow の中で panic する

## 目的

T0938 の browser-test (`just player-browser`) を流している最中に、ugoe-wasm が glow の中で panic した
(`glow-0.17.0/src/web_sys.rs:285` の `Option::unwrap`)。ブラウザの WebGL のコンテキストの数の上限 (16) に当たっていた最中で、
コンテキストを作れなかったことが原因と見込まれる (T0938 の候補。確かめていない)。
`WebPuppet.create` は、作れないときに panic (wasm の trap で以後の呼び出しが壊れる) ではなく、コードのある例外で返すべき。

## 受け入れ条件

- [ ] 再現の手順をメモに書いた (コンテキストの上限を下げた Chrome、作成と破棄の繰り返しなど)。panic の起きる経路 (wgpu → glow の呼び出し) を特定した
- [ ] `WebPuppet.create` (と作り直しの経路) が、コンテキストを作れないときに panic せず、`name`・`code` のある例外 (T0434・T0465 の形に揃える) を返す。上流の glow の不具合なら、呼ぶ前に確かめて避けるか、上流へ報告する案をメモに
- [ ] `@ugoe/player` の PuppetSurface と `<ugoe-avatar>` が、その例外を作り直しの失敗として扱う (T0938 の回数に数える)。browser-test にシナリオがある
- [ ] `just wasm-check`・`just player-browser`・`just conformance`・`just ci` が通る

## 参考

- T0938 のメモ (起票の候補)、packages/player/browser-test/run.mjs (`--max-active-webgl-contexts`)
- crates/ugoe-wasm/README.md、docs/ui.md#webpuppet-の寿命

## メモ

- オーケストレーターが直接起票した (T0938 の候補。バグなので候補の一覧を待たずに起票)。
