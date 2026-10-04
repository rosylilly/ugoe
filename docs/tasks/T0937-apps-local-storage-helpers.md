---
id: T0937
title: アプリの localStorage の扱いを揃える (ビューアとスタジオの設定・テストの MemoryStorage・版の違う保存値。判断が要る)
status: todo
milestone: M9
crates: [packages, apps/viewer, apps/studio, docs]
depends_on: []
assignee:
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

- [ ] ビューアとスタジオの設定の読み書きが共有の入れ物を使い、既存の保存値を読める (移行のテストがある)
- [ ] `MemoryStorage` の写しを共有のテスト用の部品に寄せた
- [ ] 版の違う保存値の方針を決め (利用者の判断を待つならメモに案を書いてこの項目だけ残す)、決めたなら player に入れてテストがある
- [ ] `just ci` が通る

## 参考

- T0930・T0011 のメモ (起票の候補)、T0925

## メモ

- T0026 で起票した (元の候補: T0930 の 2 つ、T0011 の `ModelMemory` の版)。
