---
id: T0486
title: "<ugoe-avatar>: 読み込みの途中に GPU を失ったとき、作り直せるのに error を送らない (判断が要る)"
status: todo
milestone: M9
crates: [bindings/web, docs]
depends_on: []
assignee:
---

# T0486: <ugoe-avatar>: 読み込みの途中に GPU を失ったとき、作り直せるのに error を送らない (判断が要る)

## 目的

T0484 の適合テストで、`<ugoe-avatar>` が読み込み (`load_bundle`) や `#start` のモデルの適用の途中に GPU を失うと、`LoadError` (`device_lost`) の
`error` イベントを送った後に作り直して読み直すことが分かった (T0484 の候補。バグ寄り)。利用者には作り直せたのに `error` が届く。

**判断が要る:** 案:

- A. 読み込み・モデルの適用で `device_lost` を受けたら `error` を送らず、ふつうの喪失として作り直しを待つ (諦めたときだけ `error`)。
- B. `webglcontextlost` などの喪失の知らせを待ってから、喪失なら A、そうでなければ `error` を送る。
- C. 今のまま (`error` を送ってから作り直す) にして、README に「`device_lost` の `error` の後に作り直されることがある」と書く。

推奨は、`@ugoe/player` の `PuppetSurface` (T0946) の扱いを調べて、そろう案を書く。

## 受け入れ条件

- [ ] 案と推奨をメモに書き、利用者の判断を得た
- [ ] 決めた形で直し、偽物の WebPuppet で「読み込みの途中に喪失 → 作り直して読み直す」の順序のテストが `avatar.test.ts` にある
      (C なら文書だけ)
- [ ] bindings/web/README.md (イベントの表) を直した
- [ ] `just ci` と、T0484 の適合テスト (`just conformance --only avatar`) が通る

## 参考

- T0484 のメモ (内訳・起票の候補)、T0946 (`GpuLostRepeatedlyError`)
- bindings/web/src (`<ugoe-avatar>`)、bindings/web/README.md

## メモ

- T0041 で起票した (元の候補: T0484 (`load_bundle` の途中の喪失))。
