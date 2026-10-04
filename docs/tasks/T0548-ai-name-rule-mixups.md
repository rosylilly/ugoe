---
id: T0548
title: AI: 名前の規則の取り違えを減らす (部位の語のないハイライト・リボン・top)
status: done
milestone: M9
crates: [ugoe-ai, docs]
depends_on: [T0528]
assignee: claude-worker-T0548
---

# T0548: AI: 名前の規則の取り違えを減らす

## 目的

T0528 のレビューで見つけた、名前の規則 (`classify/rules.rs`) の取り違え。T0528 では髪のハイライト (`前髪ハイライト`・`back hair hl`) が
目のハイライトになるのを、髪の規則を目の規則より先に見ることで直した。残りは規則の並びだけでは直らないので、まとめて扱う。

- `highlight`・`hl`・「ハイライト」は部位の語がなくても目のハイライトになる。顔・頬・服のハイライト (`顔ハイライト`・`cheek hl`) も
  `eye_highlight` になり、目の開閉でつぶれる
- 「リボン」・`ribbon` はいつも `headwear`。首のリボン (`neck ribbon`・`胸リボン`) は `neckwear` にしたい (`docs` の語彙の説明では
  `neckwear` にリボンを含む)
- `top` は `topwear` の語なので、`hair top`・`top hair` が服になる

## 受け入れ条件

- [x] 上の名前が期待するラベルになる (部位の語との組み合わせで決める規則など)。See-Through の PSD の正解 (`tests/classify.rs`) は変わらない
- [x] 規則のテスト (`classify/rules.rs` の単体テスト)
- [x] 規則の説明 (`docs/ai-integration.md#レイヤーの意味分類`) を直す (必要なら)
- [x] `just ci` が通る

## 参考

- `crates/ugoe-ai/src/classify/rules.rs`
- `docs/tasks/T0528-review-ai-autorig.md` のメモ

## メモ

- 2026-10-04 claude-worker-T0548: 実装した。
  - `Rule` に「一緒に書かれた部位の語」(`near_words`・`near_kanji`) を足し、`with_part` で作る。首・胸・襟の語 (`neck` `chest` `collar` `breast` `bust`・首・胸・襟) と一緒の「リボン」・`ribbon`・`bow` を `neckwear` にする規則を、頭の飾りより先に置いた。`neckribbon`・`chestribbon` (1 語) も `neckwear`、`hairtie` を `headwear` に足した (`hair tie` が `tie` で `neckwear` になっていた)。
  - ハイライトの規則を 2 つに分けた。目の語を含むもの (`eyehighlight` `eyehl` `eyelight` `catchlight`・目ハイライト・瞳ハイライト) と、部位の語のないもの (`highlight` `hl`・ハイライト)。後者が当たったら後ろの規則で部位を探し、目・瞳 (`eye`・`iris`) 以外の部位があればその部位を採る (`顔ハイライト` → `face`、`cheek hl` → `cheek`、`lip hl` → `mouth`)。後者の位置は定数 `HIGHLIGHT` で持ち、位置がずれたら単体テストが落ちる。
  - `top`・`tops` を服の規則から外し、規則の表の最後 (その他の前) に置いた。`hair top`・`top hair` は髪 (`hair` だけなので `hair_front` の vague)、`top lip` は口、`top` だけなら `topwear`。`tanktop`・`croptop` は服の規則に残す。
  - 単体テスト `highlights_take_the_part_they_are_on`・`ribbons_on_the_neck_are_neckwear`・`top_is_clothing_only_without_a_part`・`highlight_index_points_at_the_bare_highlight_rule`。See-Through の正解 (`tests/classify.rs`) は変えずに通る。
  - `docs/ai-integration.md#レイヤーの意味分類` の名前の規則に、組み合わせで決める語を書いた。英語の公開仕様 (`spec/`) には名前の規則の節がないので直していない。
  - `just ci` は通った。
- やらなかったこと: `白目ハイライト` は白目の規則が先に当たり `eye_white` のまま (目の開閉でつぶれる点は同じなので害は小さい)。`hair top` は `hair_front` の vague (確からしさ 0.5) で、前髪か後ろ髪かは決めない。
- 起票の候補: なし。
