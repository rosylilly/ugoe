---
id: T1625
title: "エディタ: 保存のダイアログの後の withExtension を、ブラウザ用の差し替えの側に移す"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T1620, T0940]
assignee: claude-worker-T1625
---

# T1625: エディタ: 保存のダイアログの後の withExtension を、ブラウザ用の差し替えの側に移す

## 目的

T1616 で、エディタの保存のダイアログはバックエンド (`save_file_dialog`) が開き、拡張子もバックエンドが付けるようになった。
画面は保存のダイアログの後にも `withExtension` を呼ぶ (`fileMenu.svelte.ts`・`viewport/saveScreenshot.ts`・`preview/trackingHost.ts`・
`agents/ConnectAgents.svelte`) が、本物のアプリでは何もしない (ブラウザ用の差し替え `window.__UGOE_FILE_DIALOGS__` のために残した。T1616 の候補)。
差し替えの側で拡張子を付けるようにして、画面の呼び出しを外す。

`fileMenu` の保存を触る T1620 と、`saveScreenshot.ts` を触る T0940 の後に着手する。

## 受け入れ条件

- [x] 保存のダイアログの答えに拡張子を付けるのは、バックエンドとブラウザ用の差し替え (`fileDialogs.ts` の差し替えの経路) だけになり、画面の 4 か所から `withExtension` の呼び出しが無い
- [x] ブラウザの差し替えで拡張子の無い名前を答えたときに拡張子が付くテストがある。ヘッドレスの Chrome の e2e と `just ui-audit` の差し替えが今までどおり動く
- [x] docs/editor.md (読み書きできるパス・ダイアログの節) を直した
- [x] `just ci` が通る

## 参考

- T1616 のメモ (起票の候補)
- apps/editor/src/lib/fileDialogs.ts、apps/editor/src/lib/filePaths.ts

## メモ

- T0036 で起票した (元の候補: T1616 (`withExtension`))。
- やったこと:
  - `fileDialogs()` は差し替え (`window.__UGOE_FILE_DIALOGS__`) を包んで返し、`saveFile` の答えに最初のフィルタの最初の拡張子が
    無ければ付ける (`withExtension`。バックエンドの `save_file_dialog` と同じ規則)。開く・既定の場所はそのまま渡す。
  - 画面の 4 か所 (`fileMenu.svelte.ts` の Save As・Export・Export Module、`viewport/saveScreenshot.ts`、`preview/trackingHost.ts`、
    `agents/ConnectAgents.svelte`) から `withExtension` を外し、ダイアログの答えをそのまま使う。`withExtension` は `filePaths.ts` に残る
    (使うのは `fileDialogs.ts` だけ)。
  - テスト: `fileDialogs.test.ts` (拡張子の無い名前に付く・大文字の拡張子はそのまま・取り消しとフィルタ無しはそのまま・開くのは触らない)。
  - docs/editor.md のファイルダイアログ (拡張子を付けるのはバックエンドと差し替えの経路だけ・差し替えの説明)、読み書きできるパス、
    スクリーンショットの節を直した。
- 確かめたこと: e2e (tests/e2e/specs/editor.test.mjs) の `save-file` の答えはどれも拡張子付き (`dress-up-with-psd.ugoe`・`cat-ears.ugoemod`)。
  `just e2e --only editor` は 13 件とも通った。`just ui-audit --only editor --quick` は場面の失敗・エラー 0 (差し替えは開くダイアログだけを使う)。
  `just ui-audit-check` も通った。
