---
id: T0223
title: 参照 (ugoe render) と wasm で圧縮した KTX2 をデコードするか決める (依存の選定と ADR。判断が要る)
status: in-progress
milestone: M9
crates: [docs/adr, ugoe-format, ugoe-headless, ugoe-wasm, tests/conformance]
depends_on: []
assignee: claude-worker-T0223
---

# T0223: 参照 (ugoe render) と wasm で圧縮した KTX2 をデコードするか決める (依存の選定と ADR。判断が要る)

## 目的

T0222 で、ヘッドレス (`ugoe render`) と wasm が非圧縮の KTX2 を読めるようにした。圧縮した KTX2 (BCn・ETC2・ASTC・Basis Universal) は
読めない形式として警告し白で描く (T0222 の候補)。Godot の `ktx2.gd` は BC3 を読むので、組み込み先で見え方が違う。

**判断が要る:** デコードに使う依存 (ライセンスは MIT のクレートに入れられるもの。wasm32 でビルドできるか・大きさ) と、対応する形式の範囲。
ランタイム層に重い依存を入れるので ADR を書く (AGENTS.md)。

## 受け入れ条件

- [ ] 候補の依存 (ライセンス・wasm32・大きさ・保守) と、対応する形式の範囲を比べた ADR の案を書いた (利用者の判断を待つ間は blocked)
- [ ] 判断の後: 決めた形式を参照と wasm でデコードし、適合テストに圧縮した KTX2 のケースがある (できれば Godot の `ktx2.gd` の BC3 も適合テストへ移す)。大きければ実装は別のタスクに分けて、その案をメモに書く
- [ ] `just ci`・`just wasm-check`・`just conformance` が通る

## 参考

- T0222・T0448 のメモ、crates/ugoe-format/src/image.rs、tests/conformance/README.md

## メモ

- T0026 で起票した (元の候補: T0222 の圧縮した KTX2)。T0222 の残りの候補 (アプリの訳文・スタジオの仮想カメラ) は T0934・T0836 に入れた。
