---
id: T0690
title: 直書きの文言の検査の見落とし (テンプレート文字列・例外の文・ファイルのフィルタの名前)
status: done
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

- [x] 検査がテンプレート文字列の文を見る (識別子・パス・URL だけのものは除く)
- [x] 画面に出る例外の文の扱いを決める (検査するか、`t()` を通す規約にするか。docs/ui.md#文言の書き方 に書く)
- [x] ファイルのフィルタの `name` を検査する
- [x] 見つかったものを直すか、許す一覧に入れる (`preview/dressUp.ts` の `MODULE_FILTER` は `lib/filePaths.ts` のものに寄せる)
- [x] 検査のテスト (`scripts/check-hardcoded-strings.test.mjs`) を足す
- [x] `just ci` が通る

## 参考

- ADR 0029、docs/ui.md#文言の書き方、docs/translation.md#直書きの文言の検査
- T0670 のメモ

## メモ

### やったこと

- 検査 (`scripts/check-hardcoded-strings.mjs`) を広げた。
  - テンプレート文字列: 差し込みや記号で文が切れる (`#${id}…`) ので、全体の形 (`SENTENCE`) に加えて語を数える
    (`isTemplateSentence`。前後の記号を除いて文字だけになる語が 2 つ以上で、1 つが大文字で始まる)。識別子・パス・URL・CSS は
    語が文字だけにならないので見ない。
  - 例外の文: **検査する** 方にした (`isErrorText`。`new XxxError(...)` の最初の引数と `throw` の値。小文字で始まる文も、
    文字だけの語が 2 つ以上なら見つける)。画面に出る例外は `t()` で作り、画面に出ない開発者向けのものは `i18n-ignore` と理由を
    書く規約を docs/ui.md#文言の書き方 と docs/translation.md#直書きの文言の検査 に書いた。
  - ファイルのダイアログのフィルタ: `extensions` を持つオブジェクトの `name` を表示の場所として見る。
  - テストの部品のディレクトリ (`testing/`。@ugoe/player の偽物) を対象の外にした。
- 見つかったものを直した (許す一覧は空のまま)。
  - エディタ: `preview/dressUp.ts` の `MODULE_FILTER` を消して `filePaths.ts` のものを使う。`viewport/screenshot.ts` の
    `PNG_FILTER` (T0730 の候補) と `preview/trackingHost.ts` の JSON のフィルタを `filePaths.ts` の `PNG_FILTER`・`JSON_FILTER`
    (`fileMenu.filters.png`・`.json`) にし、法線マップの PNG のフィルタもそれに寄せた (`lighting.normalMap.pngFilter` を消した)。
    `document.svelte.ts` のモデルを読めない例外を `app.modelUnreadable` にした。
  - ビューア: JSON の保存のダイアログのフィルタの名前 (`"JSON"`) を `files.saveJson.filter` にした (検査は関数の引数を見ないので
    見つけなかったが、grep で見つけた)。
  - `i18n-ignore` にしたもの: 各アプリの `main.ts` などの `#app not found`、スタジオの `no host` (host が無いときは部品を無効にする)・
    出力ウィンドウの `readFile`・Tauri の外で開いたときの例外、@ugoe/player の `decodeFrame` (ウィンドウの間のフレームの形)、
    @ugoe/face-tracking の `parseRecording` の形の誤り (訳した `errors.recordingInvalid` の後に付ける技術的な補足)、エディタの
    テストの部品 `panels/tree/testModel.ts`。
- テスト: `scripts/check-hardcoded-strings.test.mjs` に 3 件 (テンプレート文字列・例外・フィルタ) を足し、エディタの
  `filePaths.test.ts` に PNG・JSON のフィルタを足した。

### やらなかったこと

- 関数の引数で渡す文言 (ビューアの `saveFileDialog(title, name, filterName, ...)`) は検査で見つけない (受け取る側の名前が
  分からない)。docs/translation.md に書いた。
- @ugoe/face-tracking の記録の形の誤りの文は訳していない (上のとおり `i18n-ignore`)。訳すなら `errors.recording.*` を足す。

### just ci

rebase の後に通った。

起票の候補: なし。
