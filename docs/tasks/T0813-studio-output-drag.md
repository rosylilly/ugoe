---
id: T0813
title: スタジオ: 出力ウィンドウをふつうのドラッグで動かせるようにする
status: done
milestone: M8
crates: [apps/studio, docs]
depends_on: [T0801]
assignee: claude-team-lead
---

# T0813: 出力ウィンドウのドラッグ

## 目的

利用者の報告: 出力ウィンドウをドラッグできず、位置を変えられない。T0801 では、配信中のうっかりした移動を避けるため「Alt を押しながらドラッグ」で動かす作りにしたが、
(1) 気づきにくい、(2) Linux のウィンドウマネージャ (や WSLg) は Alt + ドラッグを自分で取ることがあり、アプリに届かない、
(3) `startDragging` をフィードの Promise の解決を待ってから呼んでいて、押した瞬間に間に合わないことがある。

## 受け入れ条件

- [x] 出力ウィンドウは、左ボタンのふつうのドラッグで動かせる (修飾キーは要らない)。`startDragging` は押した時点で同期的に呼ぶ
- [x] ブラウザソースのページ (同じ Output.svelte) では何もしない
- [x] 操作ウィンドウの知らせ・マニュアル・docs/studio.md・README・権限の説明を、新しい操作に合わせる
- [x] `just ci` が通る

## メモ
- 左ボタンのドラッグで動かす形にした (Alt は要らない)。押した瞬間に startDragging を呼べるよう、配信の口を onMount で受け取っておき、host の window モジュールも先に読み込む。ブラウザソースの口は startDragging を持たないので何もしない。
- 確かめていないこと: 実機の Tauri のウィンドウでのドラッグ (WebKitGTK・WebView2・WKWebView)。
