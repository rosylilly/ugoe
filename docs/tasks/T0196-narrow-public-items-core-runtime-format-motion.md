---
id: T0196
title: "内部の整理: ugoe-core・ugoe-runtime・ugoe-format・ugoe-motion の、外で使わない公開の項目とモジュールを狭める"
status: done
milestone: M9
crates: [ugoe-core, ugoe-runtime, ugoe-format, ugoe-motion, docs]
depends_on: []
assignee: claude-worker-T0196
---

# T0196: 公開の範囲を狭める (core・runtime・format・motion)

## 目的

T0195 で、crates.io に出す 8 つのクレートの公開の項目を洗い出し、外で使わず型にも出ない項目と、中身をすべて根で再公開している
モジュールを見つけた。約束のためではなく内部の整理として (利用者の方針 2026-10-11。docs/release.md#公開の範囲)、
変えやすく読みやすい形にするために狭める。4 つのクレートはどれも「影響が無い」(外で使う所が無い) ものだけなので 1 つにまとめた。
物理は量が多いので T0336 に分けた。

## 受け入れ条件

- [x] ugoe-core: モジュール `color`・`id` を非公開にした (中身は根で再公開している)。doc のリンク (`[`color::…`]`) を直した
- [x] ugoe-runtime: モジュール `compile`・`compose`・`dynamic`・`instance`・`parameters` を非公開にし、`compose::REPLACED_PART_PREFIX`・
      `keyform::linear_weights`・`keyform::GridWeights` を `pub(crate)` にした
- [x] ugoe-format: `bundle::{HEADER_LEN, CHUNK_ENTRY_LEN, CHUNK_ALIGN, CHUNK_FLAG_ZSTD}` を `pub(crate)` にした
- [x] ugoe-motion: `Rng`・`easing` を `pub(crate)` にした
- [x] 狭めた後、使われなくなった項目の dead_code の警告が出れば、消すかテストの中へ移した (`#[allow(dead_code)]` で黙らせない)
- [x] ワークスペースの外 (`bindings/godot`・`fuzz/`・`tests/conformance`) もビルドできる (`just fuzz-check` は nightly が要るので、無ければ grep で使う所が無いことを確かめてメモに)
- [x] 各クレートの README と docs の、狭めた項目の道 (`ugoe_runtime::compile::…` など) を直した
- [x] `just wasm-check`・`just nostd-check` が通る
- [x] `just ci` が通る

## 参考

- T0195 のメモ (項目ごとの方針・起票の候補 1〜4)、docs/release.md#公開の範囲
- ADR 0035 の追記 (公開の前の安定の扱い)

## メモ

- T0095 で起票した (元の候補: T0195 (core: モジュール color・id を非公開にする)、T0195 (runtime: モジュール compile・compose・dynamic・instance・
  parameters を非公開にし、REPLACED_PART_PREFIX・linear_weights・GridWeights を pub(crate) に)、T0195 (format: bundle の配置の定数 4 つ
  (HEADER_LEN など) を pub(crate) に)、T0195 (motion: Rng・easing を pub(crate) に))。
- M9 の後続 (T0195 と同じ)。利用者の方針 (2026-10-11) で「内部の整理」として扱うので、互換の別名・移行の説明・semver の注記は要らない。
- 洗い出しのときに外で使う所が無かった項目だけ。着手のときに、T0195 の後に入ったコードが使い始めていないかを grep で確かめる
  (使っていれば、その項目は公開のままにするか、使う側に意味のある口を足すかを決めてメモに書く)。
- 着手のときに grep で確かめた: T0195 の後に入ったコードも、狭めた項目 (モジュールの道 `ugoe_core::{color,id}::…`・
  `ugoe_runtime::{compile,compose,dynamic,instance,parameters}::…`、`REPLACED_PART_PREFIX`・`linear_weights`・`GridWeights`・bundle の配置の定数 4 つ・
  `ugoe_motion::{Rng,easing}`) を外で使っていない (ワークスペースの外の `bindings/godot`・`fuzz/`・`tests/conformance` も)。
  `ugoe_runtime::compose` の道は関数 `compose` (根で再公開) を指すので、文書とコードはそのまま。

### やったこと

- ugoe-core: `mod color`・`mod id` を非公開に。`[`color::…`]` のような doc のリンクは無かった (直す所は無し)。
- ugoe-runtime: `compile`・`compose`・`dynamic`・`instance`・`parameters` を非公開に。`REPLACED_PART_PREFIX`・`linear_weights`・`GridWeights`
  (とその `with_capacity`・`compute`) を `pub(crate)` に。
- ugoe-format: `bundle::{HEADER_LEN, CHUNK_ENTRY_LEN, CHUNK_ALIGN, CHUNK_FLAG_ZSTD}` を `pub(crate)` に。
- ugoe-motion: `Rng` (とそのメソッド)・`easing` を `pub(crate)` に、根の再公開から外した。テストは `crate::player::easing` を使う。
  `EyeBlinkPlayer` の doc の `[`Rng`]` のリンク (非公開の項目へのリンクになる) を文に直した。
- 狭めた後も dead_code の警告は出なかった (どの項目もクレートの中で使う)。rustdoc (`-D warnings`) も警告無し。
- README・docs・公開仕様に、狭めた項目の道を書いた所は無かった (`easing` は仕様の式の名前)。
- 通した検査: `just crate` (4 つ)・`just wasm-check`・`just nostd-check`・`just conformance`・`just godot-check`・`just fuzz-check`・`just ci`。

### 起票の候補

- 無し。
