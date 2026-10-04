---
id: T0925
title: i18n: 言語の設定の保存と OS の言語の取得を @ugoe/i18n にまとめる
status: done
milestone: M9
crates: [packages, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0913]
assignee: claude-worker-T0925
---

# T0925: i18n: 言語の設定の保存と OS の言語の取得を @ugoe/i18n にまとめる

## 目的

T0913 のレビューで見つけた重複。言語の設定を localStorage に覚えて読む関数 (`loadLocalePreference`・`saveLocalePreference`) と、
起動時に tauri-plugin-os の `locale()` で OS の言語を取って `setSystemLanguages()` に渡す処理が、3 つのアプリにほぼ同じ形で書かれている
(`apps/editor/src/lib/locale.ts`・`apps/viewer/src/lib/settings.ts` と `src/main.ts`・`apps/studio/src/lib/settings.ts` と `src/control.ts`)。
テーマの設定は T0909 で `@ugoe/ui` の `theme.svelte.ts` (キーは `<アプリ>.theme`) にまとめたので、言語も同じ流儀で
`@ugoe/i18n` に置き、アプリはキーの名前 (`ugoe-editor.locale` など) だけを渡す形にする。

## 受け入れ条件

- [x] `@ugoe/i18n` に、キーを受け取って言語の設定を読む・覚える関数 (使えない localStorage と知らない値では `"system"`) とテストがある
- [x] OS の言語を渡す処理の置き場所を決めた (`@ugoe/i18n` は Tauri に依存させないなら、取得の関数をアプリから渡す形にする)
- [x] 3 つのアプリが新しい関数を使い、重複した実装を消した。保存のキーは変えない (覚えた設定を引き継ぐ)
- [x] packages/i18n/README.md と docs/ui.md を更新した
- [x] `just ci` が通る

## 参考

- T0913 のメモ (所見の表)
- packages/i18n/README.md の「言語」「OS の言語」、packages/ui/src/theme.svelte.ts (T0909)

## メモ

- やったこと:
  - `packages/i18n/src/preference.ts` を足した。`loadLocalePreference(key, storage?)`・`saveLocalePreference(key, value, storage?)`
    (テーマの `loadThemePreference`・`saveThemePreference` と同じ形)、起動時の `initLocale(key, systemLanguages?)`、
    設定の画面の `setLocalePreference(value)` (`initLocale()` のキーに覚える。テーマの `setThemePreference` と同じ形)。
  - OS の言語の取得はアプリが関数で渡す (`SystemLanguagesSource`: タグか一覧を返す非同期の関数、取れなければ `null`)。
    `@ugoe/i18n` は Tauri に依存させない。`initLocale()` は拒否しない Promise を返し、ビューアは前と同じく待ってから描く。
    待つ間に呼び直されたら古い呼び出しの結果を捨てる (世代の番号。AGENTS.md の「非同期の開始と停止」)。
  - 3 つのアプリの重複 (エディタの `lib/locale.ts` の読み書き・`changeLocale`・`initLocale`、ビューア・スタジオの `settings.ts` の
    読み書きと `main.ts`・`control.ts` の OS の言語の処理、設定のダイアログの `$state` の写し) を消した。エディタの `lib/locale.ts` には
    キーと tauri-plugin-os から取る関数だけが残る。ビューア・スタジオは `host` の `systemLocale()` を渡す。
  - 保存のキーと値は変えていない (`ugoe-editor.locale`・`ugoe-viewer.locale`・`ugoe-studio.locale`、値は `system`・`en`・`ja`)。
    `preference.test.ts` で前からの値を読めること、各アプリのテストでキーが前のままであることを確かめた。移す処理は要らない。
  - packages/i18n/README.md (「言語」「OS の言語」とファイルの一覧)・docs/ui.md (テーマの節の「言語も同じ流儀」)・docs/editor.md (言語) を更新した。
- やらなかったこと:
  - テーマのキーの入れ物をまとめること (オーケストレーターの補足の T0930 の候補)。テーマは T0909 で `@ugoe/ui` の `initTheme(key)` に
    まとまっていて、アプリにはキーの定数 (`THEME_STORAGE_KEY`。index.html の <head> のスクリプトと同じ値) しか残っていない。
    `@ugoe/player` の `PlayerStorage` は Player が覚えるものの名前 (`STORAGE_NAMES`) に限った入れ物で、言語・テーマを入れると
    i18n・ui が player の型に寄るので入れなかった。ビューア・スタジオのテストでテーマのキーが前のままであることも確かめるようにした。
  - 実機の Tauri (tauri-plugin-os の `locale()`) での起動は確かめていない (コードの流れは前と同じで、関数の渡し方だけが変わった)。
- 起票の候補: なし。
- `just ci`: 通った (rebase の時点で main と差分なし)。

