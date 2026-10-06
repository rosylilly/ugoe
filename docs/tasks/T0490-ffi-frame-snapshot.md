---
id: T0490
title: "C ABI: 毎フレームの描画の状態をまとめて受け取る形に置き換え、描画要素ごとの関数を廃止する"
status: todo
milestone: M11
crates: [ugoe-ffi, ugoe, bindings/unity, tests/conformance, docs]
depends_on: [T0489]
assignee:
---

# T0490: C ABI: 毎フレームの描画の状態をまとめて受け取る形に置き換え、描画要素ごとの関数を廃止する

## 目的

毎フレームの描画の状態は、描画要素ごとの関数 (`ugoe_puppet_drawable_state`・`ugoe_puppet_positions` など) で読んでいる。
そのため、境界を越える呼び出しが重いホスト (cgo・マネージドな言語) では、呼び出しが描画要素の数に比例して増える。
毎フレームの状態は 1 回の呼び出しでまとめて受け取る形だけにし、描画要素ごとの関数は廃止する。読み込みのときに 1 回読む情報 (UV・インデックス・
テクスチャ・マスクの参照など) は描画要素ごとのまま残す ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md#毎フレームの描画の状態はまとめて渡す))。
描き方 (描画順に従って各描画要素を描く) は変えない。

## 受け入れ条件

- [ ] まとめて受け取る形で、すべての描画要素の状態 (今の `UgoeDrawableState` の中身: 表示・不透明度・色・合成モード・マスクの効き具合・
      描画順の値・変更のフラグ)、描画順、描画順が変わったか、頂点の位置を読める
- [ ] 頂点の位置は、描画要素ごとの範囲 (先頭と数) を持つ 1 つの連続した配列。借用の形 (次の `ugoe_puppet_update`・着せ替え・破棄まで有効) と、
      ホストのバッファへのコピーの形 (足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で要る数) の両方がある
- [ ] 変更のフラグで、変わっていない描画要素の位置を写さずに済む
- [ ] `ugoe_puppet_drawable_state`・`ugoe_puppet_positions`・`ugoe_puppet_copy_positions`・`ugoe_puppet_render_order`・
      `ugoe_puppet_render_order_changed` を消し、`ABI_VERSION` を上げた (T0489 と同じリリースの間なら、上げるのは 1 回でよい)
- [ ] 置き換える前の関数と同じ値になることを、消す前にテストで確かめた (見本のモデルで、いくつかのフレームを比べる)
- [ ] リポジトリの中の利用者を移した: C の見本 (`examples/c/sample.c`)、適合テスト (`tests/conformance/c/runner.c`)、
      Unity (`Native.cs` と毎フレームの更新。1 フレームの呼び出しの数が減ったことをメモに書く)。`just refs` が通る
- [ ] SDK (`ugoe`) に連続した配列の形が要るなら足した (`ugoe-runtime` の `DrawableState` の形は変えない)
- [ ] ヘッダ (`just ffi-header`)・crates/ugoe-ffi/README.md・docs/portability.md#c-abi-ugoe-ffi-の規約 (頂点データの借用とコピーの説明) を更新した。
      `just ffi-check` と Unity のテストが通る
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、crates/ugoe-ffi/src/drawable.rs、docs/spec/rendering.md#描画の最適化、
  [ADR 0016](../adr/0016-draw-batching.md)

## メモ

- 2026-10-06 起票。同日、利用者の判断で「足すだけ」から「置き換えて廃止する」に改めた。
- T0489 と同じファイル (C ABI・C の見本・適合テスト・Unity) を触るので、T0489 の後に行う。
- パーツの不透明度 (`ugoe_puppet_part_opacity` など) は描画の状態ではないので対象外。
