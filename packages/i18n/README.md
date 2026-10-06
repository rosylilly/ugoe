# packages/i18n

エディタ・ビューア・スタジオと共有パッケージで使う、画面の文言の翻訳 (npm パッケージ `@ugoe/i18n`、pnpm のワークスペース)。
[i18next](https://www.i18next.com/) を包み、言語の検出と切り替え・数値と日付の書式・カタログの型・カタログの検査を出す。
方針は [ADR 0029](../../docs/adr/0029-shared-ui-and-i18n.md)、文言の書き方は [docs/ui.md](../../docs/ui.md#文言の書き方)、
翻訳する人向けの手順は [docs/translation.md](../../docs/translation.md)。

**ライセンスは GPL-3.0-only** (使うアプリと同じ)。依存の i18next は MIT (実行時の依存なし)。

```
src/i18n.svelte.ts      i18next の 1 つのインスタンスと、今の言語 ($state)。registerCatalogs・translator・setLocale・setSystemLanguages
src/languages.json      対応する言語の一覧 (原文の言語・言語自身での名前・欠けのない訳として検査するか)。検査のスクリプトも読む
src/languages.ts        上の一覧の型 (Locale・LocalePreference)
src/preference.ts       言語の設定を覚える・起動時に戻す (アプリが渡すキーで localStorage に)。initLocale・setLocalePreference
src/storage.ts          localStorage の読み書き (例外を出さない)。readStoredItem・writeStoredItem・removeStoredItem (T0937)
src/testing.ts          テスト用の部品 (`@ugoe/i18n/testing`)。MemoryStorage・BrokenStorage・useLocalStorage
src/detect.ts           言語の検出 (BCP 47 の言語の部分で、対応する最初の言語を選ぶ)
src/format.ts           数値・日付・相対時間・一覧の書式 (Intl を今の言語で使う)
src/rich.ts / RichText.svelte  訳文の中の印 (`<link>…</link>`) をスニペットで描く
scripts/check-catalogs.mjs     カタログの検査 (`just i18n`)
test/client-environment.ts     Vitest の環境 (runes を client 向けにコンパイルする)
```

## 使い方

### パッケージ・アプリの文言

カタログは `<パッケージ>/locales/<言語>.json` ([i18next JSON v4](https://www.i18next.com/misc/json-format))。原文は `en.json`。
名前空間はパッケージの短い名前 (`face-tracking`・`ui`・`player`・`editor`・`viewer`・`studio`)。カタログを読む所を 1 つ作り、`t()` を出す。

```ts
// packages/face-tracking/src/i18n.ts
import { registerCatalogs, translator } from "@ugoe/i18n";
import en from "../locales/en.json";
import ja from "../locales/ja.json";

declare module "@ugoe/i18n" {
  interface CatalogTypes {
    "face-tracking": typeof en; // t() のキーを原文のカタログで検査する
  }
}

registerCatalogs("face-tracking", { en, ja });
export const t = translator("face-tracking");
```

```svelte
<button title={t("calibration.calibrate.tooltip")}>{t("calibration.calibrate.label")}</button>
<span>{t("recording.stop.label", { frames: count })}</span>
```

- `t()` は呼ぶたびに今の言語を読むので、テンプレートと `$derived` の中で使えば言語の切り替えで再描画される。
  `const label = t(...)` のように 1 度だけ評価した値は追従しない (クラスのフィールドや例外の文に入れた文は、作った時の言語のまま)。
- 複数形は引数 `count` を渡し、カタログに `_one`・`_other` などを書く。書式は `{{value, number}}` か `formatNumber()` など。
- 文の一部をリンクなどにするときは、訳文に印を書き、`RichText` で描く:
  `<RichText text={t("notice.manual")}>{#snippet link(text)}<a href={url}>{text}</a>{/snippet}</RichText>`。
- `tsconfig.json` に `"resolveJsonModule": true` が要る。

### 言語

- 既定は「OS に合わせる」(`"system"`): OS の優先する言語を前から見て、最初に対応している言語を使う。どれも対応していなければ英語。
- `setLocale("ja" | "en" | "system")` で上書きする (覚えない)。`getLocale()` が今の言語、`getLocalePreference()` が今の設定、`LANGUAGES` が選択肢 (言語自身の名前)。
- 設定を覚えるのは `src/preference.ts` (T0925。テーマの `@ugoe/ui` の `initTheme` と同じ流儀)。アプリはキー (`<アプリ>.locale`) を渡す:

  ```ts
  // main.ts (mount の前に 1 回)。覚えた設定を使い、OS の言語を取る関数があれば取って setSystemLanguages() に渡す。
  void initLocale("ugoe.editor.locale", systemLocale); // 最初の画面から OS の言語で出すなら await する (ビューア)
  ```

  設定の画面は `getLocalePreference()` を選択の値にし、`setLocalePreference(value)` で切り替えて `initLocale()` のキーに覚える。
  キーを指定して読み書きするだけなら `loadLocalePreference(key, storage?)`・`saveLocalePreference(key, value, storage?)`。
  使えない localStorage (例外を出す・無い) では覚えず、覚えていない・知らない値は `"system"` として読む。
  キーはアプリごと (`ugoe.editor.locale`・`ugoe.viewer.locale`・`ugoe.studio.locale`) で、値は `system`・`en`・`ja`。
- `<html lang>` は今の言語に合わせる。

### localStorage の読み書き (T0937)

- 設定を localStorage に覚える処理は `src/storage.ts` の `readStoredItem(key, storage?)`・`writeStoredItem(key, value, storage?)`・
  `removeStoredItem(key, storage?)` を通す。言語 (`preference.ts`)・テーマ (`@ugoe/ui`)・Player が覚えるもの (`@ugoe/player` の
  `PlayerStorage`)・スタジオの古い設定の片付けが使う。翻訳とは別の役目だが、3 つのアプリとすべての共有の部品が依存する一番下の
  パッケージなのでここに置く。
- localStorage を引くだけで例外になる環境 (保存を禁止したブラウザ)・無い環境・容量の超過でも例外を出さない。読めなければ `null`
  (覚えていない)、書けなければ何もしない。`storage` を省くと呼ぶたびに `globalThis.localStorage` を引く。
- テストは `@ugoe/i18n/testing` の `MemoryStorage` (Map に覚える `Storage`)・`BrokenStorage` (すべて例外を出す)・
  `useLocalStorage(storage)` (`globalThis.localStorage` を置き換える) を使う。パッケージとアプリのテストで写しを持たない。
  画面のコードからは import しない。
- ビューアは設定の言語 (OS に合わせる・日本語・English) で切り替える (T0723)。エディタは設定の「General」の言語 (OS に合わせる・English・日本語) で切り替える (T0663)。

### OS の言語

既定では WebView の `navigator.languages` を OS の言語として使う。アプリが OS から取った一覧 (BCP 47 のタグ。前ほど優先) を
`setSystemLanguages()` で渡すと、そちらを使う (WebView の変化は追わなくなる)。

Tauri の WebView で確かめたこと (2026-10-02):

| WebView | `navigator.languages` | 確かめ方 |
| --- | --- | --- |
| WebKitGTK (Linux) 2.52.6 | プロセスのロケール (`LC_ALL` / `LANG`) の 1 つだけ。`LANGUAGE` の一覧は読まない。ロケールが入っていなければ `en-US` | PyGObject で WebKit2 4.1 の WebView を作り、環境変数を変えて読んだ (WSL2)。`ja_JP.UTF-8` → `["ja-JP"]`、`LANGUAGE=ja:en` と `LANG=en_US.UTF-8` → `["en-US"]` |
| WebView2 (Windows) | 環境の言語 (`CoreWebView2EnvironmentOptions.Language`) に従う。OS の表示言語を変えても `["en-US", "en"]` のままという報告がある | 未確認 (手元に Windows が無い)。[tauri#2735](https://github.com/tauri-apps/tauri/issues/2735)、[CoreWebView2EnvironmentOptions](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2environmentoptions) |
| WKWebView (macOS) | OS の言語に従うという報告と、アプリのローカライズ (Info.plist の `CFBundleLocalizations`) との重なりに制限されるという報告がある | 未確認 (手元に Mac が無い)。[Apple Developer Forums](https://developer.apple.com/forums/thread/750119) |

どの WebView でも、OS の優先する言語の **一覧** は取れない (取れても 1 つ)。正確にするには、アプリの Rust 側で OS の言語を取り
(`sys-locale` クレートなど)、`setSystemLanguages()` に渡す。エディタ・ビューア・スタジオは tauri-plugin-os の `locale()` で取り、
`initLocale()` の 2 つ目の引数 (タグか一覧を返す非同期の関数。取れなければ `null`) として渡している。
このパッケージは Tauri に依存しないので、取り方はアプリが決める (Tauri の外では `null` を返すか、引数を省く)。
取得が失敗・`null`・空の一覧なら `navigator.languages` のまま。`initLocale()` が待つ間に呼び直されたら、古い呼び出しの結果は捨てる。

## カタログの検査 (`just i18n`)

`packages/*/locales/` と `apps/*/locales/` を調べる (`just ci` に入っている)。

- 対応する言語のファイルだけがあり、原文 (`en.json`) がある。値はすべて文字列。キーに `.` を使わない (入れ子にする)。
- 訳に原文に無いキーが無い。`complete` の言語 (日本語) は原文のキーがすべてあり、空の文字列が無い。ほかの言語は欠けてよい (英語を出す)。
- 複数形はその言語の CLDR の区分 (`Intl.PluralRules`) がそろう (英語は `_one`・`_other`、日本語は `_other`)。`_zero` はどの言語でも足してよい。
- 引数 (`{{name}}`) の名前が原文と同じ (`count` は省いてよい)。
- 未使用のキー: パッケージの `src/` (テストを除く) に、キーがそのままの文字列で現れない。`` t(`source.${kind}`) `` のように
  キーを組み立てるときは、テンプレート文字列の頭 (`source.`) で始まるキーを使っているとみなす。なるべく組み立てず、
  キーを並べた表 (`{ camera: "source.camera", ... }`) を書く。

型の検査 (svelte-check) は、`t()` のキーが原文のカタログにあることを確かめる。カタログを JSON から読むので、
引数の名前は型では検査しない (言語の間の食い違いは `just i18n` が見る)。

## コマンド

```sh
pnpm --filter @ugoe/i18n test    # Vitest (検出・書式・切り替えと再評価・検査のスクリプト)
pnpm --filter @ugoe/i18n check   # svelte-check
just i18n                        # カタログの検査
```

`test` と `check` は `just web-check` (`just ci`) に入っている。

## 依存の版

i18next は `^26.4.2` (ロックファイルで固定)。ADR 0029 で版を完全に固定すると決めたのは Bits UI だけで、i18next は大きな版の中で
互換が保たれている。大きな版を上げるときは、変更の記録を読み、このパッケージのテストと `just i18n` を通す。
