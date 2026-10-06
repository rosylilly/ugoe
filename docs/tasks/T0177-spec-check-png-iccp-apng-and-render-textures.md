---
id: T0177
title: "spec-check: iCCP と APNG のフレームを持つ PNG の例を足し、描画の例のテクスチャでも PNG の記述を使う"
status: todo
milestone: M9
crates: [spec, tools/spec-check, ugoe-format, ugoe-render, docs]
depends_on: []
assignee:
---

# T0177: spec-check: iCCP と APNG のフレームを持つ PNG の例を足し、描画の例のテクスチャでも PNG の記述を使う

## 目的

T0175 で、ファイルの例 (`spec/examples` の `png-*.ugoe`) に PNG の読み方を確かめる例を足し、目録の `png` の記述から spec-check が PNG を作るようにした。
残ったもの (T0175 の候補):

- `iCCP` と APNG のフレーム (`fcTL`・`fdAT`) は、spec/rendering.md#textures で「読み飛ばす」と決めているが、それを確かめる例が無い。
- 描画の例 (`render.json`) のテクスチャは `png` の記述を使えないので、16 ビットやパレットの画像を乗算済みのアルファに変えるところを描画では確かめていない。

## 受け入れ条件

- [ ] ファイルの例に `iCCP` を持つ PNG と、APNG のフレームを持つ PNG (既定の画像と違う色のフレーム) の例がある。spec-check の `encodePng` が作り、
      読み手が既定の画像 (`IDAT`) だけを色を変えずに読むことを目録の `rgba` で確かめる
- [ ] `iCCP` で色を変換する・APNG のフレームを読む、の誤った読み方を spec-check の読み込みに一時的に入れると落ちることを確かめ、メモに書いた
- [ ] 描画の例のテクスチャに `png` の記述を使え、16 ビットとパレット (`tRNS` 付き) の画像を使う描画の例がある。参照の画像を作り直し、
      `crates/ugoe-render/tests/spec_render.rs` と spec-check の両方で一致する
- [ ] `crates/ugoe-format/tests/spec_bundles.rs`・`spec_examples.rs` が新しい例を読み、参照実装が満たす (`KNOWN_MISMATCHES` を増やさない)
- [ ] spec/examples/README.md・spec/rendering.md#textures (例への参照)・tools/spec-check/README.md を直した
- [ ] `just ci` が通る

## 参考

- T0175 のメモ (やったこと・起票の候補)
- spec/rendering.md#textures、docs/spec/rendering.md#前提、tools/spec-check/src/png.ts・render-examples.ts

## メモ

- T0041 で起票した (元の候補: T0175 (`iCCP` と APNG の例)・T0175 (描画の例の `png` の記述))。ugoe-format と ugoe-render はテストだけを触る。
