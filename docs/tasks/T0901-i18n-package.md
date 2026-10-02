---
id: T0901
title: i18n: 翻訳の共有パッケージ @ugoe/i18n
status: done
milestone: M8
crates: [packages, scripts, justfile, docs, apps/editor, apps/viewer, pnpm-lock.yaml, lefthook.yml, .github/workflows/ci.yml]
depends_on: [T0900]
assignee: claude-worker-T0901
---

# T0901: i18n: 翻訳の共有パッケージ @ugoe/i18n

## 目的

3 つのアプリと共有パッケージが同じ仕組みで日本語と英語を出せるようにする。

## 受け入れ条件

- [x] packages/i18n: ADR 0029 で決めたデファクトの仕組み (既定の案は i18next) を包み、言語ごとのカタログ (翻訳サービスが読める JSON。型でキーの過不足を検査)、
      引数の差し込み、複数形、数値・日付の書式 (Intl)、言語の検出 (OS / ブラウザ) と上書き、
      切り替えたときの再描画 (Svelte 5 の runes)
- [x] packages/face-tracking の strings.ts をこの仕組みに移す (表示は変えない)
- [ ] i18next が npm の依存のライセンス検査 (T0902 が作る) を通ること。T0902 より先に入るなら、マージのときに team-lead が確かめる
- [ ] Tauri の各 WebView で OS の言語が取れるかを確かめる
- [x] カタログのキーの過不足と、未使用のキーを検査する `just i18n` (just ci に入れる)
- [x] 翻訳の協力者向けの手順 (カタログの置き場所・キーの足し方・訳の確かめ方) を docs に書く
- [x] Vitest のテストと README。`just ci`・`just licenses` が通る

## 参考

- ADR 0029、docs/ui.md

## メモ

- **範囲の追加 (team-lead の承認済み):** face-tracking の `strings.ts` を消すと、エディタの `import { en }` と `strings:` が壊れ、
  既定 (OS の言語) では表示が変わる。そのため apps/editor と apps/viewer を足し、変更を次に留めた: 両アプリの `main.ts` で
  `setLocale("en")` / `setLocale("ja")` (今の表示を保つ。切り替えの画面は後続)、エディタの `preview.svelte.ts` から `en` と `strings:` を外す、
  両アプリの `package.json` に `@ugoe/i18n`。ビューアは T0904 と衝突しないよう main.ts と package.json だけ。
  justfile・ci.yml・lefthook.yml の i18n の検査は face-tracking の行の隣に置いた。
- **やったこと:** packages/i18n (i18next 26.4.2 を包む。`registerCatalogs`・`translator` (キーは `CatalogTypes` から i18next の
  `ResourceNamespaceMap` で型検査)・`setLocale` / `setSystemLanguages` / `getLocale` (`$state`)・`formatNumber` など (Intl)・
  `RichText` (訳文の印をスニペットで描く)・`languages.json` (対応する言語の一覧))。`just i18n` (packages/i18n/scripts/check-catalogs.mjs。
  キーの過不足・複数形の CLDR の区分・引数・空の訳・未使用のキー)。face-tracking の文言を locales/en.json・ja.json に移した。
  旧 `strings.ts` の全 47 項目と新しいカタログの出力を、両言語で一時的なテストで突き合わせて一致を確かめた (テストはコミットしていない)。
  翻訳の協力者向けの手順は docs/translation.md。docs/ui.md から参照と `RichText` の名前を書いた。
- **版:** i18next は `^26.4.2` (ロックファイルで固定)。ADR 0029 で完全に固定すると決めたのは Bits UI だけのため。
- **ライセンス:** i18next 26.4.2 は MIT、実行時の依存なし (`npm view` と node_modules の package.json で確かめた)。npm の依存の
  ライセンス検査は T0902 が作るので、scripts/check-licenses.mjs は触っていない。いまの `just licenses` は通る。T0902 の検査が
  入った後に通るかはマージのときに確かめる (受け入れ条件の 3 つ目は未チェック)。
- **OS の言語:** WebKitGTK 2.52.6 は PyGObject で直接確かめた (navigator.languages はプロセスのロケールの 1 つだけ、`LANGUAGE` は読まない)。
  WebView2 と WKWebView は手元に無く未確認 (報告と文書へのリンクを packages/i18n/README.md に書いた)。どれも OS の優先する言語の一覧は
  取れないので、アプリの Rust 側で OS の言語を取り (`sys-locale` クレートなど。外部依存の追加なので利用者の判断)、`setSystemLanguages()` に渡すのを後続に回す
  (受け入れ条件の 4 つ目は、Linux だけ確かめたので未チェック)。
- **やらなかったこと:** 画面のスクリーンショットでの確認 (文言の一致はテストで確かめた)。引数の名前の型検査 (カタログを JSON から読むため、
  型では検査できない。言語の間の食い違いは `just i18n` が見る)。エラーの文は起きた時の言語のまま (切り替えても変わらない)。
- **後続の候補:** 言語の切り替えの画面 (設定に「OS に合わせる・日本語・English」、覚える) と、Rust 側で OS の言語を取って
  `setSystemLanguages()` に渡すこと。エディタ・ビューアの直書きの文言のカタログへの移行 (T0908 の検査と合わせて)。

