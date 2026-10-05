---
id: T0164
title: 公開仕様と spec-check の残り (検証するカーブの範囲・目パチの段の上限・拒まれる例の名前・文字列の enum の形。判断が要る)
status: done
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

- [x] 1: 参照実装が無視するカーブの数値を検証するかを確かめ、`spec/model.md` (と `docs/spec/` の対応する節) に明記した
- [x] 2: spec-check の目パチが巨大な `dt` で例外を出さない (仕様で上限を決めたなら仕様と参照実装も) テストがある
- [x] 3: 拒まれる例の文書の名前を決め、`check-spec.mjs` を合わせた (変えないなら理由をメモに)
- [x] 4: 文字列の enum の形を 1 つに揃えた生成物と、それを確かめるテストがある (判断を待つなら blocked にしてメモに案を書く)
- [x] `just spec` と `just ci` が通る

## 参考

- T0326・T0017 のメモ (起票の候補)
- docs/spec/public-spec.md、ADR 0021、tools/spec-check/

## メモ

- T0026 で起票した (元の候補: T0326 の 3 つ、T0017 の enum の形)。
- 1 (検証するカーブの範囲): 参照実装 (`ugoe_model::validate::validate_motion`) は対象を見ずにすべてのカーブを検査する。
  未知の対象の種類・未知のエフェクトグループ名・未知のセグメントを含むカーブも、フェードと既知のセグメントの点を検査し、
  未知のセグメントの中身と未知の種類の対象の `id` は読み捨てるので検査しない。挙動は変えず、`spec/model.md#motion-and-expression-validation`
  と `docs/spec/model.md#モーションと表情の検証` に明記し、それを確かめるテスト (`curves_ignored_by_playback_are_validated`) を足した。
  spec-check (`validateMotion`) は元からこのとおり (除いた曲線も検査し、未知のセグメントは飛ばす)。
- 2 (目パチの段の上限): 仕様は上限を許すだけ (MAY) のまま、値は決めない。参照実装の値 (段 4000・超えた分の時間を捨てる) を
  `spec/motion.md#eye-blink` に参考 (Informative) として書き、spec-check はそれに従った (spec-check の独立性の規則 4 のため、先に仕様に書いた)。
  同じ種類の問題がモーションのループにもあった (巨大な `dt` や極端に短い `duration` で止まらない) ので、参照実装の周回の上限 (1000・超えた分は発火せず剰余)
  も `spec/motion.md#motion-playback` に参考として書き、spec-check を合わせた。あわせて spec-check のモーション・表情・目パチ・呼吸が
  無限大の `dt` で進んでいた (仕様は有限の `dt > 0` だけが進める) のを直した。テストは `tools/spec-check/test/motion.test.ts`
  (直す前のコードでは目パチの試験が止まらないことを確かめた)。
- 3 (拒まれる例の名前): 変えない。`invalid-*.json` のままにする。理由: `*.motion.json`・`*.expression.json` の名前は「その種類の正しい文書で、
  JSON Schema を満たす」ことを表す約束として `check-spec.mjs` と `spec/examples/README.md` が使っている。拒まれる例は検証の規則を
  わざと破り、`1e39` は JSON Schema の `f32` の範囲も外れるので、名前で外すほうが、`bundles.json` の `error: validation` を見て
  検査から外す例外を足すより単純で、ファイル名だけで拒まれる例と分かる。名前を変えると公開仕様の例のファイル名 (`bundles.json`・README) も変わる。
- 4 (文字列の enum の形): `oneOf` の `const` の並びに揃えた。`spec_files.rs` の変換で、schemars が doc コメントの無い enum に出す
  `{"type": "string", "enum": [...]}` を `oneOf` の `const` に書き換える。今の生成物はすべて元からこの形 (T0017 で doc コメントを
  付けたため) なので、生成物は変わらない。テストは `spec_files::tests` (doc コメントの有無で同じ形) と `tests/spec_files.rs`
  (生成物に `enum` が無い)。`spec/schema/README.md` に形を書いた。
  - 別の案 (採らなかった): `enum` の形にまとめる。短く読みやすいが、今の 5 つのスキーマの文字列の enum がすべて変わる
    (意味は同じ)。生成物を変えない今の形を選んだ。`enum` の形がよければ、`string_enum_to_one_of` の向きを逆にして `just gen` するだけで変えられる。
- `just spec`・`just ci`・`just wasm-check` が通った。

