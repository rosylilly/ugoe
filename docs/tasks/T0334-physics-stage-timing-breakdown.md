---
id: T0334
title: "性能: 物理の段の内訳 (1 段目の評価・物理・2 段目) を測る仕組みをリポジトリに置くか決める (判断が要る)"
status: done
milestone: M7
crates: [ugoe-physics, bindings/web, docs]
depends_on: []
assignee: claude-worker-T0334
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

- [x] 判断をメモに書いた
- [x] (A・B) `physics_timing` に段ごとの計測を足した (例の先頭のコメントに各行の意味を書く)。`cargo run --release -p ugoe-physics --example physics_timing`
      の出力の例をメモに書いた。ランタイム層のライブラリのコードは変えない (例だけ)
- [x] (A・B・C) docs/testing.md#1-フレームの評価 に、段ごとの調べ方 (ネイティブの例、wasm の CPU プロファイラの手順) を書いた
- [ ] (B) 計測用のビルドの口を足し、`measure.mjs` が段ごとの時間を出す。公開の `@ugoe/runtime` の型 (`just wasm-types`) が変わらない
- [x] `just ci` が通る

## 参考

- T0193 のメモ (測り方と条件・起票の候補)、T0190・T0192 のメモ、docs/testing.md#1-フレームの評価・#性能の計測
- crates/ugoe-physics/examples/physics_timing.rs、crates/ugoe/examples/puppet_timing.rs、bindings/web/examples/timing/

## メモ

- T0080 で起票した (元の候補: T0193 (物理の段ごとの計測を `physics_timing` の例か計測のページに入れるか決める))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- T0333 と同じ `ugoe-physics` を触るが、こちらは例と文書だけ (案 A)。並べて進めてよい。
- 判断 (2026-10-08): 利用者が案 A (physics_timing の例にネイティブの段ごとの時間を足し、wasm は測り方を文書に書くだけ) を選んだ。

### 結果 (claude-worker-T0334)

**やったこと (案 A):**

- `crates/ugoe-physics/examples/physics_timing.rs`: 後半に `dynamic_hair` (fixtures) の段の内訳を足した。`first stage`
  (`update_first_stage` だけ。別の回で `update_instance` の前に呼んで測る)・`update_instance` (1 段目を含む)・`physics itself` (両者の差)・
  `second stage` (`ModelInstance::update`)・`frame`。各行の意味は例の先頭のコメント。`BATCH` ずつの平均の最も速いものと中央値を出すようにした
  (前は最も速いものだけ)。
- `crates/ugoe/examples/puppet_timing.rs` (`crates:` の外。例だけ): 生成のモデルと `avatar.ugoe` の同じ段の内訳 (`physics stages`) を足した。
  入力の書き込みを `drive_parameters` に分けて共有。`--check` の判定と上限は変えていない。
- 文書: docs/testing.md (性能の計測の表に `physics_timing` の行、`puppet_timing` の行に段の内訳、#1-フレームの評価 に「物理の段の内訳の
  調べ方」: ネイティブの例と手元の値、wasm の作業用クレートと CDP の `Profiler` の手順)、crates/ugoe-physics/README.md、crates/ugoe/README.md。
- ライブラリのコードと公開 API は変えていない (例と文書だけ)。

**案 A からずれたところ:** 案 A は生成のモデルと `avatar.ugoe` も `physics_timing` で測る想定だったが、`ugoe-physics` は `.ugoe` を読めない。
`ugoe-format` を dev の依存に足すと `just publish-check` が `dependency cycle among published crates: ugoe, ugoe-format, ugoe-physics, ugoe-render`
で落ちる (`ugoe-format` が dev で `ugoe-physics` に依存し、crates.io は dev の依存も先に公開されていることを求める)。生成のモデルも
`ugoe` の例 (`puppet_timing.rs` の `medium_model`) にしか無い。そこで大きなモデルの段の内訳は `puppet_timing` に置き、`physics_timing` は
fixtures の `dynamic_hair` だけを測る。`ugoe` は `crates:` に無いが、例だけの小さな変更なので行った。

**出力の例** (AMD Ryzen 9 7900・WSL2、ほかのワーカーのビルドが並行して load average 9 前後):

```
$ cargo run --release -p ugoe-physics --example physics_timing
per frame (us), batches of 1000 frames out of 20000: best / median
  physics_hair (update)                0.15 /     0.15
  physics_hair + 100 params            0.15 /     0.15
  physics_relay (update)               0.22 /     0.22
  dynamic_hair (update_instance)       0.78 /     0.81

dynamic_hair stages per frame (us): best / median
  first stage                          0.16 /     0.16
  update_instance                      0.79 /     1.22
  physics itself (difference)          0.63 /     1.06
  second stage (update)                0.23 /     0.41
  frame                                1.02 /     1.65

$ cargo run --release -p ugoe --example puppet_timing   (末尾の段の内訳だけ)
physics stages per frame (us), without motion and effects: best / median
  generated    first stage                    2.9 /       3.1
  generated    update_instance                8.9 /       9.6
  generated    physics itself                 6.0 /       6.5
  generated    second stage                  43.7 /      44.3
  avatar.ugoe  first stage                    1.7 /       1.7
  avatar.ugoe  update_instance                2.1 /       2.2
  avatar.ugoe  physics itself                 0.4 /       0.5
  avatar.ugoe  second stage                  15.5 /      15.8
```

生成のモデルの 1 段目 2.9µs・物理そのもの 6.0µs は、T0193 が作業用のクレートで測った値 (2.9・6.3) と合う。

**やらなかったこと:**

- wasm の段の内訳の計測 (案 B)。手順を docs/testing.md に書いただけ。手順のうち wasm のビルドの細部 (`strip = false`・
  `--remove-name-section` を付けない) は T0193 のメモと wasm-bindgen の既定から書いたもので、この作業では流し直していない。
- `just wasm-check` は流していない (ランタイム層のライブラリのコードは変えていない)。

**起票の候補:** なし。

