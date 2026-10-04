---
id: T0920
title: 共有 UI (@ugoe/ui) の props の名前と公開の範囲をそろえる
status: in-progress
milestone: M9
crates: [packages/ui, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0912]
assignee: claude-worker-T0920
---

# T0920: @ugoe/ui の props の名前と公開の範囲をそろえる

## 目的

T0912 のレビューで見つけた、`@ugoe/ui` の公開 API の不ぞろいを直す。どれも利用するアプリの書き換えを伴うので、T0912 では直さなかった。

1. **読み上げの props の名前:** `Button`・`IconButton` は `aria-describedby`・`aria-labelledby` (HTML の属性の名前) を受け、
   `Checkbox`・`Toggle`・`NumberField` は `describedby`、`NumberField`・`Select`・`ColorPicker` は `labelledby`、
   `Select`・`SegmentedControl`・`ColorPicker`・`Tabs` は `ariaLabel` を受ける。どちらかの書き方にそろえる
   (Button は Bits UI のトリガーの属性を広げて受けるので、`aria-*` のままにして、ほかを合わせるのが候補)。
2. **トリガーのスニペットの名前:** `Popover`・`Menu` は `trigger`、`Tooltip` は `children` でトリガーの属性を渡す。
3. **公開の範囲:** `index.ts` の `export *` で、アプリが使っていない部品の内部の関数も出ている
   (`number.ts` の `clamp`・`decimalsOf`・`roundTo`・`formatEditable`・`parseEditable`・`stepValue`、`files.ts`、
   `shortcut.ts` の `parseShortcut`、`theme.svelte.ts` の `loadThemePreference`・`saveThemePreference`・`THEME_PREFERENCES`、
   `toasts`・`dismissToast`)。使うものだけを名前で出す (2026-10-04 の時点でアプリが使うのは、部品のほか `initTheme`・`currentTheme`・
   `showToast`・`hexToRgb`・`rgbToHex`・`mergeAttributes` と型の `ButtonBaseProps`・`MenuEntry`・`Option`)。

## 受け入れ条件

- [ ] 1 と 2 の名前を 1 つの書き方にそろえ、3 つのアプリの呼び出しを書き換えた (古い名前は残さない。`@ugoe/ui` は private のパッケージ)
- [ ] `index.ts` が出すものを、アプリとほかのパッケージが使うものと部品に絞った
- [ ] packages/ui/README.md と docs/ui.md の例を新しい名前に直した
- [ ] `just ci` が通る

## 参考

- T0912 のメモ
- docs/ui.md#部品の一覧 (「Bits UI を包むときの約束」)

## メモ
