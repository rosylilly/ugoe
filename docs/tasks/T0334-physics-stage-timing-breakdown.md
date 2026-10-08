---
id: T0334
title: "性能: 物理の段の内訳 (1 段目の評価・物理・2 段目) を測る仕組みをリポジトリに置くか決める (判断が要る)"
status: todo
milestone: M7
crates: [ugoe-physics, bindings/web, docs]
depends_on: []
assignee:
---

# T0334: 性能: 物理の段の内訳を測る仕組みをリポジトリに置くか決める (判断が要る)

## 目的

T0193 は、物理の段のどこに時間が掛かるかを、リポジトリの外 (scratchpad) の作業用のクレートで測った: `ModelInstance` と `PhysicsSystem` を直接使い、
`update_first_stage`・`update_instance`・`update` の時間を、ネイティブ (`Instant`) と wasm (JS の `performance.now()` を import) の同じコードで測り、
wasm は関数名の節を残したビルドをヘッドレスの Chrome の CPU プロファイラ (CDP の `Profiler`) で関数ごとに数えた (T0193 のメモの「測り方と条件」)。
これはリポジトリに無いので、次に物理の速さを調べる人 (T0333 など) は作り直す必要がある。この計測をリポジトリに置くか決め、置くなら置く。

## 判断が要る

- **案 A (推奨): ネイティブの段ごとの計測を `physics_timing` の例に足す。** `ugoe-physics` の例 (crates/ugoe-physics/examples/physics_timing.rs) に、
  生成のモデル (と `avatar.ugoe` があればそれ) で 1 段目の評価・物理・2 段目を分けて測る行を足す。wasm の段ごとの計測とプロファイラの手順は
  docs/testing.md#1-フレームの評価 に「調べ方」として書くだけにする (wasm の計測のページは `WebPuppet` の公開の口を通るので段を分けられない)。
- 案 B: A に加え、wasm でも段を分けて測る。計測のページ (bindings/web/examples/timing) 用に、段ごとの時間を返す口を ugoe-wasm の計測用の
  ビルドだけに足す (feature で分け、公開の `@ugoe/runtime` には入れない)。wasm の段の差を `just perf-wasm` でも見られるが、ugoe-wasm と
  ビルドの手順が増える (`crates:` に ugoe-wasm が要る)。
- 案 C: 置かない。docs/testing.md に T0193 の測り方を文で残すだけ。

推奨は A。物理の速さを調べるときにまず要るのはネイティブの段の内訳で、wasm との差は T0193 で理由 (`libm` の平方根) が分かり、
残りは wasm の一般的な遅さの範囲だった。wasm の段の内訳が要るのはまれなので、手順を文で残せば足りる。

## 受け入れ条件

- [ ] 判断をメモに書いた
- [ ] (A・B) `physics_timing` に段ごとの計測を足した (例の先頭のコメントに各行の意味を書く)。`cargo run --release -p ugoe-physics --example physics_timing`
      の出力の例をメモに書いた。ランタイム層のライブラリのコードは変えない (例だけ)
- [ ] (A・B・C) docs/testing.md#1-フレームの評価 に、段ごとの調べ方 (ネイティブの例、wasm の CPU プロファイラの手順) を書いた
- [ ] (B) 計測用のビルドの口を足し、`measure.mjs` が段ごとの時間を出す。公開の `@ugoe/runtime` の型 (`just wasm-types`) が変わらない
- [ ] `just ci` が通る

## 参考

- T0193 のメモ (測り方と条件・起票の候補)、T0190・T0192 のメモ、docs/testing.md#1-フレームの評価・#性能の計測
- crates/ugoe-physics/examples/physics_timing.rs、crates/ugoe/examples/puppet_timing.rs、bindings/web/examples/timing/

## メモ

- T0080 で起票した (元の候補: T0193 (物理の段ごとの計測を `physics_timing` の例か計測のページに入れるか決める))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- T0333 と同じ `ugoe-physics` を触るが、こちらは例と文書だけ (案 A)。並べて進めてよい。
- 判断 (2026-10-08): 利用者が案 A (physics_timing の例にネイティブの段ごとの時間を足し、wasm は測り方を文書に書くだけ) を選んだ。
