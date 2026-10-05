---
id: T0934
title: 読み込みの警告・エラーの訳の表を codes.json と比べるテストと、KTX2 の警告の訳文の直し
status: in-progress
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

- [ ] player とエディタの訳の表が `codes.json` のすべてのコードを持つことを確かめるテストがあり、足りないコードを足した
- [ ] KTX2 の警告の訳文 (en・ja) を「読めない形式」の意味に直した
- [ ] `just i18n` と `just ci` が通る

## 参考

- T0452・T0222 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0452 の訳の表の比較、T0222 の訳文)。エディタの訳の表を触る T1611 の後に着手する。
