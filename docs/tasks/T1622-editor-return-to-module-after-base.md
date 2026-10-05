---
id: T1622
title: "エディタ: 「Open base project」で本体を直した後、モジュールに戻る道を作る"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1622
---

# T1622: エディタ: 「Open base project」で本体を直した後、モジュールに戻る道を作る

## 目的

T1609 で、モジュールの適用のエラーから「Open base project」で本体を開けるようにした。本体を直し終えた後、元のモジュールに戻る道
(最近開いたモジュールを開き直すボタンなど) が無い。本体の問題の枠の同じボタンも同じ (T1609 の候補)。

## 受け入れ条件

- [x] 本体を「Open base project」で開いたとき、元のモジュールを開き直す手段 (ボタンなど) が見える所にあり、押すとモジュールを開き直して適用をやり直す
- [x] 文言は `@ugoe/i18n` のカタログ (日英) にあり、部品のテストがある
- [x] docs/editor.md (とマニュアルの更新の候補を T0919 のメモ) に書いた
- [x] `just ci` が通る

## 参考

- T1609 のメモ (起票の候補)
- docs/editor.md

## メモ

- T0032 で起票した (元の候補: T1609 (モジュールに戻る道))。
- やったこと:
  - 画面: 「Open base project」(本体の問題の枠・適用のエラーの案内) で本体を開くと、ヘッダーの下の知らせの帯に
    `ModuleReturnBar.svelte` (「Editing the base of module …」と「Return to module」) を出す。`FileMenuFlow.returnTo` が戻る先
    (モジュールのプロジェクトの場所・名前・拒否された設定) を持ち、`returnToModule` が本体の未保存を確かめてから `open_project` で
    モジュールを開き直す (保存した本体の上に組み直す)。開き直しが本体の問題で失敗すれば「Open…」と同じ問題の枠を出し、帯は残す。
    ほかのプロジェクトに替わる・帯を閉じると消す。
  - 適用のやり直し: モジュールの設定の画面で拒否された `set_module_settings` から本体を開いたときは、戻った後に同じ設定を適用し直す。
    また拒否されたら設定の画面をその設定とエラーを欄に入れて開く (`SettingsRetry`・`ModuleSettingsDialog` の `retry`)。インスペクタの上の
    エラー (ほかの編集) からは開き直すだけ (本体の要素の編集は送り直しても同じ理由で拒否されるため。docs/editor.md#モジュールに戻る)。
  - 読めるパス (T1616): `open_project` は、閉じるのが保存したモジュールのプロジェクトなら、そのフォルダーを読めるパスに加える
    (`project::open_at`。テスト `opening_the_base_keeps_the_module_folder_readable`)。本体の問題の枠からのときは開くダイアログで選んだので読める。
  - 文言: `moduleReturn.*`・`fileMenu.notice.openedBaseReturn`・`returnedToModule`・`returnedAndApplied` (日英)。
  - テスト: `fileMenu.test.ts` (戻る・適用し直す・また拒否される・未保存の確かめ・問題の枠・消すとき・未保存のモジュール)、
    `ModuleReturnBar.test.ts` (帯の部品と、設定の画面の `retry` の欄)。`just ci` は通った。
  - docs/editor.md に「モジュールに戻る」を足し、「読み書きできるパス」を直した。マニュアルの更新の候補は T0919 のメモに書いた。
- やらなかったこと: 画面を実際に起動しての目視は行っていない (vitest・svelte-check まで)。docs/manual は直していない (T0919 の候補)。
  未保存のモジュール (場所が無い) からは戻る道を出さない (戻る先が無い)。
- 起票の候補: なし。
