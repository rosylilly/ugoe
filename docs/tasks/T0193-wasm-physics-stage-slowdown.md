---
id: T0193
title: "性能: wasm (ブラウザ) で物理の段がネイティブの約 2.5 倍遅い理由を調べる"
status: todo
milestone: M7
crates: [ugoe-physics, bindings/web, docs]
depends_on: []
assignee:
---

# T0193: 性能: wasm (ブラウザ) で物理の段がネイティブの約 2.5 倍遅い理由を調べる

## 目的

T0192 の計測のページ (`bindings/web/examples/timing/`) で、物理を止めた `update` は wasm とネイティブでほぼ同じ (生成のモデルで 48µs と 47µs) だが、
物理の段だけが wasm で約 21µs、ネイティブで約 8µs と約 2.5 倍遅かった (docs/testing.md#1-フレームの評価)。
三角関数などの浮動小数点の関数が wasm で遅い見込みだが、確かめていない。理由を突き止め、安く直せるなら直す。

1ms の目標には遠い (wasm の `update` 全体で約 0.07ms) ので急がない。

## 受け入れ条件

- [ ] 物理の段のどこに時間が掛かっているかを、ブラウザ (ヘッドレスの Chrome。T0192 の `measure.mjs`) とネイティブで比べて書いた
      (例: 物理の 1 段を分けて測る・ブラウザの profiler・関数を差し替えた比較)。見込み (三角関数などの `libm` の関数) が当たっていたかを書く
- [ ] 直す価値がある (安く、結果が変わらない) なら直した。物理の結果 (`ugoe-physics` の既存のテストとフィクスチャの数値) は許容誤差の中で変わらない。
      直さないなら理由をメモに書いた
- [ ] 直したら、計測のページで前と後を測り、docs/testing.md#1-フレームの評価 の表を書き直した
- [ ] ランタイム層を触ったら `just wasm-check` が通る
- [ ] `just ci` が通る

## 参考

- T0192 のメモ (測った値・確かめていないこと)、docs/testing.md#1-フレームの評価・#性能の計測
- crates/ugoe-physics/README.md、docs/spec/physics.md
- bindings/web/examples/timing/ (計測のページと `measure.mjs`)

## メモ

- T0078 で起票した (元の候補: T0192 (wasm の物理の段がネイティブの約 2.5 倍遅い理由を調べる。急がない))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- 結果の決定性 (docs/spec/evaluation.md・physics.md) を崩す変更 (近似の関数に置き換えるなど) は、結果が変わるので仕様の判断になる。
  その場合は案をメモに書いて止まる。
