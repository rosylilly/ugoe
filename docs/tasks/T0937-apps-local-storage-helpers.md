---
id: T0937
title: アプリの localStorage の扱いを揃える (ビューアとスタジオの設定・テストの MemoryStorage・版の違う保存値。判断が要る)
status: done
milestone: M9
crates: [packages, apps/viewer, apps/studio, docs]
depends_on: []
assignee: claude-worker-T0937
---

# T0937: アプリの localStorage の扱いを揃える (ビューアとスタジオの設定・テストの MemoryStorage・版の違う保存値。判断が要る)

## 目的

- ビューアとスタジオの `src/lib/settings.ts` (言語・テーマの localStorage の読み書き) が別々の形。T0930 で player に作った入れ物と同じ形に揃え、
  `@ugoe/ui` か `@ugoe/i18n` に置く (T0930 の候補。T0914 の横断の最後の項目)。
- テストの `MemoryStorage` は player では `src/testing/memoryStorage.ts` にまとめたが、ビューア (`drop.test.ts`)・スタジオ (`settings.test.ts`) に写しがある (T0930 の候補)。
- `@ugoe/player` の `ModelMemory` は、版 (`version`) が今と違う保存値を空として扱い、次に覚えるときに上書きする (新しい版のビューアで覚えたものを
  古い版で開くと失う。T0011 の候補)。

**判断が要る:** 版の違う保存値の方針 (読めない新しい版の値を残して別の鍵に書く、など)。版を上げる前に決める。

## 受け入れ条件

- [x] ビューアとスタジオの設定の読み書きが共有の入れ物を使い、既存の保存値を読める (移行のテストがある)
- [x] `MemoryStorage` の写しを共有のテスト用の部品に寄せた
- [x] 版の違う保存値の方針を決め (利用者の判断を待つならメモに案を書いてこの項目だけ残す)、決めたなら player に入れてテストがある
- [x] `just ci` が通る

## 参考

- T0930・T0011 のメモ (起票の候補)、T0925

## メモ

- T0026 で起票した (元の候補: T0930 の 2 つ、T0011 の `ModelMemory` の版)。

### やったこと

- **共有の入れ物:** `@ugoe/i18n` に `src/storage.ts` (`readStoredItem`・`writeStoredItem`・`removeStoredItem`。try の中で引き、
  読めなければ `null`、書けなければ何もしない) を足し、言語 (`preference.ts`)・テーマ (`@ugoe/ui` の `theme.svelte.ts`)・
  `@ugoe/player` の `PlayerStorage`・スタジオの `forgetLegacySettings` の重複した try をこれに寄せた。
  - 置き場所は `@ugoe/i18n`: `@ugoe/ui` は `@ugoe/i18n` に依存し、`@ugoe/player` は `@ugoe/ui` に依存しないので、
    言語・テーマ・Player のすべてが使える一番下のパッケージはここだけ (README に理由を書いた)。
  - T0925 で、ビューアとスタジオの `settings.ts` はすでにキーの定数だけになっていた (読み書きは `initLocale`・`initTheme`)。
    今回はその中身を共有の関数にした。キーと値は変えていないので移す処理は要らない。前の版が書いた値 (`ugoe-viewer.locale`・
    `ugoe-studio.theme` など) を共有の読み書きで読めることをアプリのテストで確かめた (`@ugoe/ui` から `loadThemePreference` を出した)。
- **テストの `MemoryStorage`:** `@ugoe/i18n/testing` (package.json の `exports` に足した) に `MemoryStorage`・`BrokenStorage`・
  `useLocalStorage` を置き、player (`testing/memoryStorage.ts` を消した・`storage.test.ts`)・i18n・ui・ビューア (`drop.test.ts`)・
  スタジオ (`settings.test.ts`) の写しを消した。
- **版の違う保存値の方針 (決めた):** `ModelMemory` は、今より新しい版 (整数で `STORAGE_VERSION` より大きい) の値を読まず、
  書き換えも消しもしない。その間の覚える・忘れる・すべて忘れるは入れ物の中 (そのセッション) だけで効く。壊れた値・版の無い値は前と同じく
  次に覚えるときに書き直す。版を上げるときは前の版の値を今の形に直して読む (コメントと README に書いた)。先に失敗するテストを足した。
  - 根拠: ビューアの `readable-files.json` (T0011) は「読めない (壊れてはいない) 値はそのセッションでは書かない、壊れた JSON は次の操作で
    書き直す」で、エディタの設定 (`history.rs`) も「新しい版が書いた設定は起動しただけでは書き換えない」。エディタ・スタジオの設定は
    1 つの値なので利用者が変えたら上書きするが、`ModelMemory` は 1 つのキーに多くのモデルの値が入り、1 つ覚えるだけで新しい版の値を
    すべて失うので、`readable-files.json` と同じく書かない側に寄せた。別の鍵に書く案 (`name.v1` など) は、古い版の鍵が残り続け、
    新しい版が古い版の鍵を読み直す取り決めも要るので採らなかった。
  - 1 つの値の `lighting` (`{version, settings}`) は、エディタ・スタジオの設定と同じく利用者が変えたときに今の版で書く (変えていない)。

### やらなかったこと

- エディタの localStorage の小さな読み書き (`TimelinePanel.svelte`・`layout.ts`・`externalChange.ts`) は `crates:` の外なので
  共有の関数に寄せていない (どれも try の中で読み書きしていて、例外は出さない。形が重複しているだけ)。

### 起票の候補

- エディタの localStorage の読み書き (`TimelinePanel.svelte`・`layout.ts`・`externalChange.ts`) を `@ugoe/i18n` の
  `readStoredItem` などに寄せる (try の写しを減らすだけの小さな整理)。

`just ci`: 通った (main に追従済み)。
