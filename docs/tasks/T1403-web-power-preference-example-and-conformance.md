---
id: T1403
title: "@ugoe/runtime: 見本のページに power-preference の選択と adapterName を出し、適合テストで変えて作り直すシナリオを足す"
status: done
milestone: M8
crates: [bindings/web, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T1403
---

# T1403: @ugoe/runtime: 見本のページと適合テストに power-preference を足す

## 目的

T1401 で `createPuppet` の `powerPreference` と `<ugoe-avatar>` の `power-preference` 属性・`adapterName` を足したが、見本のページ
(`bindings/web/examples/basic`) には選ぶ部品も GPU の名前の表示も無く、本物の wasm で属性を変えて作り直し、描けることは確かめていない
(偽の wasm の単体テストで振る舞いだけを確かめた)。利用者が手で試せるように見本に足し、適合テストに本物の wasm のシナリオを足す。

## 受け入れ条件

- [x] examples/basic に `power-preference` を選ぶ部品 (`default`・`low-power`・`high-performance`) と、今の `adapterName` (と描画の API) の表示を足した。
      変えると作り直され、表示が新しい値になる。bindings/web/README.md#見本-examplesbasic の説明を書き直した
- [x] tests/conformance/web/avatar.js に、本物の wasm の `<ugoe-avatar>` で `power-preference` を変えて作り直し、作り直した後も描ける (画が空でない・
      参照と比べる形は既存のシナリオに合わせる) ことと、`adapterName` が作り直しの後も文字列 (伏せられたら `""`) であることを確かめるシナリオを足した
- [x] 解釈した値が変わらない書き換え (大文字小文字・知らない値 ↔ `default`) では作り直さないことも、本物の wasm で 1 つ確かめた
- [x] `just conformance` (web) が通る
- [x] `just ci` が通る

## 参考

- T1401 のメモ (やったこと・やらなかったこと)、bindings/web/README.md#gpu-の電力の希望・#見本-examplesbasic
- tests/conformance/README.md、tests/conformance/web/avatar.js

## メモ

- T0078 で起票した (元の候補: T1401 (examples/basic に `power-preference` の選択と `adapterName` の表示を足す・tests/conformance/web/avatar.js に
  本物の wasm で `power-preference` を変えて作り直し、描けることを確かめるシナリオを足す))。同じ機能の見本と確かめなので 1 つにした。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- 希望が実際に別の GPU を選ぶか (2 つの GPU を持つ実機) は確かめられない。ヘッドレスの Chrome では同じアダプタのままでよい。
- やったこと (2026-10-08):
  - examples/basic: 「power-preference」の選択 (`default`・`low-power`・`high-performance`) を足し、変えると `<ugoe-avatar>` の属性を書く
    (要素が作り直す)。`load` のたびに `#adapter` に GPU の名前 (`adapterName`。空なら「伏せています」)・描画の API・希望を出し、
    `window.ugoeExample.element` にも `powerPreference`・`adapterName` を置く。`?power-preference=` で両方の例 (要素と `createPuppet`) の
    初めの希望を変えられる (選択肢に無い値は `default`。`createPuppet` が知らない値を受け付けないため)。`plain` に `adapterName` も置く。
    bindings/web/README.md#見本-examplesbasic を書き直した。
  - tests/conformance/web/avatar.js に `power-preference-change` を足した: `low-power` で作り、属性で `high-performance`、プロパティで
    `default` に変えるたびに、新しい canvas・新しい `WebPuppet`・`WebPuppet.create` の 3 つ目の引数がその値・モデルとパラメータの値の
    引き継ぎ・`adapterName` が文字列・描けること (既存の `expectDrawn`)・生きている `WebPuppet` が 1 つを確かめる。続けて
    `default` → `Bogus` → `DEFAULT` → ` default ` では作り直さず (`create` を呼ばず `load` も来ない)、知らない値の警告が 1 度だけを確かめる。
  - 受け入れ条件の「表示が新しい値になる」を自動で確かめるため、tests/conformance/example.mjs の `guarded` の回に、選択を
    `high-performance` に変えて `element.powerPreference` と `#adapter` の表示が新しい値になることの確かめを足した (`crates:` の
    tests/conformance の中)。tests/conformance/README.md (シナリオの表・`example` の説明) も直した。
  - `just conformance --only avatar,example` を 4 回流してすべて通った (avatar 11/11、example guarded・unguarded)。ヘッドレスの Chrome
    (SwiftShader) では希望によらず同じアダプタ (`ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device ...))`) になる。
  - 確かめの確かめ: dist の要素を「`power-preference` の変更で常に作り直す」に書き換えると、`power-preference-change` が
    `default -> Bogus: WebPuppet.create was called` で落ちることを確かめた (元に戻した)。
  - `just ci` が通った。
- やらなかったこと: 2 つの GPU を持つ実機で希望によって GPU が変わるかは確かめていない (環境が無い)。WebGPU での作り直しは
  ヘッドレスの Chrome に WebGPU のアダプタが無いので確かめていない (シナリオは `backend="webgl2"`)。見本を人の目で開いての確かめはしていない
  (example.mjs の自動の確かめだけ)。
- 起票の候補: なし。
