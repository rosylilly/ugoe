---
id: T0902
title: UI: 共有部品 @ugoe/ui (デザイントークンと基本の部品・部品の見本の画面)
status: done
milestone: M8
crates: [packages, scripts, justfile, docs, .github/workflows]
depends_on: [T0900]
assignee: claude-worker-T0902
---

# T0902: UI: 共有部品 @ugoe/ui (デザイントークンと基本の部品・部品の見本の画面)

## 目的

3 つのアプリで見た目と操作を揃える土台を作る。

## 受け入れ条件

- [x] packages/ui: デザイントークン (CSS 変数。ライトとダーク)、基本の部品 (Button・IconButton・Checkbox / Toggle・Slider と NumberField (リセット付き)・Select・Tabs・SegmentedControl・
      折りたためる Section・Toolbar)。bits-ui を土台にし (ADR 0029)、無い部品は自作する。密度 (data-density) とテーマの上書き (data-theme)。キーボードで操作でき、aria を付ける
- [x] bits-ui の版を完全に固定する (ADR 0029)。scripts/check-licenses.mjs で packages/ とアプリの npm の依存も検査する仕組みを作り (svelte-toolbelt は理由と版を書いた例外の一覧。ADR 0029)、`just licenses` で bits-ui とその依存を検査する
- [x] IconButton は aria-label を必ず付ける (ツールチップは T0903 で足す)
- [x] 部品の見本の画面 (gallery。vite で開ける) と、ヘッドレスの Chrome でのスクリーンショットの検査
- [x] 文言は @ugoe/i18n を通す (部品の中の固定の文言があれば)
- [x] Vitest のテストと README。`just ci` が通る

## 参考

- ADR 0029、docs/ui.md、apps/editor/src/app.css (いまの CSS 変数)

## メモ

- **やったこと:**
  - `packages/ui` (`@ugoe/ui`、GPL-3.0-only): `src/tokens.css` (docs/ui.md の値。ライト・ダークは OS の設定と `data-theme`、
    密度は `data-density`。属性を省くと compact。派生の色 (`-soft`・`focus`) は `data-theme` の要素ごとに作り直す)。
    部品は Button・IconButton・Checkbox・Toggle・Slider (NumberField とリセット付き)・NumberField (自作、APG の Spinbutton)・Select・
    SegmentedControl (Bits UI の RadioGroup)・Tabs・Section (Collapsible、見出しの中のボタン)・Toolbar (中の Button / IconButton は
    context で Bits UI の Toolbar.Button になり、roving tabindex)。props は ugoe の言葉で決め、Bits UI の props は外に出さない。
  - `bits-ui` は 2.19.4 に完全に固定 (peer の `@internationalized/date` も 3.12.4 に固定)。
  - Bits UI 2.19.4 の Select の項目は無効でも `aria-disabled` を付けないので、`@ugoe/ui` で付けた (docs/ui.md に例として書いた)。
    Select のトリガーは `role="combobox"` ではなく `aria-haspopup="listbox"` のボタン。
  - Vitest (happy-dom の上で svelte の `mount`。@testing-library は入れない): 29 件。数値の解釈・書式と、各部品のキーボード操作と aria
    (ラベルの結び付け・roving tabindex・Esc で戻す・Select をキーで開いて選ぶなど)。
  - 見本の画面 `packages/ui/gallery` (`pnpm --filter @ugoe/ui gallery`。`?theme=&density=&lang=`) と、
    `just ui-shots <出力先>` (`packages/ui/scripts/shots.mjs`): 書き出した見本を vite preview で配り、ヘッドレスの Chrome で
    テーマ × 密度 × 言語の 8 通りと 360px 幅を撮る。DOM を書き出して、描き終わり・エラー無し・`<html>` の属性・
    操作できる部品すべての読み上げの名前を検査する。CI には入れていない (Chrome が要る)。
  - `scripts/check-licenses.mjs`: apps/*・packages/* の実行時の npm の依存 (dependencies・optionalDependencies・入っている peer) を
    node_modules から推移的にたどり、GPL_COMPATIBLE で検査する。bindings/web も推移的にたどる形にした (基準は PERMISSIVE のまま)。
    SPDX の式の無い依存は `NPM_EXCEPTIONS` (名前・確かめた版・実際のライセンス・確かめ方) に登録し、版が変わると失敗する。
    svelte-toolbelt 0.10.6 の LICENSE が MIT であることを node_modules の実物でも確かめた。
  - `just web-check` に @ugoe/ui の check と test を足した。docs/ui.md (密度の既定・確かめ方)・docs/testing.md を更新。
- **devDependency の追加:** `happy-dom` (MIT。部品のテストと、撮った画面の DOM の検査に使う。利用者の決定)。
  happy-dom 20.14 は ResizeObserver・scrollIntoView・pointer capture・getAnimations を持つので、bits-ui の部品は代わりの実装なしで動いた。
  Section を閉じたときの中身は、Bits UI が閉じる動きを待ってから `hidden` にするので、テストは `vi.waitFor` で待つ。
- **範囲外の変更 (承認済み):** `.github/workflows/ci.yml` の rust-checks ジョブに pnpm のセットアップと `pnpm install --frozen-lockfile` を足した。
  `just licenses` が npm の依存 (node_modules) を読むようになり、インストールしないと失敗するため。ほかのジョブは触っていない。
- **文言:** 部品の文言は props で受ける。部品が既定で出す文言 (Slider のリセットの名前) だけ、@ugoe/i18n (T0901) の名前空間 `ui`
  (`packages/ui/locales/`) から引き、props で上書きできる。見本の画面は `setLocale()` で言語を切り替える。npm のライセンス検査で
  i18next 26.4.2 (MIT) がたどられて通ることを確かめた。
- **確かめていないこと:** 本物の WebView (WebView2・WKWebView・WebKitGTK) での表示と、スクリーンリーダーでの読み上げ。
  ポインタでのスライダーのドラッグ (happy-dom では検査していない。見本の画面で手で触った確認もしていない)。
  見本の画面の撮影は scratchpad/T0902/shots/ に 9 枚 (すべて ok)。
- **後続の候補:** T0903 (Tooltip を IconButton に付ける・2 段目の部品)。apps に bits-ui などの第三者のライセンスの表記を出す (アプリの third-party notices。T0723・T0663 で移すとき)。
  AGENTS.md の `just licenses` の説明 (「依存するクレートの」) に npm も足す (範囲外なので触っていない)。
