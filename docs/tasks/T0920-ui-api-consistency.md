---
id: T0920
title: 共有 UI (@ugoe/ui) の props の名前と公開の範囲をそろえる
status: done
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

- [x] 1 と 2 の名前を 1 つの書き方にそろえ、3 つのアプリの呼び出しを書き換えた (古い名前は残さない。`@ugoe/ui` は private のパッケージ)
- [x] `index.ts` が出すものを、アプリとほかのパッケージが使うものと部品に絞った
- [x] packages/ui/README.md と docs/ui.md の例を新しい名前に直した
- [x] `just ci` が通る

## 参考

- T0912 のメモ
- docs/ui.md#部品の一覧 (「Bits UI を包むときの約束」)

## メモ

### やったこと

- **読み上げの props:** HTML の属性の名前 (`aria-label`・`aria-labelledby`・`aria-describedby`) にそろえた (Button・IconButton に合わせる案)。
  `Checkbox`・`Toggle` (`aria-describedby`)、`NumberField` (`aria-labelledby`・`aria-describedby`)、`Select`・`ColorPicker` (`aria-labelledby`・`aria-label`)、
  `SegmentedControl`・`Tabs` (`aria-label`)。中で使う変数の名前は変えていない (`"aria-label": ariaLabel` で受ける)。
- **トリガーのスニペット:** `Tooltip` を `children` から `trigger` に (Popover・Menu と同じ)。部品の中の呼び出し (IconButton・Toggle) も直した。
- **公開の範囲:** `index.ts` の `export *` をやめ、部品と `numberFieldCommit`・`mergeAttributes`・`initTheme`・`currentTheme`・`showToast` と型
  (`ButtonBaseProps`・`ButtonVariant`・`MenuEntry`・`Option`・`Side`・`Tone`・`TriggerAttributes`・`ToastOptions`・`Theme`) だけを名前で出す。
  型は部品の props と出す関数の引数・戻り値に現れるものを残した。`hexToRgb`・`rgbToHex` はアプリが使っているように見えたが、エディタの
  `lighting.ts` の同名の関数で、`@ugoe/ui` のものは使われていなかったので出さない。
- エディタの `agents/bridge.test.ts` だけが `toasts`・`dismissToast` を使っていた (知らせの列を読んで検査)。`showToast` を `vi.mock` して
  呼び出しを検査する形に書き換えた (検査する中身は同じ)。
- 3 つのアプリ・`@ugoe/face-tracking`・見本の画面 (gallery) の呼び出しを、`@ugoe/ui` から import した部品のタグだけを対象に書き換えた
  (エディタの `TabStrip`・`LightColor` はアプリの部品で、`ariaLabel` のまま)。古い名前は残していない (svelte-check が知らない props を誤りにする)。
- テスト: 新しい名前で受けることの検査を `components.test.ts` に足し、既存のテストの props の名前を直した。
- 文書: docs/ui.md の「Bits UI を包むときの約束」に名前と公開の範囲の約束を足し、packages/ui/README.md に例 (aria の props・`trigger`) を足して、
  出さなくなった関数 (`dismissToast`・`setThemePreference`・`formatShortcut`・`ariaKeyShortcuts`) の記述を直した。
- 見た目と振る舞いは変えていない (props の名前と export だけ)。

### 確かめたこと

- `pnpm --filter <pkg> check`・`test` を editor・viewer・studio・face-tracking・player・ui で流して通った。`just ci` が通った。
- 実機 (Tauri の画面) と `just ui-shots` は流していない (Chrome が要る。出力の DOM の属性は変わらないので見た目への影響は無い想定)。

### 起票の候補

- エディタの `TabStrip`・`LightColor` などアプリの部品も `ariaLabel` を受けている。アプリの中の部品の props の名前を `@ugoe/ui` に合わせるか (任意)。

