---
id: T1633
title: "エディタ: localStorage のキーの名前を ugoe.editor.* にそろえる"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0675]
assignee: claude-worker-T1633
---

# T1633: エディタ: localStorage のキーの名前を ugoe.editor.* にそろえる

## 目的

エディタが localStorage に覚える値のキーの形がそろっていない (`ugoe.editor.*`・`ugoe.timeline`・`ugoe-editor.locale`・`ugoe-editor.theme`・`ugoe-editor.mcpb` など)。
T0670 は「変えると利用者の覚えた値が消える」として見送ったが、リリース前なので覚えた値の移し替えは要らない (利用者の方針)。T0673 で区切りのキーは `ugoe.editor.*` にそろえた (T0673 の候補)。

## 受け入れ条件

- [x] エディタの localStorage のキーがすべて `ugoe.editor.<名前>` の形で、1 か所 (定数の一覧) から使う。古いキーを読む処理や移し替えは入れない
- [x] キーの一覧とその形が単体テストで確かめられる (同じキーを 2 つの用途で使わない)
- [x] docs/editor.md (覚える値) を直した。ビューア・スタジオのキーの形も調べ、そろっていなければメモに書いた (直すのは範囲の外)
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0673 のメモ (起票の候補)、T0670 のメモ (見送りの表)
- apps/editor/src/lib (`locale.ts`・`layout.ts`・`externalChange.ts`・`panels/timeline/timeline.ts`・`agents/ConnectAgents.svelte`)、apps/editor/src/main.ts

## メモ

- T0041 で起票した (元の候補: T0673 (localStorage のキーの名前))。T0675 (レビュー中) がタイムラインを触るので、その後に着手する。
- T1633 (claude-worker-T1633):
  - やったこと: キーの一覧 `apps/editor/src/lib/storageKeys.ts` (`STORAGE_KEYS`。`ugoe.editor.<名前>`) を作り、言語 (`ugoe-editor.locale` → `ugoe.editor.locale`)・テーマ (`ugoe-editor.theme` → `ugoe.editor.theme`。`index.html` の `<head>` も)・タイムラインの開閉 (`ugoe.timeline` → `ugoe.editor.timelineOpen`)・自動で開き直す・区切りの 4 つをすべてここから使うようにした。`locale.ts` の `LOCALE_KEY` は消した。古いキーの読み込みや移し替えは入れていない。
  - テスト: `storageKeys.test.ts` で形 (`ugoe.editor.` + 一覧の名前)・重なりが無いこと・区切りの大きさのキーが一覧のものであること・`index.html` が読むキーがテーマのキーと一致することを検査する。`ugoe-editor.mcpb` は localStorage のキーではなく、書き出すファイルの既定の名前なので変えていない。
  - docs/editor.md に「覚える値」(キー・値・既定・場所の表) を足し、言語・テーマ・タイムライン・外での書き換え・区切りの記述から参照した。docs/ui.md のテーマと言語のキーも直した。
  - 範囲の外の小さな変更: `tests/e2e/specs/editor.test.mjs` (言語の e2e が読むキー) と `packages/i18n/README.md` (例と説明のキー)。
  - ビューア・スタジオのキーの形: ビューアは `ugoe-viewer.locale`・`ugoe-viewer.theme`・Player の `ugoe-viewer.<名前>`、スタジオは `ugoe-studio.locale`・`ugoe-studio.theme`・Player の `ugoe-studio.player.<名前>` と古い設定の片付け (`ugoe-studio.tick-rate` など) で、どちらも `ugoe-<アプリ>.*` の形。エディタの `ugoe.editor.*` とはそろっていない (直していない)。
  - `just e2e --only editor` は 15 件すべて通った。`just ci` は通った。ただし e2e の後に `apps/editor/src/lib/viewport/generated/` (e2e が作る wasm。git の対象外) が残っていると、`just ci` の vitest で `src/lib/panels/unreadableModel.test.ts` が wasm の fetch の中断 (`NetworkError ... ugoe_wasm_bg.wasm: The operation was aborted`) を unhandled rejection として 2 件出して失敗した (テストは 677 件すべて通る。この変更とは無関係で、単独で流すと通る)。generated を退けてから流すと通った。
  - 起票の候補: (1) ビューア・スタジオのキーを `ugoe.viewer.*`・`ugoe.studio.*` にそろえる (Player の接頭辞・index.html のテーマのキーも)。(2) generated の wasm があるときに、エディタの部品のテスト (unreadableModel.test.ts など) が wasm の fetch を終わらせずに抜けて unhandled rejection になるのを直す。
