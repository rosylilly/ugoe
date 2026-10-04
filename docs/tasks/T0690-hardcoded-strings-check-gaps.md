---
id: T0690
title: 直書きの文言の検査の見落とし (テンプレート文字列・例外の文・ファイルのフィルタの名前)
status: in-progress
milestone: M9
crates: [scripts, apps/editor, apps/viewer, apps/studio, packages, docs]
depends_on: [T0670]
assignee: claude-worker-T0690
---

# T0690: 直書きの文言の検査の見落とし (テンプレート文字列・例外の文・ファイルのフィルタの名前)

## 目的

T0670 のレビューで、`scripts/check-hardcoded-strings.mjs` (`just i18n`) が次の英語の文言を見つけていなかった
(エディタの分は T0670 で直した)。検査を広げ、ほかのアプリと packages/ に残るものを直す。

- テンプレート文字列の文 (`` `Asking ${name} to revise proposal #${id}…` `` を進み具合の表示に渡していた)
- 画面に出す例外の文 (`throw new Error("the proposal could not be applied")` を `errorText` でそのまま表示していた)。
  いまの検査は `new Error(...)` を対象の外にしている
- OS のファイルダイアログに出るフィルタの名前 (`{ name: "Photoshop document (.psd)", extensions: [...] }`)。
  `name` は表示に使うプロパティ (`DISPLAY_PROPS`) に無い。
  `apps/editor/src/lib/preview/dressUp.ts` の `MODULE_FILTER` はまだ英語で、`lib/filePaths.ts` の `MODULE_FILTER` と重複している

## 受け入れ条件

- [ ] 検査がテンプレート文字列の文を見る (識別子・パス・URL だけのものは除く)
- [ ] 画面に出る例外の文の扱いを決める (検査するか、`t()` を通す規約にするか。docs/ui.md#文言の書き方 に書く)
- [ ] ファイルのフィルタの `name` を検査する
- [ ] 見つかったものを直すか、許す一覧に入れる (`preview/dressUp.ts` の `MODULE_FILTER` は `lib/filePaths.ts` のものに寄せる)
- [ ] 検査のテスト (`scripts/check-hardcoded-strings.test.mjs`) を足す
- [ ] `just ci` が通る

## 参考

- ADR 0029、docs/ui.md#文言の書き方、docs/translation.md#直書きの文言の検査
- T0670 のメモ

## メモ
