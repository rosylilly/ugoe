---
id: T0164
title: 公開仕様と spec-check の残り (検証するカーブの範囲・目パチの段の上限・拒まれる例の名前・文字列の enum の形。判断が要る)
status: in-progress
milestone: M9
crates: [spec, tools/spec-check, scripts, ugoe-model, docs]
depends_on: []
assignee: claude-worker-T0164
---

# T0164: 公開仕様と spec-check の残り (検証するカーブの範囲・目パチの段の上限・拒まれる例の名前・文字列の enum の形。判断が要る)

## 目的

T0326 で公開仕様を直した際に、spec-check (仕様だけから書いた別の実装) の作業者が次を指摘した (T0326 の候補)。あわせて T0017 の公開仕様の生成物の揃え方も扱う。

1. 仕様の曖昧さ: 「Motion and expression validation」の「every curve の数値は有限」が、再生で無視するカーブ (未知の対象・未知のセグメントの種類) を
   含むかが書かれていない (未知の `op` の調整は明記されている)。spec-check は字義どおり復号できた数をすべて検証した。
2. spec-check の `EyeBlink.advance` (`tools/spec-check/src/`) に 1 回の更新の段の数の上限が無く、巨大な `dt` で RangeError になる。仕様は上限を許すが値を決めていない。
3. 拒まれる例の JSON を `*.motion.json`・`*.expression.json` の名前にしたいなら、`scripts/check-spec.mjs` が `error: validation` の項目だけが使う文書を
   JSON Schema の検査から外す必要がある (今は `invalid-*.json` の名前で避けている)。
4. 公開仕様の JSON Schema で、文字列の enum の形 (`enum` か `oneOf` の `const`) が doc コメントの有無で変わる (`crates/ugoe-model/src/spec_files.rs` の変換。T0017 の候補)。

**判断が要る:** 1 は参照実装 (`ugoe-format`・`ugoe-model` の検証) の挙動に揃えて仕様に書く (挙動を変えるなら利用者の判断)。2 は仕様で上限を決めるか、spec-check だけで守るか。
4 は公開仕様の生成物が変わるので、どちらの形に揃えるかを決める。

## 受け入れ条件

- [ ] 1: 参照実装が無視するカーブの数値を検証するかを確かめ、`spec/model.md` (と `docs/spec/` の対応する節) に明記した
- [ ] 2: spec-check の目パチが巨大な `dt` で例外を出さない (仕様で上限を決めたなら仕様と参照実装も) テストがある
- [ ] 3: 拒まれる例の文書の名前を決め、`check-spec.mjs` を合わせた (変えないなら理由をメモに)
- [ ] 4: 文字列の enum の形を 1 つに揃えた生成物と、それを確かめるテストがある (判断を待つなら blocked にしてメモに案を書く)
- [ ] `just spec` と `just ci` が通る

## 参考

- T0326・T0017 のメモ (起票の候補)
- docs/spec/public-spec.md、ADR 0021、tools/spec-check/

## メモ

- T0026 で起票した (元の候補: T0326 の 3 つ、T0017 の enum の形)。
