---
id: T1644
title: "エディタ: プレビューの着せ替えで、自動で外れた知らせと行の失敗の重複をまとめ、device_lost を Player と同じ扱いにする"
status: todo
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee:
---

# T1644: エディタ: プレビューの着せ替えで、自動で外れた知らせと行の失敗の重複をまとめ、device_lost を Player と同じ扱いにする

## 目的

エディタのプレビューの着せ替え (`apps/editor/src/lib/preview/dressUp.svelte.ts`・`DressUpSection.svelte`) に 2 つの残りがある。

- 同じスロットに 2 つ付けたいときなど、自動で外れた知らせ (`dressUp.notices`) と行の失敗 (`entry.problem`) が同じ理由を 2 回出す (T0676 の候補)。
- `load_module` などが `device_lost` を投げると、`preview.dressUp.problem.deviceLost` の文で失敗として出す。T0955 で `@ugoe/player` は、
  読み込みの途中の `device_lost` を失敗とせず、作り直しの後に開き直すようになった。プレビューも同じ扱いにそろえるかを確かめる (T0955 の候補)。

## 受け入れ条件

- [ ] 同じ理由を知らせと行の失敗の両方に出す場合を洗い、どちらか一方にまとめた (どちらに残したかと理由をメモに書いた)。部品か状態のテストがある
- [ ] プレビューの描画の作り直し (エディタのビューポートの喪失の扱い) を調べ、着せ替えの読み込みで `device_lost` を受けたときに作り直しの後に付け直す形にそろえた。そろえないならその理由をメモと docs/editor.md に書いた
- [ ] そろえた場合、偽の WebPuppet で「読み込みの途中に喪失 → 作り直して付け直す」の順序のテストがある (修正の前に失敗することを確かめた)
- [ ] docs/editor.md (プレビューの着せ替え) を今の形に直した
- [ ] `just ui-audit --only editor` の `-preview-dress-up` の指摘が増えない。`just e2e --only editor` と `just ci` が通る

## 参考

- T0676 のメモ (起票の候補)、T0955 のメモ (やったこと・起票の候補)、T0486
- apps/editor/src/lib/preview/dressUp.svelte.ts・DressUpSection.svelte、packages/player/src (読み込みの `device_lost` の扱い)

## メモ

- T0043 で起票した (元の候補: T0676 (自動で外れた知らせと行の失敗の重複)・T0955 (プレビューの着せ替えの `device_lost`))。
  同じファイルを触るので 1 つにまとめ、M10 の候補を含むので milestone は M10 にした。
