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

- [ ] packages/ui: デザイントークン (CSS 変数。ライトとダーク)、基本の部品 (Button・IconButton・Checkbox / Toggle・Slider と NumberField (リセット付き)・Select・Tabs・SegmentedControl・
      折りたためる Section・Toolbar)。bits-ui を土台にし (ADR 0029)、無い部品は自作する。密度 (data-density) とテーマの上書き (data-theme)。キーボードで操作でき、aria を付ける
- [ ] bits-ui の版を完全に固定する (ADR 0029)。scripts/check-licenses.mjs で packages/ とアプリの npm の依存も検査する仕組みを作り (svelte-toolbelt は理由と版を書いた例外の一覧。ADR 0029)、`just licenses` で bits-ui とその依存を検査する
- [ ] IconButton は aria-label を必ず付ける (ツールチップは T0903 で足す)
- [ ] 部品の見本の画面 (gallery。vite で開ける) と、ヘッドレスの Chrome でのスクリーンショットの検査
- [ ] 文言は @ugoe/i18n を通す (部品の中の固定の文言があれば)
- [ ] Vitest のテストと README。`just ci` が通る

## 参考

- ADR 0029、docs/ui.md、apps/editor/src/app.css (いまの CSS 変数)

## メモ
