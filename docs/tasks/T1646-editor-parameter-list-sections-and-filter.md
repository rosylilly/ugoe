---
id: T1646
title: "エディタ: パラメータの一覧の区切りの開閉を覚え、絞り込みの欄を足す"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1646
---

# T1646: エディタ: パラメータの一覧の区切りの開閉を覚え、絞り込みの欄を足す

## 目的

T1630 でエディタのパラメータの一覧をグループで区切ったが、区切りの開閉は開き直すと戻り、100 個のパラメータから探す手段も無い
(T1630 の候補)。よく使うグループだけを開いておけるようにし、ビューア・プレビューのような絞り込みの欄を足す。

## 受け入れ条件

- [x] 区切りの開閉を覚える。覚える場所 (プロジェクトのエディタの設定か localStorage の `ugoe.editor.*`) を決めてメモに書いた。プロジェクトを開き直しても保たれる
- [x] パラメータの一覧に絞り込みの欄がある (名前と ID で絞る。絞り込み中は区切りの開閉によらず一致した行を出す。一致が無いときの文)。キーボードで使え、読み上げの名前がある
- [x] 部品のテスト (開閉を覚える・絞り込み・一致なし)
- [x] `just ui-audit --only editor` の `-many-parameters` の場面の指摘が増えない。docs/editor.md を直し、マニュアルの更新の候補をメモに書いた
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T1630 のメモ (やったこと・起票の候補)
- apps/editor/src (パラメータのパネル)、ビューアの絞り込みの欄 (`@ugoe/player`)、docs/editor.md

## メモ

- T0043 で起票した (元の候補: T1630 (区切りの開閉を覚える)・T1630 (絞り込みの欄))。
- やったこと (claude-worker-T1646):
  - 開閉を覚える場所は localStorage の `ugoe.editor.parameterSectionsClosed` と決めた (`storageKeys.ts` の一覧に足した)。値は閉じた区切りの
    グループの ID の JSON の配列 (「Other」は空の文字列)。既定は開く。プロジェクトごとには分けない (同じ ID のグループは
    どのプロジェクトでも同じ開閉)。理由: 開閉は表示の状態で、プロジェクトのファイル (共有・差分に出る) に入れたくない。よく使う
    グループの ID (Face など) はモデルをまたいでそろうことが多い。数は新しく閉じたものから 200 個まで。
    読み書きは `panels/parameters/sections.ts` (読めない値・使えない localStorage ではすべて開く)。
  - 絞り込みの欄: パラメータがあると一覧の上に検索の欄 (`TextField type="search"`、aria-label「Filter the parameters」/
    「パラメータを絞り込む」) を出し、プレビューと同じ `preview.ts` の `filterSections` で名前と ID から絞る (大文字と小文字を区別しない)。
    絞り込み中は覚えた開閉によらず一致した区切りを開いて出す。絞り込み中に閉じた区切りは覚えず、語を変えると開き直す。語を消すと
    覚えた開閉に戻る。一致が無いときは `parameters.filter.none` を `role="status"` で出す。グループの無いモデルでも絞る。文言は英語と日本語。
  - テスト: `sections.test.ts` (保存と読み込み・壊れた値・使えない localStorage・上限)、`ParametersPanel.test.ts` (開き直しても閉じた
    区切りが保たれる・名前と ID で絞る・閉じた区切りの一致も出す・絞り込み中の開閉は覚えない・一致なしの文・見出しの無い一覧の絞り込み)。
  - 文書: docs/editor.md (パラメータのグループの「一覧の区切り」に開閉を覚えること、「絞り込み」の項、「覚える値」の表)。
  - `just e2e --only editor` は 15 件すべて通った。`just ui-audit --only editor` は 412 場面すべて 0 件 (`-many-parameters`・
    `-many-parameters-groups` も 0 件。T1630 の時点もすべて 0 件なので増えていない)。
- マニュアルの更新の候補: docs/manual/screen.md の「パラメータ」の表に、一覧の上の絞り込みの欄 (名前か ID で絞る。絞り込み中は閉じた
  グループも開いて出る) と、グループの見出しの開閉が次に開いたときも保たれることを足す。T0919 の撮り直す画像にパラメータのパネル (欄が増えた)。
- やらなかったこと: 区切りの開閉をプロジェクトごとに覚えること。絞り込みの語は覚えない (開き直すと空)。
- `just ci` は通った (main に rebase した後。e2e が作る `apps/editor/src/lib/viewport/generated/` は T1633 のメモの flaky を避けるため退けてから流した)。
  最初の `just ci` は `just refs` で自分のコメントの参照の書き方 (2 つの見出しを「・」でつないだ) が誤りと出たので直した。
- 起票の候補: (1) docs/manual/screen.md の更新 (上の「マニュアルの更新の候補」)。(2) ui-audit に絞り込みの場面 (一致なしの文・
  絞り込み中の一覧) を足す (今は欄が出ている `-many-parameters` だけで見ている)。
