---
id: T0900
title: UI と i18n: ADR とガイドライン (共有パッケージの構成・デザイントークン・翻訳の仕組み)
status: done
milestone: M8
crates: [docs]
depends_on: []
assignee: claude-worker-T0900
---

# T0900: UI と i18n: ADR とガイドライン (共有パッケージの構成・デザイントークン・翻訳の仕組み)

## 目的

エディタ・ビューア・スタジオで UI の部品と翻訳の仕組みを共有するための方針を決める。いまはエディタが英語・ビューアが日本語の直書きで、
部品 (ボタン・スライダー・ダイアログなど) は各アプリが別々に持っている。

## 受け入れ条件

- [x] ADR 0029 (共有 UI と i18n): パッケージの分け方 (`@ugoe/ui`・`@ugoe/i18n`・`@ugoe/player`。いずれも packages/ で GPL-3.0-only)、
      翻訳の仕組み (将来の翻訳の協力者が使えるよう、デファクトの形式と実装に乗る。既定の案は i18next の JSON (CLDR の複数形) を
      カタログにし、実行は i18next を @ugoe/i18n で包んで Svelte 5 で再描画させる。Paraglide JS・Fluent・ICU MessageFormat と比べ、
      Weblate・Crowdin などの翻訳サービスが読めることを条件に決める。サービスの選定と導入は後で行う)、対応する言語 (日本語と英語。既定は OS の言語)、
      Rust 側のメッセージの扱い (コードと引数を返し、画面で訳す。MCP や CLI の文言は英語のまま)、マニュアルの翻訳を範囲に入れるか
- [x] docs/ui.md: デザイントークン (色・余白・文字・角丸、ライトとダーク)、部品の一覧と使い分け、アクセシビリティの基準 (キーボード・フォーカス・コントラスト)、
      文言の書き方 (翻訳キーの命名・複数形・数値と日付の書式)
- [x] docs/tasks/README.md の ID の表に T08xx (スタジオ)・T09xx (共有 UI・i18n) を足す
- [x] `just ci` が通る

## 参考

- packages/face-tracking/src/strings.ts (いまの ja/en の切り替え)、apps/editor/src/app.css・apps/viewer/src/app.css

## メモ

- やったこと: ADR 0029 (Proposed) と docs/ui.md を書いた。docs/README.md と docs/adr/README.md の表に足した。
  docs/tasks/README.md の ID の表 (T08xx・T09xx) は main で入っていたので確かめただけ。
- 利用者の判断 (2026-10-02): 翻訳は i18next (i18next JSON v4)、部品の土台は Bits UI (推奨の自前から変わった)、マニュアルは M8 では日本語だけで
  docs/manual/en/ の置き場所と言語でのリンクの切り替えだけ決める、原文は英語。ADR 0029 に反映して Accepted にした。
  強調色は、利用者の確認でエディタの紫 (#6d28d9) にそろえることに決まった (スタジオも紫、ビューアの青はやめる)。ADR に決定として書いた。
- Bits UI の依存のライセンスを tarball まで確かめた。svelte-toolbelt は package.json に license が無く (同梱の LICENSE は MIT)、
  `just licenses` では理由付きの例外の一覧に入れることを推奨した (実装は T0901)。版は完全に固定し、人が更新する方針にした。
- 版とライセンスは 2026-10-02 に npm view で確かめた。大きさ (minify と gzip) は scratchpad で esbuild を使って測った (i18next 約 14 KB・
  intl-messageformat 約 10 KB・@fluent/bundle 約 4 KB)。色のトークンのコントラスト比はスクリプトで計算した。
- 後続のタスクとの違い (タスクファイルは直していない):
  - T0902: 1 段目の部品に SegmentedControl と NumberField を挙げた (タスクの「数値の欄」に当たる)。トークンに密度 (`data-density`) を足した。
    受け入れ条件に Bits UI (版の固定) を土台にすることが無い。IconButton はツールチップ (T0903 の段) を使う。
    T0902 は T0901 を待たないので、先に入ると bits-ui の依存のライセンスの検査 (T0901) より前に依存が入る。
  - T0903: Popover (Bits UI) を 2 段目に足し、ColorPicker はその中に作るとした。Dialog は Bits UI の Dialog・AlertDialog を土台にする
    (受け入れ条件の「フォーカスを閉じ込める・Esc で閉じる」は Bits UI が持つ)。Notice・Toast・ColorPicker・DropZone・Kbd は自作。
  - T0904: @ugoe/player は、パネルを部品に移すまで @ugoe/ui に依存しなくてよいとした (T0904 は T0902 を待たないため)。
  - T0901: `scripts/check-licenses.mjs` が packages/ とアプリの npm の依存を検査していないので、i18next を入れるときに検査を足す。
- 後続の候補: テーマ (OS に合わせる・ライト・ダーク) の設定の画面 (どのタスクにも無い)。AGENTS.md の「最初に読むもの」の表に
  docs/ui.md を足す (crates: の範囲外なので触っていない)。
