---
id: T1403
title: "@ugoe/runtime: 見本のページに power-preference の選択と adapterName を出し、適合テストで変えて作り直すシナリオを足す"
status: todo
milestone: M8
crates: [bindings/web, tests/conformance, docs]
depends_on: []
assignee:
---

# T1403: @ugoe/runtime: 見本のページと適合テストに power-preference を足す

## 目的

T1401 で `createPuppet` の `powerPreference` と `<ugoe-avatar>` の `power-preference` 属性・`adapterName` を足したが、見本のページ
(`bindings/web/examples/basic`) には選ぶ部品も GPU の名前の表示も無く、本物の wasm で属性を変えて作り直し、描けることは確かめていない
(偽の wasm の単体テストで振る舞いだけを確かめた)。利用者が手で試せるように見本に足し、適合テストに本物の wasm のシナリオを足す。

## 受け入れ条件

- [ ] examples/basic に `power-preference` を選ぶ部品 (`default`・`low-power`・`high-performance`) と、今の `adapterName` (と描画の API) の表示を足した。
      変えると作り直され、表示が新しい値になる。bindings/web/README.md#見本-examplesbasic の説明を書き直した
- [ ] tests/conformance/web/avatar.js に、本物の wasm の `<ugoe-avatar>` で `power-preference` を変えて作り直し、作り直した後も描ける (画が空でない・
      参照と比べる形は既存のシナリオに合わせる) ことと、`adapterName` が作り直しの後も文字列 (伏せられたら `""`) であることを確かめるシナリオを足した
- [ ] 解釈した値が変わらない書き換え (大文字小文字・知らない値 ↔ `default`) では作り直さないことも、本物の wasm で 1 つ確かめた
- [ ] `just conformance` (web) が通る
- [ ] `just ci` が通る

## 参考

- T1401 のメモ (やったこと・やらなかったこと)、bindings/web/README.md#gpu-の電力の希望・#見本-examplesbasic
- tests/conformance/README.md、tests/conformance/web/avatar.js

## メモ

- T0078 で起票した (元の候補: T1401 (examples/basic に `power-preference` の選択と `adapterName` の表示を足す・tests/conformance/web/avatar.js に
  本物の wasm で `power-preference` を変えて作り直し、描けることを確かめるシナリオを足す))。同じ機能の見本と確かめなので 1 つにした。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- 希望が実際に別の GPU を選ぶか (2 つの GPU を持つ実機) は確かめられない。ヘッドレスの Chrome では同じアダプタのままでよい。
