---
id: T0326
title: "公開仕様の小さな直し: 物理の時間を進める手順の剰余と、検証で拒まれるモーションの例"
status: done
milestone: M9
crates: [spec, docs]
depends_on: []
assignee: claude-worker-T0326
---

# T0326: 公開仕様の小さな直し: 物理の時間を進める手順の剰余と、検証で拒まれるモーションの例

## 目的

- `spec/physics.md` の「Advancing time」の手順 3 が、1 フレームの刻みの上限を超えたときの剰余の扱い (T0316 で直した) を反映していない (T0320 の候補)。
  日本語の `docs/spec/physics.md` と食い違っていないかも確かめる (食い違えば英語版が正。ADR 0021)。
- T0323 でモーションと表情の数値 (NaN・無限大) を検証で拒むようにしたが、`spec/examples` に拒まれる例 (`error: validation`) が無い。

## 受け入れ条件

- [x] `spec/physics.md` の手順 3 を実装 (`ugoe-physics`) と合わせ、`docs/spec/physics.md` との食い違いが無い
- [x] `spec/examples` に、検証で拒まれるモーション (と表情) の例を足し、独立の読み込み器の検証 (T0120・T0126) が拒むことを確かめる
- [x] 公開仕様の同期の検査 (`docs/spec/public-spec.md`) が通る
- [x] `just ci` が通る

## 参考

- T0320・T0323 のメモ (起票の候補)
- spec/physics.md・spec/examples/、docs/spec/public-spec.md

## メモ

- T0019 で起票した (元の候補: T0320 の剰余、T0323 の拒まれる例)。

### やったこと

- `spec/physics.md` の Advancing time の手順 3 を実装 (`ugoe-physics` の `advance`) に合わせた: `whole ≤ 8` なら `accumulator −= whole × step`、
  超えたら `accumulator mod step` (正確な剰余、`[0, step)`) で `n = 8`。理由 (巨大な `dt` での桁落ち) を箇条に書いた。
  日本語の `docs/spec/physics.md#時間の進め方` は「上限を超えた分は捨てる」としか書いていなかったので、同じ剰余の扱いを足した (食い違いはこれ以外に無い)。
- `spec/examples/bundles/` に検証で拒まれるファイルの例を 3 つ足した (`error: validation`。どれも single-quad のモデル + 1 つの規則だけを破る):
  `invalid-motion.ugoe` (点の値が JSON で `1e39` → 無限大。MOTN には float 32 の無限大)・`invalid-motion-duration.ugoe` (負の `duration`)・
  `invalid-expression.ugoe` (調整の `value` が `1e39`)。JSON の文書は `invalid-*.json` (`*.motion.json` にしないのは JSON Schema の `f32` の範囲を満たさないため。README に書いた)。
  README の表と「この版の例は ... だけを使う」の文に `validation` を足した。
- `tools/spec-check/` (`crates:` の外、小さなもの): 独立性の規則のため、`crates/`・`docs/spec/` を読まないサブエージェントが `spec/` だけから行った。
  物理の手順 3 の剰余 (`test/physics.test.ts`)、モーションと表情の検証 (`validate.ts`、無視される曲線・未知の `op` の数も検証のために残す)、
  バンドルの読み込みの手順 7 と時間の例の読み込みでの検証 (`reader.test.ts`)、例のファイルの作成 (`make-bundles.ts`。既存の例は変わらなかった)。
- 参照実装も 3 つを `validation` の段で拒むことを `ugoe-format` の `tests/spec_bundles.rs` (目録の全ファイルを照合) と `ugoe validate` で確かめた。
- `just spec` と `just ci` は通った (rebase 後)。ランタイム層のコードは触っていないので `just wasm-check` は流していない。

### 起票の候補

- なし。
