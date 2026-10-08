---
id: T1405
title: "Godot: ノードに形が変わったか (と書き換えた回数) を出し、適合テストと smoke で変わらないフレームの書き換えを省いたことを確かめる (判断が要る)"
status: todo
milestone: M8
crates: [bindings/godot, tests/conformance, docs]
depends_on: []
assignee:
---

# T1405: Godot: ノードに形が変わったか (と書き換えた回数) を出し、書き換えを省いたことを確かめる (判断が要る)

## 目的

T1404 で、Godot の `Renderer::draw` (bindings/godot/src/render.rs) は `Puppet::changed` が false で、最初の描画でもマスクの解像度の変化でもなければ、
頂点のバッファ・draw index・uniform を書き換えずに戻るようになった。ただし、Godot で実際に書き換えを省いたことは確かめていない
(ヘッドレスの Godot の smoke と適合テストは通るが、書き換えの回数を数えていない。T1404 のメモ)。適合テストの `changed` (各 `update` の後の
`Puppet::changed`) も、C ABI・Unity とだけ比べ、Godot とは比べていない。ノードに口を足して、smoke と適合テストで確かめる。

## 判断が要る

GDScript から見えるノードの公開の口が増える。

- **案 A (推奨): 2 つ足す。** `is_changed() -> bool` (直前の `update`・`follow` の後の `Puppet::changed`。C ABI の `ugoe_puppet_changed`・
  Unity の `Puppet.Changed` と同じ意味と名前) と、`get_redraw_count() -> int` (頂点のバッファなどを書き換えた回数。読み込みで 0 に戻す)。
  `is_changed` で適合テストの `changed` を C ABI・Unity と同じく参照と比べ、`get_redraw_count` で smoke が書き換えを省いたことを数える。
- 案 B: `is_changed()` だけ。書き換えを省いたかは `changed` が false のフレームで描かないことを信じる (smoke で回数は確かめない)。
- 案 C: 回数だけ (`get_redraw_count`。デバッグ用と書く)。適合テストとは比べない。
- 名前: wasm に合わせた `is_shape_changed` もありうるが、wasm の `shape_changed` は描くまで積む (意味が違う) ので、`Puppet::changed` に合わせた
  `is_changed` を推奨する (T1404 で C ABI を `ugoe_puppet_changed` にしたのと同じ理由)。

## 受け入れ条件

- [ ] 判断 (足す口と名前) をメモに書いた
- [ ] (A・B) ノードに `is_changed()` を足し、bindings/godot/README.md#使い方-gdscript に書いた。適合テスト (tests/conformance) の Godot の側が結果に
      `changed` を書き、参照 (`ugoe::Puppet::changed`) と完全に一致することを比べる (tests/conformance/README.md の手順 4 を直す)
- [ ] (A・C) `get_redraw_count()` を足し、`example/tests/smoke.gd` で、待機モーション・目パチ・呼吸の無いモデルを読み込んで数フレーム進めると
      最初の 1 回の後は回数が増えないこと、パラメータを変えると 1 回増えること、`mask_scale` を変えると増えることを確かめた
- [ ] 直す前の `Renderer::draw` (変わらないフレームでも書き換える形) にすると smoke が落ちることを確かめた (一時的な変更で。コミットしない)
- [ ] `just godot-check` と `just conformance --only godot` が通る
- [ ] `just ci` が通る

## 参考

- T1404 のメモ (確かめていないこと・起票の候補)、docs/spec/evaluation.md#見た目の変化
- bindings/godot/src/render.rs (`Renderer::draw`・`PendingChanges`)・src/node.rs、bindings/godot/example/tests/smoke.gd、bindings/godot/README.md#検査
- tests/conformance/README.md (結果の `changed`)

## メモ

- T0080 で起票した (元の候補: T1404 (Godot のノードに `is_shape_changed()` か描き直した回数を出し、適合テストと smoke で書き換えを省いたことを確かめる))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- 実際の GPU のドライバでバッファの転送が減ることは測らない (書き換えの呼び出しの回数だけを確かめる)。
