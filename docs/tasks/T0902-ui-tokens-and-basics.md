---
id: T0902
title: UI: 共有部品 @ugoe/ui (デザイントークンと基本の部品・部品の見本の画面)
status: todo
milestone: M8
crates: [packages, scripts, justfile, docs]
depends_on: [T0900]
assignee:
---

# T0902: UI: 共有部品 @ugoe/ui (デザイントークンと基本の部品・部品の見本の画面)

## 目的

3 つのアプリで見た目と操作を揃える土台を作る。

## 受け入れ条件

- [ ] packages/ui: デザイントークン (CSS 変数。ライトとダーク)、基本の部品 (Button・IconButton・Checkbox / Toggle・Slider と数値の欄 (リセット付き)・Select・Tabs・
      折りたためる Section・Toolbar)。キーボードで操作でき、aria を付ける
- [ ] 部品の見本の画面 (gallery。vite で開ける) と、ヘッドレスの Chrome でのスクリーンショットの検査
- [ ] 文言は @ugoe/i18n を通す (部品の中の固定の文言があれば)
- [ ] Vitest のテストと README。`just ci` が通る

## 参考

- ADR 0029、docs/ui.md、apps/editor/src/app.css (いまの CSS 変数)

## メモ
