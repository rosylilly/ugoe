---
id: T1672
title: "エディタ: 「New from PSD…」で別のプロジェクトを作った後も、モジュールを開けなかった知らせ (ModuleProblemBar) が残るのを直す"
status: done
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

- [x] 先に失敗するテスト (fileMenu のテスト): 開けなかったモジュールの知らせが出ている状態で `newFromPsd` に成功すると、知らせが消える
- [x] `newFromPsd` (と、ほかにプロジェクトを置き換える操作で消していないものがあればそれも) が `problem` を消す。
      失敗したとき (PSD を読めなかった) に知らせを残すか消すかを決め、テストとコメントに書いた
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0969 のメモ (起票の候補。e2e のマニュアルの筋書きは知らせを閉じてから進める)
- apps/editor/src/lib/fileMenu.svelte.ts (`#perform` の `newFromPsd`・`open`・`openSample`)、ModuleProblemBar

## メモ

- T0058 で起票した (元の候補: T0969 (「New from PSD…」で知らせが消えない))。
- e2e のマニュアルの筋書きにある、知らせを閉じてから進める手順はそのままでよい (外すなら tests/e2e も範囲に入るので、メモに書くだけにする)。
- やったこと (T1672):
  - 原因: 知らせ (`problem`) を消すのが、プロジェクトを置き換える操作ごとにばらばらだった。`newFromPsd` に加えて
    「New」(`newProject`) も消していなかった。
  - `#run` が `"replaced"` で成功したとき (別のプロジェクトに替わったとき) に `problem` を消すようにまとめ、
    `createModule`・`openSample`・`#reopenModule`・`#openBaseProject` の個別の消し方を除いた。「Open…」は従来どおり、
    失敗しても前の知らせを消し、開こうとしたプロジェクトの問題があればそれを出す。
  - 失敗したとき (PSD を読めなかった) は知らせを残すと決めた: 開いているプロジェクトは替わらず、直し方もそのまま使えるため。
    `#run` のコメント・docs/editor.md#本体の問題の直し方 に書いた。
  - テスト (fileMenu.test.ts): PSD から作ると消える・PSD を読めなければ残る・New で消える。先に前の 2 つと New が失敗するのを確かめた
    (失敗時に残るテストは修正前も通る)。
  - e2e のマニュアルの筋書き (tests/e2e/specs/editor.manual.mjs) の、知らせを閉じてから進める回避を外した
    (直後の「知らせが消えるのを待つ」がこの修正を確かめる)。範囲外 (tests/e2e) の 2 行の小さな変更。
    `just e2e --manual --only editor` が通るのを確かめた (画像は更新していない)。
  - `just e2e --only editor`・`just ci` が通る。
- 起票の候補: なし。

