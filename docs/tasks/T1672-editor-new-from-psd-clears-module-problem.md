---
id: T1672
title: "エディタ: 「New from PSD…」で別のプロジェクトを作った後も、モジュールを開けなかった知らせ (ModuleProblemBar) が残るのを直す"
status: in-progress
milestone: M10
crates: [apps/editor]
depends_on: []
assignee: claude-worker-T1672
---

# T1672: エディタ: 「New from PSD…」で別のプロジェクトを作った後も、モジュールを開けなかった知らせ (ModuleProblemBar) が残るのを直す

## 目的

モジュールのプロジェクトを本体の問題で開けなかったときの知らせ (`ModuleProblemBar`) は、`fileMenu.svelte.ts` の `problem` で出す。
「Open」(`open`)・見本を開く (`openSample`)・新しいモジュール (`createModule`) は `this.problem` を消すが、「New from PSD…」(`newFromPsd`) は
消さないので、PSD から別のプロジェクトを作った後も前の知らせが残る。

## 受け入れ条件

- [ ] 先に失敗するテスト (fileMenu のテスト): 開けなかったモジュールの知らせが出ている状態で `newFromPsd` に成功すると、知らせが消える
- [ ] `newFromPsd` (と、ほかにプロジェクトを置き換える操作で消していないものがあればそれも) が `problem` を消す。
      失敗したとき (PSD を読めなかった) に知らせを残すか消すかを決め、テストとコメントに書いた
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0969 のメモ (起票の候補。e2e のマニュアルの筋書きは知らせを閉じてから進める)
- apps/editor/src/lib/fileMenu.svelte.ts (`#perform` の `newFromPsd`・`open`・`openSample`)、ModuleProblemBar

## メモ

- T0058 で起票した (元の候補: T0969 (「New from PSD…」で知らせが消えない))。
- e2e のマニュアルの筋書きにある、知らせを閉じてから進める手順はそのままでよい (外すなら tests/e2e も範囲に入るので、メモに書くだけにする)。
