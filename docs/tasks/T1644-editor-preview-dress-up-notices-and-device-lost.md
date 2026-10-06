---
id: T1644
title: "エディタ: プレビューの着せ替えで、自動で外れた知らせと行の失敗の重複をまとめ、device_lost を Player と同じ扱いにする"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1644
---

# T1644: エディタ: プレビューの着せ替えで、自動で外れた知らせと行の失敗の重複をまとめ、device_lost を Player と同じ扱いにする

## 目的

エディタのプレビューの着せ替え (`apps/editor/src/lib/preview/dressUp.svelte.ts`・`DressUpSection.svelte`) に 2 つの残りがある。

- 同じスロットに 2 つ付けたいときなど、自動で外れた知らせ (`dressUp.notices`) と行の失敗 (`entry.problem`) が同じ理由を 2 回出す (T0676 の候補)。
- `load_module` などが `device_lost` を投げると、`preview.dressUp.problem.deviceLost` の文で失敗として出す。T0955 で `@ugoe/player` は、
  読み込みの途中の `device_lost` を失敗とせず、作り直しの後に開き直すようになった。プレビューも同じ扱いにそろえるかを確かめる (T0955 の候補)。

## 受け入れ条件

- [x] 同じ理由を知らせと行の失敗の両方に出す場合を洗い、どちらか一方にまとめた (どちらに残したかと理由をメモに書いた)。部品か状態のテストがある
- [x] プレビューの描画の作り直し (エディタのビューポートの喪失の扱い) を調べ、着せ替えの読み込みで `device_lost` を受けたときに作り直しの後に付け直す形にそろえた。そろえないならその理由をメモと docs/editor.md に書いた
- [x] そろえた場合、偽の WebPuppet で「読み込みの途中に喪失 → 作り直して付け直す」の順序のテストがある (修正の前に失敗することを確かめた)
- [x] docs/editor.md (プレビューの着せ替え) を今の形に直した
- [x] `just ui-audit --only editor` の `-preview-dress-up` の指摘が増えない。`just e2e --only editor` と `just ci` が通る

## 参考

- T0676 のメモ (起票の候補)、T0955 のメモ (やったこと・起票の候補)、T0486
- apps/editor/src/lib/preview/dressUp.svelte.ts・DressUpSection.svelte、packages/player/src (読み込みの `device_lost` の扱い)

## メモ

- T0043 で起票した (元の候補: T0676 (自動で外れた知らせと行の失敗の重複)・T0955 (プレビューの着せ替えの `device_lost`))。
  同じファイルを触るので 1 つにまとめ、M10 の候補を含むので milestone は M10 にした。
- **やったこと (T1644):**
  - 重複の洗い出し: 同じ理由を 2 か所に出していたのは `#takenOff` (読み直しで WebPuppet が外したもの (`detached_modules_json`) と、
    `#reconcile` で 1 つずつ試して付けられなかったもの) だけだった。行の `problem` とパネルの頭の `notices` の両方に同じ理由を出していた。
    ほかの失敗 (ファイルを読めない・同じ名前・自分のスロット・一覧の保存) は、行があれば行、無ければパネルの頭のどちらか一方だった。
  - **行に残した** (`DressUp.notices`・`dismissNotice` とパネルの頭の warning の `Notice` を除いた)。理由: 行は直すときに触る場所
    (チェックボックス・読み直し・除く) の隣で、付け直せるまで残る (閉じられる知らせは閉じると、なぜ外れたかが画面から消える)。
    自動で外れたことが分かるよう、行の文は「… was taken off: 理由」(`preview.dressUp.takenOff`。`problem.code`・`detail` は元のまま)。
    行が無いもの (実際には起きない) は `console.warn` だけ。
  - `device_lost`: ビューポート (`ViewportController`) は T0955 で、読み込みの途中の喪失を文にせず作り直した後に読むようになっていた。
    プレビューは毎フレーム描くので次のフレームの `checkLost()` で作り直し、`#setUp` → `PreviewMode.loaded` → `DressUp.loaded` が呼ばれる。
    着せ替えもこれにそろえた: `load_module` が投げて `is_lost()` が真なら、バイト列を `#pending` に取っておき (`add` なら付けるか、
    ファイルから読み直していた行も)、文を出さない。付け外し (`attach`・`detach`・`setSlot`) は付けたい組 (`wanted`) だけを変える
    (付けるときは同じスロットのほかの行を外す)。`DressUp.loaded` は持っているバイト列と `#pending` を読み込んで付ける。
    作り直した WebPuppet がまた失っていたら (`loaded` の途中・`#reconcile` の `set_modules`) 付けたい組を変えずに戻る。`setOwn` の
    読み込みも文を出さない (バイト列は持っているので次の `loaded` で読む)。`preview.dressUp.problem.deviceLost` の文は、`is_lost()` が
    偽の `device_lost` のための残り (`describeProblem`) として残した。
  - テスト (dressUp.test.ts): 偽の WebPuppet に `state.lost` (読み込みと付け外しが `device_lost` を投げる) と `is_lost` を足し、
    「失っている間に読み込む → 作り直して付ける (同じスロットは差し替え・一覧を残す)」「失っている間の付け外し → 作り直した後に付けたい組で付く」
    「ファイルから読み直す途中に失う」「作り直した WebPuppet がまた失っても外さない」の 4 つ。直す前に 4 つとも落ちることを確かめた。
    外れたことの知らせのテストを、行の文 1 つに直した (直す前は落ちる)。PreviewMode のテストの偽物 (poseFollow・tracking) に `is_lost` を足した。
  - 文書: docs/editor.md (プレビューの着せ替えの「読み直しと外れたことの知らせ」・「GPU の喪失」・エラーの知らせの形)、
    docs/ui.md#webpuppet-の寿命 (読み込みの途中の喪失にプレビューの着せ替え)、docs/manual/dress-up.md (外れた枠は行に出る・GPU が止まったとき)。
- **やらなかったこと・確かめていないこと:**
  - 本物の GPU の喪失での確かめ (偽物のテストだけ。e2e・browser-test にシナリオは足していない)。
  - 失っている間に「✕」で除いたものは、WebPuppet からは消せない (`unload_module` が投げる) が、手元のバイト列を捨てるので作り直した
    WebPuppet には読み込まない (前と同じ)。
- **検査:** `just e2e --only editor` は 15 件すべて通った。`just ui-audit --only editor` は 412 場面すべて 0 件 (`-preview-dress-up` の 12 場面も 0。
  直す前の基準は撮っていないが、T0676 の後も範囲の画面は 0 件で、増えていない)。`just ci` は main f34ad4d0 に rebase した後に通った。
- **起票の候補:**
  - 本物の wasm での「読み込みの途中の喪失 → 作り直して付け直す」を、エディタの e2e か browser-test のシナリオにする (今は偽物の単体テストだけ)。
