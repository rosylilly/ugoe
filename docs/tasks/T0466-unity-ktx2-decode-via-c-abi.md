---
id: T0466
title: "Unity: KTX2 のテクスチャを C ABI のデコーダで読む (判断が要る)"
status: in-progress
milestone: M9
crates: [ugoe-ffi, bindings/unity, docs]
depends_on: [T0464]
assignee: claude-worker-T0466
---

# T0466: Unity: KTX2 のテクスチャを C ABI のデコーダで読む (判断が要る)

## 目的

T0223 (ADR 0030) で、参照 (`ugoe render`)・wasm・Godot は圧縮した KTX2 (BC1〜BC5・BC7) を `ugoe_format::image` でデコードするようになった。
Unity は KTX2 をまったくデコードせず、白で代用している (T0223 の候補)。C ABI に `ugoe_format::image` のデコードを出せば、Unity も PNG 以外を同じに読める。

**判断が要る:** C ABI に足す関数の形 (テクスチャの番号でデコード済みの RGBA を返す・バンドルの読み込みでデコードまで済ませる、など) と、
共有ライブラリの大きさへの影響。同じ C ABI と Unity を触る T0464 の後に着手する。

## 受け入れ条件

- [ ] C ABI の関数の形の案と推奨をメモに書き、利用者の判断を得た
- [ ] Unity が KTX2 (圧縮していないもの・BC1〜BC5・BC7) を参照と同じ画素で読む。読めない形式は今のとおり警告と白にする
- [ ] C ABI のテスト (Rust) と Unity のテストがあり、ヘッダ・`just refs`・bindings/unity の README を揃えた
- [ ] 適合テスト (`just conformance`) で Unity が使う C ABI の経路の KTX2 のケースを比べた (実際の Unity で確かめられなければメモに)
- [ ] `just ci` が通る

## 参考

- T0223 のメモ、ADR 0030
- crates/ugoe-ffi/README.md、bindings/unity/README.md

## メモ

- T0032 で起票した (元の候補: T0223 (Unity))。
