---
id: T0909
title: UI: テーマの切り替えの設定 (OS に合わせる・ライト・ダーク) を 3 つのアプリに
status: done
milestone: M8
crates: [packages, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0723, T0663, T0801]
assignee: claude-worker-T0909
---

# T0909: テーマの切り替え

## 目的

docs/ui.md のトークンはライトとダークを持つが、利用者が選ぶ画面がどのタスクにも無い (T0900 のメモ)。

## 受け入れ条件

- [x] @ugoe/ui にテーマの設定の部品と保存の仕組み (OS に合わせる・ライト・ダーク)
- [x] エディタ・ビューア・スタジオの設定に置く
- [x] `just ci` が通る

## 参考

- docs/ui.md

## メモ

- `@ugoe/ui` に `theme.svelte.ts` (`initTheme(key)`・`setThemePreference`・`getThemePreference`・`currentTheme`・
  `load/saveThemePreference`) と部品 `ThemeSetting` (SegmentedControl で「OS に合わせる・ライト・ダーク」) を足した。文言は名前空間 `ui` の `theme.*`。
  「OS に合わせる」は `data-theme` を外すだけ (tokens.css の `prefers-color-scheme` がそのまま追従する)。`currentTheme()` は matchMedia を見張り、
  JS で CSS の変数を読むところが変化を知るのに使う。Vitest (`theme.test.ts`) で保存・<html> への反映・OS への追従・部品の操作を検査。
- 保存のキーは `ugoe-editor.theme`・`ugoe-viewer.theme`・`ugoe-studio.theme` (言語の `<アプリ>.locale` と同じ流儀)。
- ちらつき: 3 つのアプリの `index.html` の `<head>` に、同じキーを読んで先に `data-theme` を付ける小さなスクリプトを置いた (CSP は null)。
- 置き場所: ビューアは設定のダイアログ (言語の下)、エディタは「General」のタブ (LanguageSetting の下に 1 行足しただけ)、スタジオは操作ウィンドウのヘッダー (言語の隣)。
  スタジオの出力ウィンドウ (`output.ts`・`output.html`) は @ugoe/ui を読まないので、テーマは効かず透過のまま。
- エディタのビューポートの背景は、前は OS の `prefers-color-scheme` の変化だけで読み直していたので、`currentTheme()` の `$effect` に替えた。
- 確認: ビルドした 3 つのアプリをヘッドレスの Chrome で開き (エディタは T0663 のブリッジ、スタジオは偽の StudioHost)、明示のテーマを OS と逆にして撮った。
  JS の module を止めても `<head>` のスクリプトで `data-theme` が付くこと、部品で切り替えると <html> と localStorage が変わること、
  「OS に合わせる」で OS の変化に追従すること、出力ウィンドウが透過のままで `data-theme` を付けないことを検査した。
- 確かめていないこと: 実機の Tauri の WebView (WebView2・WKWebView・WebKitGTK) での見た目。ネイティブのタイトルバーはテーマに合わせていない。
- 後続の候補: Tauri の `Window.setTheme()` でタイトルバーもテーマに合わせる (権限 `core:window:allow-set-theme` が要る)。
  窓が開いてから WebView が描くまでの地の色 (tauri.conf.json の `backgroundColor`) をテーマに合わせる。
