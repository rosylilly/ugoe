---
id: T0006
title: 実機の Tauri のアプリを動かす自動テスト (Linux の CI で流す)
status: in-progress
milestone: M8
crates: [tests, scripts, justfile, .github/workflows, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0812, T0813]
assignee: claude-worker-T0006
---

# T0006: 実機の自動テスト

## 目的

M8 のアプリは主にヘッドレスの Chrome とテストで確かめたため、利用者が実機で触ってすぐに重い不具合が 2 つ見つかった (T0812 のトラッキングの固まり、T0813 の出力ウィンドウのドラッグ)。
本物の Tauri のアプリ (WebKitGTK) を起動して操作する自動テストを作り、同じ種類の不具合を CI で捕まえる。T0812 の調査で使った手順 (scratchpad の debug-tracking/e2e) を
使い捨てでなく、リポジトリの道具にする。

## 受け入れ条件

- [ ] 本物の Tauri のアプリを起動して操作する仕組み (道具の選定は利用者に確認する。候補: tauri-driver + WebdriverIO、または WebKitWebDriver を直接)。
      利用者の設定とプロファイルを汚さない (XDG の向き先をテスト用にする)
- [ ] 3 つのアプリの主な流れのテスト:
  - エディタ: 見本のプロジェクトを開く・パラメータを動かす・保存する・言語を切り替える
  - ビューア: 見本を開く・トラッキングの記録を流してパラメータが動く・設定を開く
  - スタジオ: 見本のプロファイルを開く・シーンを切り替える・偽のカメラ (重い検出を模したもの) でトラッキングが指揮役と出力に届く (T0812 の回帰)・出力ウィンドウを隠しても止まらない・API とブラウザソースにつなぐ
- [ ] `just e2e` で流せる。CI の Linux のジョブに入れる (xvfb などで画面なしで。時間がかかるなら別のワークフローか毎晩)
- [ ] 失敗したときにスクリーンショットとログを残す
- [ ] Windows と macOS は手で確かめる一覧 (T0007) に回す。docs/testing.md に書く
- [ ] `just ci` が通る

## 参考

- scratchpad の debug-tracking/e2e と T0812-e2e (調査と修正の確認に使った手順)、docs/testing.md

## メモ
