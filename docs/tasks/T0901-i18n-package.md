---
id: T0901
title: i18n: 翻訳の共有パッケージ @ugoe/i18n
status: todo
milestone: M8
crates: [packages, scripts, justfile, docs]
depends_on: [T0900]
assignee:
---

# T0901: i18n: 翻訳の共有パッケージ @ugoe/i18n

## 目的

3 つのアプリと共有パッケージが同じ仕組みで日本語と英語を出せるようにする。

## 受け入れ条件

- [ ] packages/i18n: ADR 0029 で決めたデファクトの仕組み (既定の案は i18next) を包み、言語ごとのカタログ (翻訳サービスが読める JSON。型でキーの過不足を検査)、
      引数の差し込み、複数形、数値・日付の書式 (Intl)、言語の検出 (OS / ブラウザ) と上書き、
      切り替えたときの再描画 (Svelte 5 の runes)
- [ ] packages/face-tracking の strings.ts をこの仕組みに移す (表示は変えない)
- [ ] i18next が npm の依存のライセンス検査 (T0902 が作る) を通ること。T0902 より先に入るなら、マージのときに team-lead が確かめる
- [ ] Tauri の各 WebView で OS の言語が取れるかを確かめる
- [ ] カタログのキーの過不足と、未使用のキーを検査する `just i18n` (just ci に入れる)
- [ ] 翻訳の協力者向けの手順 (カタログの置き場所・キーの足し方・訳の確かめ方) を docs に書く
- [ ] Vitest のテストと README。`just ci`・`just licenses` が通る

## 参考

- ADR 0029、docs/ui.md

## メモ
