---
id: T0490
title: "C ABI: 1 フレーム分の描画の状態と頂点をまとめて受け取る関数"
status: todo
milestone: M11
crates: [ugoe-ffi, ugoe, tests/conformance, docs]
depends_on: []
assignee:
---

# T0490: C ABI: 1 フレーム分の描画の状態と頂点をまとめて受け取る関数

## 目的

描画の状態は描画要素ごとの関数 (`ugoe_puppet_drawable_state`・`ugoe_puppet_positions` など) で取るので、境界を越える呼び出しが重いホスト
(cgo・マネージドな言語) では呼び出しが描画要素の数に比例して増える。1 回の呼び出しで 1 フレーム分をまとめて渡す関数を足す
([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md#1-フレーム分の描画の状態をまとめて渡す))。

## 受け入れ条件

- [ ] すべての描画要素の状態 (`UgoeDrawableState` と同じ中身: 表示・不透明度・色・合成モード・マスク) と描画順、変更のフラグをまとめて受け取れる
- [ ] 頂点の位置を、描画要素ごとの範囲 (先頭と数) を持つ 1 つの連続した配列で受け取れる。借用の形 (次の `ugoe_puppet_update` まで有効) と
      ホストのバッファへのコピーの形 (足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で要る数) の両方
- [ ] 変更の無い描画要素を飛ばせる (変更のフラグで、位置を写さずに済む)
- [ ] 描画要素ごとの関数と同じ値になることをテストし、適合テスト (C ABI) をまとめた関数でも比べる
- [ ] ヘッダ・C の見本・crates/ugoe-ffi/README.md を更新した。足しただけなので ABI の版は上げない
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、crates/ugoe-ffi/src/drawable.rs、docs/spec/rendering.md#描画の最適化

## メモ

- 2026-10-06 起票。SDK の側に連続した配列の形が無ければ `ugoe` に足す (`ugoe-runtime` の `DrawableState` の形は変えない)。
  後続の候補: Unity・Godot のバインディングをまとめた関数に移して、1 フレームの呼び出しの数を減らす。
