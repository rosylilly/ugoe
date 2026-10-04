---
id: T0169
title: assets/compat の未来の版のファイルに、未知の VertexOwner を含む物理を足す
status: todo
milestone: M9
crates: [assets/compat, ugoe-format, docs]
depends_on: []
assignee:
---

# T0169: assets/compat の未来の版のファイルに、未知の VertexOwner を含む物理を足す

## 目的

T0140 で物理の未知の `VertexOwner` を読めるようにしたが、互換性テストの「未来の版を模したファイル」(`assets/compat/`) には
未知の `VertexOwner` を含む物理が無い (T0140 の候補)。今の版の読み込みが、未来の版の値で落ちないことを資料で確かめる。

## 受け入れ条件

- [ ] `assets/compat/` の未来の版のファイルに、未知の `VertexOwner` を持つ物理の出力先 (または頂点の持ち主) がある
- [ ] 互換性テストが、そのファイルを読み込めて、該当の物理を仕様どおりに扱う (無視する・警告する) ことを確かめる
- [ ] docs/testing.md#互換性テスト の資料の説明を更新した
- [ ] `just ci` が通る

## 参考

- T0140 のメモ (起票の候補)、docs/testing.md#互換性テスト

## メモ

- T0026 で起票した (元の候補: T0140 の assets/compat)。
