---
id: T0006
title: 実機の Tauri のアプリを動かす自動テスト (Linux の CI で流す)
status: done
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

- [x] 本物の Tauri のアプリを起動して操作する仕組み (道具の選定は利用者に確認する。候補: tauri-driver + WebdriverIO、または WebKitWebDriver を直接)。
      利用者の設定とプロファイルを汚さない (XDG の向き先をテスト用にする)
- [x] 3 つのアプリの主な流れのテスト:
  - エディタ: 見本のプロジェクトを開く・パラメータを動かす・保存する・言語を切り替える
  - ビューア: 見本を開く・トラッキングの記録を流してパラメータが動く・設定を開く
  - スタジオ: 見本のプロファイルを開く・シーンを切り替える・偽のカメラ (重い検出を模したもの) でトラッキングが指揮役と出力に届く (T0812 の回帰)・出力ウィンドウを隠しても止まらない・API とブラウザソースにつなぐ
- [x] `just e2e` で流せる。CI の Linux のジョブに入れる (xvfb などで画面なしで。時間がかかるなら別のワークフローか毎晩)
- [x] 失敗したときにスクリーンショットとログを残す
- [x] Windows と macOS は手で確かめる一覧 (T0007) に回す。docs/testing.md に書く
- [x] `just ci` が通る

## 参考

- scratchpad の debug-tracking/e2e と T0812-e2e (調査と修正の確認に使った手順)、docs/testing.md

## メモ
- 道具 (利用者の決定 2026-10-03): tauri-driver 2.1.0 (`cargo install --locked`、CI も同じ版) + WebKitWebDriver + 自前の小さな W3C WebDriver の
  クライアント (tests/e2e/lib/webdriver.mjs、npm の依存なし) + node:test。CI は別のワークフロー `.github/workflows/e2e.yml`
  (main への push・毎晩・手動)。
- `just e2e` (tests/e2e/run.mjs): 各アプリを `tauri build --debug --no-bundle` (フロントエンドを埋め込む) でビルドし、Xvfb の上で、
  XDG_CONFIG/DATA/CACHE/STATE_HOME を target/e2e/<アプリ>/xdg に向け、dbus-run-session の中で tauri-driver を起動して筋書きを流す。
  WSLg では /tmp/.X11-unix に書けないので、Xvfb は `-nolisten unix` (抽象ソケット) で、ロックの無い 99 番以降の番号を使う
  (番号を Xvfb に選ばせると WSLg の :0 とぶつかる)。
- 筋書き: エディタ 5 手順・ビューア 4 手順・スタジオ 9 手順 (tests/e2e/README.md#筋書き)。手元 (WSL2) で、ビルド済みからの
  全体 (3 つのビルド + テスト) が約 80 秒 (テストだけで約 40 秒)。エディタ 6 回・ビューアとスタジオ 4 回続けて流して、すべて通った。
- 偽のカメラ: 記録の再生 (`startReplay`) に、刻みごとの処理を 200ms 塞ぐ重さを足す (`replayTime` のアクセサをインスタンスだけ差し替え)。
  T0812 の修正前の pulse.ts に戻してビルドすると、操作ウィンドウが応答しなくなり、44 秒で失敗する (WebDriver の要求のタイムアウトを
  「ページが応答しない」として報告し、後の手順は待たずに失敗にする) ことを確かめた。
- 見つけた不具合と修正 (apps/studio): Linux では、出力ウィンドウを出した直後に `is_visible` がまだ false を返し (tao がウィンドウの操作を
  イベントループへ送るため)、操作ウィンドウの「出力ウィンドウ」のボタンとスイッチが切り替わらなかった。`set_output_visible`・
  `set_output_on_top` は頼んだ値を返すようにした。
- 分かった WebKitWebDriver の癖 (README に反映): Get Element Text は入れ子の要素の文字が欠ける (ページの中の textContent で探す)、
  スクロールする入れ物の中の要素を自分でスクロールしない (クリックの前に scrollIntoView)、`__TAURI_INTERNALS__.invoke` は書き換えられない
  (invoke の計測は ipc:// への fetch を包む)、隠れたウィンドウのスクリーンショットは応答しない (撮らない)。
- 確かめていないこと: CI (GitHub Actions) での実行とかかる時間 (ワークフローは手元と同じ手順。初回はキャッシュが無いので 3 つのアプリの
  ビルドに時間がかかる見込みで、timeout は 90 分)。Windows (tauri-driver + Edge Driver は動くはずだが未確認)・macOS (WebDriver が無い)。
  出力ウィンドウのドラッグ (T0813。OS のウィンドウの移動は WebDriver で起こせない)・本物のカメラと MediaPipe・操作ウィンドウの最小化
  (Xvfb にウィンドウマネージャが無い)。
- 後続の候補: Windows のランナーで同じ筋書きを流す (tauri-driver の Edge Driver の中継)。T0813 のドラッグを xdotool と軽いウィンドウマネージャで
  確かめる。偽のカメラを canvas の captureStream + 本物の MediaPipe にする (顔の画像の権利の整理が要る)。
