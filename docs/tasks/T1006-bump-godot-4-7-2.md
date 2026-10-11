---
id: T1006
title: "Godot を 4.7.2 に上げる (godot-check・CI のヘッドレスの Godot・GDExtension の互換)"
status: todo
milestone: Release
crates: [bindings/godot, scripts, .github/workflows, docs]
depends_on: []
assignee:
---

# T1006: Godot を 4.7.2 に上げる

## 目的

`just tool-updates` (T1004) が Godot 4.7.2-stable を知らせた。利用者の決定 (2026-10-12): 上げる。

## 受け入れ条件

- [ ] 固定している Godot の版と SHA512 (T1004 の `PINNED_TOOLS` の写しをすべて) を 4.7.2-stable にした。sha512 は公式の配布から取り直した
- [ ] godot-rust の対応する API の版 (`api-4-x` の feature など) と `.gdextension` の `compatibility_minimum` を確かめ、要るなら直した (下げられる最小の版の方針を README に書く)
- [ ] `just godot-check` (ヘッドレスの smoke)・`just conformance --only godot`・`node scripts/check-licenses.mjs --godot` が通る
- [ ] bindings/godot/README.md と docs/release.md の版の記述を直した
- [ ] `just ci` が通る

## 参考

- T1004 のメモ、bindings/godot (build.mjs・Cargo.toml・example)、ADR 0026、docs/release.md#固定した道具の版

## メモ

- 利用者の決定 (2026-10-12) を受けてオーケストレーターが起票した。
