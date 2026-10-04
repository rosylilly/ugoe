---
id: T0926
title: 同梱物のライセンスの表記: エディタの MIT の JS ライブラリと、アプリ間の写しの検査
status: in-progress
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

- [ ] エディタが、バンドルに入る MIT などの JS ライブラリの表記を画面から読める形で同梱する
- [ ] 3 つのアプリの表記を、ロックファイルと `packages/face-tracking/third-party-notices.txt` から生成するか、食い違いを `just licenses` などで検査する
- [ ] docs/release.md (または関係する文書) に、依存やモデルを更新したときの手順を書いた
- [ ] `just ci` が通る

## 参考

- T0913 のメモ (所見の表)
- `scripts/check-licenses.mjs`、`apps/editor/src/lib/preview/LicenseDialog.svelte`、`apps/viewer/public/third-party-notices.txt`、
  `apps/studio/public/third-party-notices.txt`

## メモ
