---
id: T0903
title: UI: 共有部品の 2 段目 (ダイアログ・知らせ・ツールチップ・メニュー・色の選択・ファイルのドロップ)
status: done
milestone: M8
crates: [packages, docs]
depends_on: [T0902]
assignee: claude-worker-T0903
---

# T0903: UI: 共有部品の 2 段目 (ダイアログ・知らせ・ツールチップ・メニュー・色の選択・ファイルのドロップ)

## 目的

T0902 の続き。アプリごとに作りが違う、重ねて出す部品を揃える。

## 受け入れ条件

- [x] bits-ui を土台にする (ADR 0029)。Dialog (モーダル。フォーカスを閉じ込める・Esc で閉じる)、知らせの帯 (Notice / Toast)、Tooltip (IconButton にも付ける)、Menu / Dropdown、Popover、ColorPicker (Popover の中)、ファイルのドロップ領域、ショートカットの表記
- [x] 見本の画面に足し、スクリーンショットの検査に入れる
- [x] Vitest のテストと README。`just ci` が通る

## 参考

- apps/editor の BaseChangeDialog.svelte など (いまのダイアログ)、apps/viewer の MemoryDialog.svelte・LicenseDialog.svelte

## メモ

- **やったこと:**
  - Bits UI を土台: `Dialog` (Bits の Dialog。中でフォーカスを回す・Esc と外のクリックで閉じる・開いた元へ戻す。閉じるボタンは並びの最後に置き、
    最初の入力欄にフォーカスが行く。`dismissible={false}` で中のボタンでしか閉じない)・`AlertDialog` (フォーカスは取り消し側、決定は既定で danger)・
    `Tooltip` と `TooltipProvider` (Provider が無ければ Tooltip が自分で置く)・`IconButton` に Tooltip (`label` を出す。`tooltip={false}` で消せる。
    名前と同じなので aria-describedby は外す。`shortcut` で Kbd と aria-keyshortcuts)・`Menu` (DropdownMenu。項目・チェック・区切り・入れ子。
    トリガーは文字のボタン / IconButton / 自前)・`ContextMenu`・`Popover`。
  - 自作: `Notice` (status / danger は alert。種類を印と読み上げの言葉でも示す)・`Toast` (`showToast()` と根元の `Toaster`。popover="manual" の
    role="status"。ポインタやフォーカスが乗っている間は時間を止める)・`ColorPicker` (Popover の中に <input type="color">・16 進の欄・R/G/B の NumberField)・
    `DropZone` (「ファイルを選ぶ…」のボタンを必ず出す。accept で選り分け。Tauri 向けに `active` と `onbrowse`)・`Kbd` (Mod → ⌘ / Ctrl)。
  - Bits UI 2.19.4 が付けないので足したもの: Tooltip の `role="tooltip"`、Menu・ContextMenu の一覧の `aria-label` (docs/ui.md に例として追記)。
  - 部品の既定の文言は名前空間 `ui` (common.close・common.cancel・notice.tone.*・toast.region.aria・colorPicker.*・dropZone.*)。英語と日本語。
  - Vitest は 29 → 70 件 (happy-dom、@testing-library なし)。新しい依存は無い。
  - 見本の画面に Dialogs・Menus・Notices・Color and files・Shortcuts を足し、`?open=dialog|alert|menu|popover|toast|tooltip` で開いた状態にできるようにした。
    `just ui-shots` は 21 枚 (重ねて出す部品 6 種 × 2 通りを足した)。開いたものが DOM にあるか・menu / dialog / tooltip / menuitem の読み上げの名前も検査する。
    動きの途中を撮らないよう Chrome に `--force-prefers-reduced-motion` を渡す。撮った画像は scratchpad/T0903/shots/ (すべて ok)。
- **確かめていないこと:** 本物の WebView (WebView2・WKWebView・WebKitGTK) とスクリーンリーダーでの読み上げ (Toast の領域が popover の中でも読まれるか含む)。
  サブメニューのキーボード操作・ContextMenu の Shift+F10 は Vitest で検査していない (見本の画面で手でも触っていない)。ColorPicker の OS の色の選択を動かす操作。
  Toast と後から開いた <dialog> (いまのアプリのダイアログ) の重なりは実物で見ていない。
- **やらなかったこと:** `Menubar` (使う画面ができたときに足す。docs/ui.md に書いた)。既存のアプリの移行 (T0907 など)。
- **後続の候補:** アプリのダイアログ (BaseChangeDialog・MemoryDialog など) を Dialog / AlertDialog へ移す。エディタのメニューバーを Menubar で作る。
  Tauri のファイルのドロップのイベントを DropZone の `active` につなぐ口 (アプリ側)。T0908 の直書き検査で `src/shortcut.ts` の修飾キーの名前 (Ctrl・Shift など) をどう扱うか。
