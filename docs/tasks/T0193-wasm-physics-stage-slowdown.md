---
id: T0193
title: "性能: wasm (ブラウザ) で物理の段がネイティブの約 2.5 倍遅い理由を調べる"
status: done
milestone: M7
crates: [ugoe-physics, bindings/web, docs]
depends_on: []
assignee: claude-worker-T0193
---

# T0193: 性能: wasm (ブラウザ) で物理の段がネイティブの約 2.5 倍遅い理由を調べる

## 目的

T0192 の計測のページ (`bindings/web/examples/timing/`) で、物理を止めた `update` は wasm とネイティブでほぼ同じ (生成のモデルで 48µs と 47µs) だが、
物理の段だけが wasm で約 21µs、ネイティブで約 8µs と約 2.5 倍遅かった (docs/testing.md#1-フレームの評価)。
三角関数などの浮動小数点の関数が wasm で遅い見込みだが、確かめていない。理由を突き止め、安く直せるなら直す。

1ms の目標には遠い (wasm の `update` 全体で約 0.07ms) ので急がない。

## 受け入れ条件

- [x] 物理の段のどこに時間が掛かっているかを、ブラウザ (ヘッドレスの Chrome。T0192 の `measure.mjs`) とネイティブで比べて書いた
      (例: 物理の 1 段を分けて測る・ブラウザの profiler・関数を差し替えた比較)。見込み (三角関数などの `libm` の関数) が当たっていたかを書く
- [x] 直す価値がある (安く、結果が変わらない) なら直した。物理の結果 (`ugoe-physics` の既存のテストとフィクスチャの数値) は許容誤差の中で変わらない。
      直さないなら理由をメモに書いた
- [x] 直したら、計測のページで前と後を測り、docs/testing.md#1-フレームの評価 の表を書き直した
- [x] ランタイム層を触ったら `just wasm-check` が通る
- [x] `just ci` が通る

## 参考

- T0192 のメモ (測った値・確かめていないこと)、docs/testing.md#1-フレームの評価・#性能の計測
- crates/ugoe-physics/README.md、docs/spec/physics.md
- bindings/web/examples/timing/ (計測のページと `measure.mjs`)

## メモ

- T0078 で起票した (元の候補: T0192 (wasm の物理の段がネイティブの約 2.5 倍遅い理由を調べる。急がない))。
- M7 の後続 (docs/roadmap.md の M7 の性能の行)。M7 の完了の条件には含めない。
- 結果の決定性 (docs/spec/evaluation.md・physics.md) を崩す変更 (近似の関数に置き換えるなど) は、結果が変わるので仕様の判断になる。
  その場合は案をメモに書いて止まる。

### 結果 (claude-worker-T0193)

**理由:** 物理の段の約半分は平方根 (`libm::sqrtf`) だった。安定版の Rust の `libm` (0.2.16) は wasm32 で `f32.sqrt` の命令を使えず
(`unstable-intrinsics` が要る)、ソフトウェア (Goldschmidt の反復) で求める。x86_64 では SSE の命令を使うのでネイティブだけ速かった。
呼ぶのは衝突の判定 (`collider.rs` の点と線分の距離。掃引と二分法の 1 回ごとに数個) で、1 フレームに数千回。さらに `@ugoe/runtime` の
wasm は `libm` を `opt-level = "z"` でビルドしていた (`HOT_CRATES` に無かった)。見込みの三角関数は 1 割未満で、半分だけ当たっていた。
詳しい数は docs/testing.md#1-フレームの評価。

**やったこと:**

- `ugoe_core::math::sqrt` を、`std` があれば `std` の `f32::sqrt` (命令) にした。`std` が無ければ今までどおり `libm`。
  平方根は IEEE で正しく丸めると決まっているのでビット列は同じ (単体テスト `sqrt_matches_libm_bit_for_bit`)。
- `ugoe-physics` の衝突の判定の長さを `math::sqrt(v.length_squared())` で求める (`glam` の `length` は中で `libm` の平方根を呼ぶので)。
- `bindings/web/scripts/build.mjs` の `HOT_CRATES` に `libm` を足した (wasm は 4,477,972 → 4,482,149 バイト、gzip は 1,359,658 → 1,358,996)。
- 文書: docs/testing.md (表の書き直し・理由の節・`just perf` の節の手元の値)、ADR 0031 に追記 (平方根の例外)、docs/portability.md、
  crates/ugoe-core/README.md、crates/ugoe-wasm/README.md、bindings/web/README.md、docs/roadmap.md。

**確かめたこと (結果が変わらない):**

- 物理の golden (`golden_tests.rs`) と `ugoe-physics` の既存のテストはそのまま通る。
- 調べるための作業用のクレート (リポジトリの外。下の「測り方」) で、生成のモデルと `avatar.ugoe` を 1,200 フレーム進めた頂点と
  パラメータのハッシュが、直す前と後・ネイティブと wasm の 4 通りですべて同じ (`e9b9cf0fb09082ce`・`ab157425c0847d11`)。
- wasm (Node の V8) で、`std` の `f32::sqrt` と `libm::sqrtf` をすべての `f32` のビット列 (2^32 通り) で比べて不一致 0。
- `just conformance` は全部 ok (直した後、rebase の前)。`just perf` も ok (ネイティブと wasm。wasm の生成のモデルの `update` 66µs / 上限 300µs)。
  ネイティブの時間は変わらない (x86_64 の `libm` はもともと SSE の命令)。

**測り方と条件:** AMD Ryzen 9 7900・WSL2、ヘッドレスの Chrome 154。ほかのワーカーのビルドが並行していたので、load average が
3〜4 に下がるのを待って測った (表の値。16:38〜16:40)。それより前の load average 10〜27 の回も同じ傾向。

- 計測のページ (`measure.mjs --runs 5`) で、直す前の dist と後の dist を交互に 2 度ずつ測った。生成のモデルの `update` は
  前 71.6・75.4µs → 後 61.4・61.6µs、物理の段 (物理を止めた `update` との差) は約 20µs → 約 10µs (ネイティブ約 7µs)。
- 段の内訳は、リポジトリの外 (scratchpad) の作業用のクレートで測った: `ModelInstance` と `PhysicsSystem` を直接使い、
  `update_first_stage`・`update_instance`・`update` の時間を、ネイティブ (`Instant`) と wasm (JS の `performance.now()` を import) の
  同じコードで測る。wasm は関数名の節を残してビルドし、ヘッドレスの Chrome の CPU プロファイラ (CDP の `Profiler`) で
  `update_instance` の下 (`update_first_stage` を除く) の自己時間を関数ごとに数えた。直す前は `libm` の `sqrtf` (と中の
  `goldschmidt`) が 48%、直した後は 2% で、残りの大半は `WorldCollider::blocks` と掃引の算術。
  ネイティブは perf が WSL2 の カーネルに無く使えなかったので、時間を分けて測るだけ。
- 関数 1 回の時間 (ns、wasm / ネイティブ): `sqrtf` 8.6 (libm) → 0.94 (命令) / 0.88、`sincosf` 6.6 (opt z) → 4.3 (opt 3) / 2.3、
  `atan2f` 6.1 / 3.0、`powf` 25.8 / 20.3。

**やらなかったこと:**

- `glam` の中の平方根 (`length`・`normalize`・`try_normalize`) は `libm` のまま。`glam` の feature `libm` が平方根も `libm` にするため。
  鎖の積分 (`chain.rs` の `try_normalize`) などで呼ぶが、プロファイルでは 2% ほどなので触らなかった。
- `floor`・`ceil`・`trunc`・`round` も wasm32 の `libm` はソフトウェアだが、反復しない安い実装で、プロファイルにほとんど出ないので変えていない。
- 残りの差 (物理そのものでネイティブの約 1.35 倍、1 段目の評価で約 1.6 倍) は wasm の一般的な遅さの範囲で、安い手が見つからなかった。
- 衝突の判定の平方根を減らす改善 (下の候補) は、wasm とネイティブの差ではなく両方に効く話なので、このタスクではしていない。
- `crates:` の外 (`ugoe-core`・`ugoe-wasm` の README) を小さく変えた。`ugoe_core::math::sqrt` の中身の変更は公開 API を変えず、
  ADR 0031 の目的 (同じ入力から同じビット列) も保つので、ADR を置き換えずに追記にした。ADR の扱いとして足りなければ判断してほしい。

**起票の候補:**

- 衝突の判定の平方根を減らす: `WorldCollider::blocks` の親の距離 (`point_segment_distance(parent, …)`) は掃引の間で変わらないので、
  `constrain_segment` の始めに衝突体ごとに 1 度求めれば、1 回の判定あたり 1 つ減る (結果のビット列は変わらない)。物理そのものの約半分が
  この算術なので、ネイティブ・wasm とも 1〜2 割速くなる見込み。急がない。
- 作業用の段ごとの計測 (上の「測り方」) を `ugoe-physics` の例 (`physics_timing`) や計測のページに入れるか。今は scratchpad にだけある。
