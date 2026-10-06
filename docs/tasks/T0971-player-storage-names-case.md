---
id: T0971
title: "@ugoe/player: 覚える値の名前 (dress-up.auto-restore・lighting.models など kebab-case) をエディタのキー (camelCase) にそろえるか決める (判断が要る)"
status: done
milestone: M9
crates: [packages/player, apps/viewer, apps/studio, docs]
depends_on: []
assignee: claude-worker-T0971
---

# T0971: @ugoe/player: 覚える値の名前 (dress-up.auto-restore・lighting.models など kebab-case) をエディタのキー (camelCase) にそろえるか決める (判断が要る)

## 目的

T0957 で localStorage のキーの接頭辞は `ugoe.<アプリ>.` にそろったが、後ろの名前の形がそろっていない。
`@ugoe/player` の `STORAGE_NAMES` (`dress-up.auto-restore`・`lighting.models` など) は kebab-case と `.` の区切り、
エディタ (`ugoe.editor.parameterSectionsClosed` など) とアプリの `storageKeys.ts` は camelCase。

**判断が要る:** 名前の形をどれにそろえるか (利用者が決める)。

- 案 A: すべて camelCase (`ugoe.viewer.dressUp.autoRestore`)。`.` は分野の区切りだけに使う。
- 案 B: すべて kebab-case (`ugoe.editor.parameter-sections-closed`)。
- 案 C: そろえない。決め (Player の名前は kebab-case、アプリは camelCase) を docs/ui.md に書くだけにする。

推奨は案 A (エディタ・アプリの 2 つが camelCase で、変える名前が Player の一覧だけで済む)。リリース前なので古いキーは読まない。

## 受け入れ条件

- [x] 利用者の判断をメモに書いた (この節の案から選ぶ)
- [x] 案 A か B なら: `STORAGE_NAMES` (かアプリのキー) を選んだ形にし、各アプリの `storageKeys.test.ts` に名前の形の検査 (正規表現) を足した。
      古いキーは読まない
- [x] apps/viewer/README.md・docs/studio.md の「覚える値」の表と docs/ui.md に、名前の形の決めを書いた
- [x] `pnpm --filter @ugoe/player test`・`just e2e --only viewer`・`just e2e --only studio` と `just ci` が通る

## 参考

- T0957 のメモ (やらなかったこと・起票の候補 (1))
- packages/player/src/storage.ts、apps/viewer/src/lib/storageKeys.ts、apps/studio/src/lib/storageKeys.ts、apps/editor/src/lib/storageKeys.ts

## メモ

- T0046 で起票した (元の候補: T0957 (Player の覚える値の名前の形))。元のタスクに合わせて milestone は M9。
- 判断 (2026-10-06): 利用者が案 A (すべて camelCase。`.` は分野の区切りだけ) を選んだ。
- やったこと: `STORAGE_NAMES` を camelCase にした (`dress-up` → `dressUp`、`dress-up.auto-restore` → `dressUp.autoRestore`、
  `idle-motion.models` → `idleMotion.models`、`auto-idle` → `autoIdle`、`motion-over-tracking` → `motionOverTracking`、
  `motion-over-tracking.kept` → `motionOverTracking.kept`、`remembered-order` → `rememberedOrder`。`lighting`・`lighting.models`・
  `lighting.markers` はもとからこの形)。古いキーは読まない。ビューア・スタジオの `storageKeys.test.ts` に、Player の名前が
  `^[a-z][A-Za-z0-9]*(\.[a-z][A-Za-z0-9]*)*$` に合う検査を足した (アプリのキーの名前は前から `^[a-z][A-Za-z0-9]*$` で検査済み。
  エディタは Player を使わないので変えていない)。決めは docs/ui.md の「名前と切り替え」に書き、apps/viewer/README.md・docs/studio.md・
  docs/editor.md の「覚える値」と packages/player/README.md からそこへ参照した。player のテストのキーの文字列も直した。
- 確かめたこと: `pnpm --filter @ugoe/player test`・`just e2e --only viewer`・`just e2e --only studio`・`just ci` が通った。
- やらなかったこと: 過去のタスクファイルに残る古いキー名 (`ugoe-viewer.dress-up` など) は経緯なので直していない。
- 起票の候補: なし。
