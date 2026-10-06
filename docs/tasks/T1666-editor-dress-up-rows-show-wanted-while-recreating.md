---
id: T1666
title: "エディタ: プレビューの着せ替えで、GPU を失っている間・作り直しの途中の付け外しを、すぐ一覧のチェックとスロットの選択に出す"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1666
---

# T1666: エディタ: プレビューの着せ替えで、GPU を失っている間・作り直しの途中の付け外しを、すぐ一覧のチェックとスロットの選択に出す

## 目的

T1658・T1659 で、GPU を失っている間と WebPuppet の作り直しの途中の付け外しは、付けたい組 (`wanted`) に残り、作り直した WebPuppet に
付くようになった。ただし一覧の行のチェックとスロットの選択は、実際に付いているか (`attached`) を写すので、作り直すまで付け外しが画面に出ない。
利用者には押しても変わらないように見える。

## 受け入れ条件

- [x] GPU を失っている間と作り直しの途中は、行のチェックとスロットの選択が `wanted` を写す (か、待っていることを行に出す)。どちらにするかと、
      付けられなかった (作り直しの後に付け損ねた) ときの戻し方を決め、メモと docs/editor.md (プレビューの着せ替え) に書いた
- [x] 待っている印を出すなら、文字か読み上げで伝え、色だけに頼らない
- [x] 部品のテスト (DressUpSection・dressUp.test.ts): 失っている間・作り直しの途中にチェックを外す・スロットで替えると、すぐ行とスロットの選択に出て、
      作り直した後も同じ。作り直しが失敗したときの行の状態
- [x] `just editor-browser` のシナリオ (`dress-up-change-while-lost` など) で、作り直す前に行のチェックが変わることを確かめた
- [x] `just ui-audit --only editor` (`-preview-dress-up` を含む組) で増えた指摘が無い。`just e2e --only editor` と `just ci` が通る

## 参考

- T1659 のメモ (起票の候補)、T1658・T1656 のメモ、docs/ui.md#webpuppet-の寿命
- apps/editor/src/lib/preview/dressUp.svelte.ts (`wanted`・`attached`)・DressUpSection.svelte、apps/editor/browser-test/

## メモ

- T0052 で起票した (元の候補: T1659 (失っている間・作り直しの途中の付け外しが一覧に出ない))。
- 決めたこと: 待っている間 (`DressUp.waiting`。付け外し・読み込みを WebPuppet に渡せなかったときに立ち、作り直した WebPuppet で
  `loaded` が揃えると下りる) は、行のチェックとスロットの選択が `wanted` を写す (`DressUp.shown`)。そのうえで、まだ WebPuppet に
  届いていない行 (`pendingChange`) に「Puts this on / Takes this off when the preview is back.」を文字で添え、チェックボックスの
  `aria-describedby` にする (色やチェックの見た目だけに頼らない)。
- 付け損ねたときの戻し方: 作り直した WebPuppet に付けられなかったものは、ほかの付け直しと同じく `#takenOff` で付けたい組から外して
  チェックを戻し、行に「… was taken off: 理由」を出す。作り直しを諦めた間 (ビューポートの「作り直す」待ち) は、付けたい組と添えた文を
  出したまま待つ (付けたい組はもうプロジェクトに残してあるため)。docs/editor.md (プレビューの着せ替え) の「待っている間の行」に書いた。
- ついでに直したこと: GPU を失った WebPuppet で、付いていないモジュールを外す (`detach`) と `#afterChange` が古い WebPuppet の状態で
  付けたい組を上書きしていた (失っている間に外したものが作り直しで付き直る)。失っていると分かっていれば試さないようにした
  (`attach` も同じ)。dressUp.test.ts にテストを足した。
- テスト: 偽の WebPuppet を `src/lib/preview/test/fakeModulePuppet.ts` に移し、DressUpSection.test.ts (新規。失っている間・作り直しの
  途中のチェックとスロットの選択・作り直しを諦めた間・付け損ねた行) と dressUp.test.ts から使う。browser-test の
  `dress-up-change-while-lost`・`dress-up-change-during-recreation` で、作り直す前に行のチェック・スロットの選択・待っている文が
  変わることを確かめる (`just editor-browser` 4/4 ok)。
- `just ui-audit --only editor`: 変更前のソースで撮った `target/ui-audit-before` と比べて `+0 -0 ~0` (`-preview-dress-up` を含む。
  ただし監査の場面は GPU を失わないので、待っている文の出る状態は撮っていない)。`just e2e --only editor`・`just ci` は通った。
- 起票の候補: ui-audit に「着せ替えで作り直しを待っている」場面 (待っている文の出る行) を足す。
