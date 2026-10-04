---
id: T0925
title: i18n: 言語の設定の保存と OS の言語の取得を @ugoe/i18n にまとめる
status: todo
milestone: M9
crates: [packages, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0913]
assignee:
---

# T0925: i18n: 言語の設定の保存と OS の言語の取得を @ugoe/i18n にまとめる

## 目的

T0913 のレビューで見つけた重複。言語の設定を localStorage に覚えて読む関数 (`loadLocalePreference`・`saveLocalePreference`) と、
起動時に tauri-plugin-os の `locale()` で OS の言語を取って `setSystemLanguages()` に渡す処理が、3 つのアプリにほぼ同じ形で書かれている
(`apps/editor/src/lib/locale.ts`・`apps/viewer/src/lib/settings.ts` と `src/main.ts`・`apps/studio/src/lib/settings.ts` と `src/control.ts`)。
テーマの設定は T0909 で `@ugoe/ui` の `theme.svelte.ts` (キーは `<アプリ>.theme`) にまとめたので、言語も同じ流儀で
`@ugoe/i18n` に置き、アプリはキーの名前 (`ugoe-editor.locale` など) だけを渡す形にする。

## 受け入れ条件

- [ ] `@ugoe/i18n` に、キーを受け取って言語の設定を読む・覚える関数 (使えない localStorage と知らない値では `"system"`) とテストがある
- [ ] OS の言語を渡す処理の置き場所を決めた (`@ugoe/i18n` は Tauri に依存させないなら、取得の関数をアプリから渡す形にする)
- [ ] 3 つのアプリが新しい関数を使い、重複した実装を消した。保存のキーは変えない (覚えた設定を引き継ぐ)
- [ ] packages/i18n/README.md と docs/ui.md を更新した
- [ ] `just ci` が通る

## 参考

- T0913 のメモ (所見の表)
- packages/i18n/README.md の「言語」「OS の言語」、packages/ui/src/theme.svelte.ts (T0909)

## メモ
