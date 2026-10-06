---
id: T0054
title: "ui-audit: Tab のたどりを速くし (PNG を Node の側で読む)、エディタの 13 組を前面の 600 秒に収める"
status: in-progress
milestone: M10
crates: [tests, docs]
depends_on: [T0053]
assignee: claude-worker-T0054
---

# T0054: ui-audit: Tab のたどりを速くし (PNG を Node の側で読む)、エディタの 13 組を前面の 600 秒に収める

## 目的

`just ui-audit --only editor` は 13 の組で 10 分ほどかかり、エージェントが前面で流す Bash の上限 (600 秒) に収まらない。
T0048・T0049・T1651・T1657 などのワーカーは毎回 `--variants` で分けて流している。T0045 で足した画像によるフォーカスの印の判定は、
止まりごとに 2 回撮り、ページの中で PNG を読むので 1 回のたどりを 4〜6 秒から 18〜42 秒に延ばした。

## 受け入れ条件

- [ ] Tab のたどりで撮った PNG を Node の側で読む (zlib で展開する小さな読み手か、既にある依存。新しい依存を入れるなら理由をメモに書く)。
      判定 (`focusChange`・`imageRing`) の結果が前と同じことを、単体テストと `--compare` で確かめた
- [ ] エディタの全部の組を 1 回の `just ui-audit --only editor` で、ほかの作業の無い機械で 600 秒より十分短く (目安 8 分以内) 流せる。
      組を並行に流す (ブラウザのページを組ごとに分ける) か、組ごとの場面を減らすかを決め、理由をメモに書いた
      (場面を減らすなら、どの組でどの場面を見なくなるかを README に書く)
- [ ] 前と後の所要時間 (たどり 1 回と、エディタの全体) をメモに書いた
- [ ] `--compare` で前と後の指摘が同じ (増減があれば理由を書く)。`just ui-audit-check` が通る
- [ ] tests/ui-audit/README.md (実行の時間・並行の数の指定があればその口) と docs/testing.md を直した
- [ ] `just ci` が通る

## 参考

- T0045 のメモ (`focus` の数と実行の時間・起票の候補)、T0048 のメモ (起票の候補)
- tests/ui-audit/run.mjs、tests/ui-audit/lib/walk.mjs・page-checks.js、tests/ui-audit/README.md

## メモ

- T0052 で起票した (元の候補: T0045 (たどりの速さ。PNG を Node の側で読む)・T0048 (エディタの 13 組が 600 秒に収まらない))。
- 同じ tests/ui-audit を触るので T0053 の後にした。
