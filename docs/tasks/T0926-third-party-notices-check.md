---
id: T0926
title: 同梱物のライセンスの表記: エディタの MIT の JS ライブラリと、アプリ間の写しの検査
status: done
milestone: M9
crates: [packages, apps/editor, apps/viewer, apps/studio, scripts, docs]
depends_on: [T0913]
assignee: claude-worker-T0926
---

# T0926: 同梱物のライセンスの表記: エディタの MIT の JS ライブラリと、アプリ間の写しの検査

## 目的

T0913 のレビュー (third-party-notices の網羅) で見つけたこと。

- エディタは `@ugoe/face-tracking` の `third-party-notices.txt` (MediaPipe だけ) を「Face tracking licenses」として出すが、
  バンドルに入る MIT の JS ライブラリ (svelte・bits-ui とその依存・`@ugoe/i18n` の i18next など) の表記がどこにも無い。
  ビューアとスタジオは `public/third-party-notices.txt` に書いている。MIT は写しに著作権の表示と許諾の文を含めることを求める。
- ビューアとスタジオの `public/third-party-notices.txt` は、MediaPipe の節を `packages/face-tracking/third-party-notices.txt` から
  手で写し、JS ライブラリの版 (svelte 5.57.1 など) も手で書いている。モデルや依存を更新したときに食い違っても気づけない。

## 受け入れ条件

- [x] エディタが、バンドルに入る MIT などの JS ライブラリの表記を画面から読める形で同梱する
- [x] 3 つのアプリの表記を、ロックファイルと `packages/face-tracking/third-party-notices.txt` から生成するか、食い違いを `just licenses` などで検査する
- [x] docs/release.md (または関係する文書) に、依存やモデルを更新したときの手順を書いた
- [x] `just ci` が通る

## 参考

- T0913 のメモ (所見の表)
- `scripts/check-licenses.mjs`、`apps/editor/src/lib/preview/LicenseDialog.svelte`、`apps/viewer/public/third-party-notices.txt`、
  `apps/studio/public/third-party-notices.txt`

## メモ

### やったこと

- `scripts/third-party-notices.mjs` を足した。アプリごとに `vite build` を書き出さずに走らせ (1 つ 1〜2 秒)、バンドルに入ったモジュールの
  node_modules のパッケージを集め、名前・版・ライセンス・ソースとパッケージのライセンスの文 (LICENSE をそのまま) を並べる。
  MediaPipe の節は `packages/face-tracking/third-party-notices.txt` の最初の節から終わりまでを使う (`@mediapipe/tasks-vision` は JS の節に重ねない)。
  `--check` で生成したものとコミットしたものを突き合わせ、`just licenses` (CI の rust-checks のジョブ・`just ci`) で流す。部品のテストは `scripts/third-party-notices.test.mjs`。
  - 依存の閉包 (dependencies を推移的にたどる) ではなくバンドルから集めるのは、閉包だと svelte のコンパイラの依存 (acorn・magic-string・typescript など)
    まで載るため。バンドルから集めた結果は、手で書いていた一覧と同じ (+ エディタの `@tauri-apps/plugin-opener`)。`@internationalized/date` は
    tree-shaking で入っていない。
  - MIT と別のライセンスから選べるもの (`@tauri-apps/api`) は MIT の文のファイルを載せる。文のファイルを持たない Tauri のプラグイン
    (`plugin-dialog`・`plugin-opener`・`plugin-os`) は `LICENSE.spdx` の著作権者 (2019-2022, The Tauri Programme in the Commons Conservancy) と MIT の文を載せる。
    文のファイルが無く扱いも決まっていないパッケージがバンドルに入ると、生成が失敗する。
  - `svelte-toolbelt` (package.json に license が無い) は `LICENSE_OVERRIDES` で MIT とする (`check-licenses.mjs` の NPM_EXCEPTIONS と同じ。向こうのコメントにも書いた)。
- 3 つのアプリの `public/third-party-notices.txt` を生成した (エディタは新規)。ビューア・スタジオの表記は、各ライブラリの著作権の表示を抜き書きする形から、
  ライセンスの文をそのまま載せる形に変わった。
- エディタのプレビューの「Third-party licenses」(`preview/LicenseDialog.svelte`) が `mediapipe/third-party-notices.txt` ではなく
  `third-party-notices.txt` を読むようにし、題と説明の文 (en・ja) を MediaPipe だけでない形に直した。アプリ全体のライセンスの画面は T0673 (todo) でまだ無い。
- 文書: `docs/release.md#同梱物のライセンスの表記` (生成のしかたと、依存・モデルを更新したときの手順)、`docs/editor.md`・`docs/studio.md`、
  3 つのアプリの README、`AGENTS.md` の `just licenses` の行 (範囲の外だが 1 行の追記)。

### やらなかったこと・確かめていないこと

- Rust のクレート (Tauri のバックエンド) の表記は Release の T0005 で扱う (オーケストレーターの指示)。
- 仮想のモジュール (Vite の preload の補助・rolldown の実行時の補助) は載せていない (ツールが足す数行の補助。release.md に書いた)。
- 実際のインストーラを作って画面に出るかは確かめていない (`vite build` の出力に `third-party-notices.txt` が入ること、CSP の `default-src 'self'` で読めることはビューア・スタジオと同じ)。
- `packages/face-tracking/scripts/mediapipe-assets.mjs` は今も `public/mediapipe/third-party-notices.txt` を写す (エディタは読まなくなった。害は無いので残した)。

### 起票の候補

- T0673 (エディタの外枠) で、設定などからアプリ全体のライセンスの画面を開けるようにするとき、この `third-party-notices.txt` を読む (プレビューの中だけでなく)。
- `NPM_EXCEPTIONS` (check-licenses.mjs) と `LICENSE_OVERRIDES` (third-party-notices.mjs) を 1 か所 (scripts/lib) にまとめる。

`just ci`: 通った (rebase 後、main ebd4d439 の上で)。
