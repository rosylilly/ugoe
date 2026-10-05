---
id: T0421
title: 公開: Godot の配布物の自動ビルドと、UPM の tarball を GitHub Release に添付する
status: todo
milestone: Release
crates: [bindings/godot, bindings/unity, .github/workflows, justfile, docs]
depends_on: [T0415, T0420]
assignee:
---

# T0421: Godot と Unity の配布物

## 目的

T0415・T0420 から、公開に関わる項目を分けた。公開の時期は v1.0.0 の完全リリースと判断するまで決めない (docs/roadmap.md の Release)。

## 受け入れ条件

- [ ] Godot: 各 OS (Linux・Windows・macOS) のビルドと、アセットライブラリ向けの zip を CI で作る (公開は利用者が行う)
- [ ] Unity: UPM の tarball を GitHub Release に添付する (公開は利用者が行う)
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0415-godot-distribution.md`・`docs/tasks/T0420-unity-and-conformance-extras.md`、`docs/release.md`

## メモ

- T0032 (3 回目の起票のまとめ) から: T0166 の候補「C ABI の共有ライブラリ (Unity の配布物) が wgpu を外してどれだけ小さくなったかを、リリースビルドで
  測って docs/portability.md に書く」をこのタスクで扱う (配布物を作るときに測る)。
