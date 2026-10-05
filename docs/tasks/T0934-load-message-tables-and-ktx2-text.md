---
id: T0934
title: 読み込みの警告・エラーの訳の表を codes.json と比べるテストと、KTX2 の警告の訳文の直し
status: done
milestone: M9
crates: [packages/player, apps/editor, docs]
depends_on: [T1611]
assignee: claude-worker-T0934
---

# T0934: 読み込みの警告・エラーの訳の表を codes.json と比べるテストと、KTX2 の警告の訳文の直し

## 目的

- `packages/player` の `loadError.ts`・`loadWarnings.ts` とエディタのプレビューの訳の表が、wasm のすべてのコード (`crates/ugoe-wasm/codes.json`、
  `@ugoe/runtime` の `LOAD_ERROR_CODES`・`LOAD_WARNING_CODES`・`DRESS_UP_ERROR_CODES`) を持つかを確かめるテストが無い (T0452 の候補)。
- `textureKtx2`・`normalMapKtx2` の訳文 (「KTX2 の画像で、まだ表示できない」) は、T0222 で非圧縮の KTX2 を読めるようになった今は不正確。
  「読めない形式の KTX2」に合わせて直す (`packages/player/locales/*.json`・`apps/editor/locales/*.json`。T0222 の候補)。

## 受け入れ条件

- [x] player とエディタの訳の表が `codes.json` のすべてのコードを持つことを確かめるテストがあり、足りないコードを足した
- [x] KTX2 の警告の訳文 (en・ja) を「読めない形式」の意味に直した
- [x] `just i18n` と `just ci` が通る

## 参考

- T0452・T0222 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0452 の訳の表の比較、T0222 の訳文)。エディタの訳の表を触る T1611 の後に着手する。

- やったこと:
  - `packages/player/src/codes.test.ts`: `codes.json` の `load_error`・`dress_up_error`・`load_warning` のすべてのコードと補足の値の組
    (`image`・`limit` は取りうる値をすべて) を、すべての言語で `describeLoadError`・`describeDressUpError`・`describeLoadWarnings` が
    訳すか (wasm の文のまま・キーのまま・`{{` が残る・原文の言語のほかで原文と同じ、を落とす) を確かめる。
  - `apps/editor/src/lib/preview/codes.test.ts`: 同じ形でプレビューの `describeLoadWarnings` (`module_detached` は着せ替えが知らせるので除く) と
    `describeProblem` (着せ替えの失敗) を確かめる。読み込みの失敗は T1611 の `viewport/loadError.test.ts` が確かめているので足していない。
  - 足りなかったコード: player の着せ替えの `device_lost` (モデルの読み込みと同じ `loadError.deviceLost` の文にした)。エディタのプレビューの
    着せ替えの `read_limit_exceeded`・`image_limit_exceeded` (@ugoe/player の `describeLimitError` でモデルの読み込みと同じ文に) と
    `device_lost` (`preview.dressUp.problem.deviceLost` を足した)。
  - KTX2 の警告 (`textureKtx2`・`normalMapKtx2`) の訳文 (en・ja、player とエディタ) を、T0223 に合わせて「読めない形式
    (ETC2・ASTC・Basis など) の KTX2」にした。BC6H は利用者に分かりにくいので例に挙げていない。
- 確かめたこと: `just i18n`・`just ci` が通った。
- やらなかったこと: ビューア・スタジオに独自の訳の表は無い (player の表を使う) ので触っていない。
