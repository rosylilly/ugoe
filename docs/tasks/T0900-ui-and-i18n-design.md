---
id: T0900
title: UI と i18n: ADR とガイドライン (共有パッケージの構成・デザイントークン・翻訳の仕組み)
status: todo
milestone: M8
crates: [docs]
depends_on: []
assignee:
---

# T0900: UI と i18n: ADR とガイドライン (共有パッケージの構成・デザイントークン・翻訳の仕組み)

## 目的

エディタ・ビューア・スタジオで UI の部品と翻訳の仕組みを共有するための方針を決める。いまはエディタが英語・ビューアが日本語の直書きで、
部品 (ボタン・スライダー・ダイアログなど) は各アプリが別々に持っている。

## 受け入れ条件

- [ ] ADR 0029 (共有 UI と i18n): パッケージの分け方 (`@ugoe/ui`・`@ugoe/i18n`・`@ugoe/player`。いずれも packages/ で GPL-3.0-only)、
      翻訳の仕組み (将来の翻訳の協力者が使えるよう、デファクトの形式と実装に乗る。既定の案は i18next の JSON (CLDR の複数形) を
      カタログにし、実行は i18next を @ugoe/i18n で包んで Svelte 5 で再描画させる。Paraglide JS・Fluent・ICU MessageFormat と比べ、
      Weblate・Crowdin などの翻訳サービスが読めることを条件に決める。サービスの選定と導入は後で行う)、対応する言語 (日本語と英語。既定は OS の言語)、
      Rust 側のメッセージの扱い (コードと引数を返し、画面で訳す。MCP や CLI の文言は英語のまま)、マニュアルの翻訳を範囲に入れるか
- [ ] docs/ui.md: デザイントークン (色・余白・文字・角丸、ライトとダーク)、部品の一覧と使い分け、アクセシビリティの基準 (キーボード・フォーカス・コントラスト)、
      文言の書き方 (翻訳キーの命名・複数形・数値と日付の書式)
- [ ] docs/tasks/README.md の ID の表に T08xx (スタジオ)・T09xx (共有 UI・i18n) を足す
- [ ] `just ci` が通る

## 参考

- packages/face-tracking/src/strings.ts (いまの ja/en の切り替え)、apps/editor/src/app.css・apps/viewer/src/app.css

## メモ
