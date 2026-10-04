---
id: T0689
title: エディタのファイルメニューの非同期の操作の排他と、流れのテスト
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0670]
assignee: claude-worker-T0689
---

# T0689: エディタのファイルメニューの非同期の操作の排他と、流れのテスト

## 目的

T0670 のレビューで見つけた、`apps/editor/src/lib/FileMenu.svelte` (約 840 行) の非同期の操作の重なりを整理し、
流れをテストできる形にする。いまは流れのロジックがすべて部品の中にあり、テストが無い。

- `save`・`overwriteExternal`・`reloadProject` は `busy` を立てないので、「Save」を続けて押すと保存が 2 回走り、
  保存の途中に外での書き換えの確認 (`checkExternal`) や他の操作が始まりうる。
- `checkExternal` はウィンドウのフォーカスと一定の間隔の両方から呼ばれ、重なると同じ変化に 2 回応じうる
  (自動で開き直す設定なら `reloadProject` が 2 回)。
- モジュールの設定の適用 (`applyModuleSettings`) は `ipc.applyCommand` を直接呼び、適用の報告 (`ApplyReport.warnings`) を捨てる。
  ほかの編集と同じく `EditorDocument.applyWith` を通す。
- 書き出しの設定の欄 (`ExportPanel.svelte` の `update`) は、続けて変えると前の変更が終わる前の古い設定を元に送り、前の変更を戻してしまう。
- 開く・本体の付け替え・外での書き換えの確認の分岐 (`run`・`perform`・`reopenModule`・`relocateBase`・`reloadBase`) に
  テストが無い。

## 受け入れ条件

- [x] 流れ (`run`・`perform`・保存と外での書き換え・本体の変化) を `.svelte.ts` のクラスか純粋な関数に移し、部品は表示と呼び出しだけにする
- [x] 保存・上書き・開き直し・外での書き換えの確認が重ならない (実行中の操作があれば、後から来たものは待つか捨てる)
- [x] モジュールの設定の適用を `EditorDocument.applyWith` に通し、書き出しの設定の変更を重ねない
- [x] 上の流れを IPC を偽物にした Vitest で確かめる (保存の連打・フォーカスと間隔の確認の重なり・本体の問題で開けないとき)
- [x] 画面の見た目と配置は変えない (M10 の範囲)
- [x] `just ci` が通る

## 参考

- docs/editor.md#外での書き換え・#本体の変化・#本体の問題の直し方
- T0670 のメモ

## メモ

### やったこと

- 流れを `apps/editor/src/lib/fileMenu.svelte.ts` の `FileMenuFlow` に移した (`run`・`choose`・`perform`・`reopenModule`・`relocateBase`・
  保存と外での書き換え・本体の変化・破棄の確かめ)。`FileMenu.svelte` は表示と呼び出しだけで、Toast・ファイルダイアログ・ドキュメントは
  `FileMenuHost` で渡す (テストでは偽物)。`App.svelte` は `FileMenu` に `doc` を渡すようにした。
- 操作を重ねない: 利用者の操作 (保存・上書き・開き直し・本体の読み直し・ダイアログで選ぶ操作・スロット・衝突体の参照を外す・本体を開く・
  モジュールの設定・新しいモジュール) は `#exclusive` で `busy` を立てて 1 つずつ行い、途中に来たものは捨てる。確かめ (`checkExternal`) は
  操作の途中なら行わず、確かめの途中に来た確かめは同じ Promise を返す (フォーカスと間隔が重なっても 1 回だけ応じる)。確かめの途中に
  来た利用者の操作は、確かめ (自動の開き直しまで) を待ってから行う。破棄の確かめのダイアログを開いている間も確かめない。
- モジュールの設定の適用を `EditorDocument.applyWith` に通した。ダイアログに失敗を返すため `applyWith` に `{ rethrow: true }` を足した
  (`doc.error` に置かずに例外のまま返す)。名前の変更に伴う選択の付け替えは、適用の後・読み直しの前 (applyWith の action の中) で行う
  (TreePanel が読み直しの後に存在しない選択を外すため、順番が要る)。
- 書き出しの設定の欄は `apps/editor/src/lib/exportSettings.svelte.ts` の `ExportSettingsForm` に移し、変更を列にして 1 つずつ、残した設定に
  重ねて送る。変えた値はすぐ欄に出し、断られた変更だけを欄から戻す。
- テスト: `fileMenu.test.ts` (保存の連打・保存の途中の確かめ・上書きの二度押し・フォーカスと間隔の確かめの重なり (開き直し・尋ねる・本体の
  読み直し)・確かめの途中の保存・本体の問題で開けないときの直し方 (選び直し・スロット・付け替え・本体を開く)・モジュールの設定)、
  `exportSettings.test.ts`、`document.test.ts` に `applyWith` の `rethrow`。
- docs/editor.md の「外での書き換え」「モジュールの設定」「書き出し」に追記した。

### 確かめていないこと

- 画面の見た目と配置: テンプレートの構造・クラス・スタイルは変えておらず (束縛先を `flow.*` に替えただけ)、svelte-check は通ったが、
  実際のエディタ (Tauri) やヘッドレスのブラウザで開いて目で確かめてはいない。

### just ci

通った (rebase 後の main に対して)。

### 起票の候補

- なし。

