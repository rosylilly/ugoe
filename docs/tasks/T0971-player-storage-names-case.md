---
id: T0971
title: "@ugoe/player: 覚える値の名前 (dress-up.auto-restore・lighting.models など kebab-case) をエディタのキー (camelCase) にそろえるか決める (判断が要る)"
status: in-progress
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

- [ ] 利用者の判断をメモに書いた (この節の案から選ぶ)
- [ ] 案 A か B なら: `STORAGE_NAMES` (かアプリのキー) を選んだ形にし、各アプリの `storageKeys.test.ts` に名前の形の検査 (正規表現) を足した。
      古いキーは読まない
- [ ] apps/viewer/README.md・docs/studio.md の「覚える値」の表と docs/ui.md に、名前の形の決めを書いた
- [ ] `pnpm --filter @ugoe/player test`・`just e2e --only viewer`・`just e2e --only studio` と `just ci` が通る

## 参考

- T0957 のメモ (やらなかったこと・起票の候補 (1))
- packages/player/src/storage.ts、apps/viewer/src/lib/storageKeys.ts、apps/studio/src/lib/storageKeys.ts、apps/editor/src/lib/storageKeys.ts

## メモ

- T0046 で起票した (元の候補: T0957 (Player の覚える値の名前の形))。元のタスクに合わせて milestone は M9。
- 判断 (2026-10-06): 利用者が案 A (すべて camelCase。`.` は分野の区切りだけ) を選んだ。
