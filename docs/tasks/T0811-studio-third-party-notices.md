---
id: T0811
title: スタジオ: 第三者のライセンスの表記 (MediaPipe などの同梱物) を出す画面
status: done
milestone: M8
crates: [apps/studio, docs]
depends_on: [T0803]
assignee: claude-worker-T0811
---

# T0811: スタジオのライセンスの表記

## 目的

スタジオは T0803 で MediaPipe (Apache-2.0) のモデルと wasm を同梱するようになったが、ビューアやエディタのようなライセンスの表記の画面が無い (T0803 のメモ)。

## 受け入れ条件

- [x] ビューアの LicenseDialog と同じ形で、スタジオ自身 (GPL-3.0-only) と同梱する第三者 (MediaPipe・bits-ui・i18next など) のライセンスを出す画面を操作ウィンドウに置く
- [x] third-party-notices に同梱する JS と MediaPipe の資産を載せる (ビューアの T0723 の作り方にそろえる)
- [x] 文言は @ugoe/i18n、部品は @ugoe/ui。`just ci` が通る

## 参考

- apps/viewer の LicenseDialog と third-party-notices.txt、packages/face-tracking/third-party-notices.txt

## メモ

- **やったこと:**
  - `apps/studio/src/LicenseDialog.svelte` (ビューアのものと同じ形。@ugoe/ui の Dialog で、開いたときに一度だけ `third-party-notices.txt` を読む)。
    操作ウィンドウのヘッダーの末尾に「ライセンス」のボタン (ghost・small) を置いた。Control.svelte の変更は import・状態 1 つ・ボタン・ダイアログの 4 か所と、下の折り返しだけ。
  - `apps/studio/public/third-party-notices.txt`: ビューアのものの名前を「ugoe studio」に替えたもの。載せているのは MediaPipe の wasm とモデル (顔・上半身)、
    JS のライブラリ (svelte・bits-ui と依存の runed・svelte-toolbelt・Floating UI・tabbable・clsx・style-to-object・inline-style-parser・i18next・@tauri-apps/api・plugin-dialog・plugin-os)。
    版は lockfile と同じ (ビューアと同じ版)。`vite build` の出力に @internationalized/date が入っていないことを確かめた (ビューアと同じく載せない)。
  - 文言は名前空間 `studio` の `toolbar.license` と `license.*` (英語・日本語)。apps/studio/README.md と docs/studio.md の「操作ウィンドウ」に 1 行ずつ足した。
  - ヘッダーに `flex-wrap: wrap` を足した。既定の 1280 幅でも負荷の表示が出るとヘッダーは幅いっぱいで、ボタンは 2 行目に折り返す (狭い幅では前からはみ出していた)。
    置き場所は team-lead と決めた (A: 折り返す)。
  - 確認: ヘッドレスの Chrome と偽の StudioHost (T0807 のものに入力・モーション・ホットキーの既定値を足した) で、日英とダーク・狭い幅を撮った
    (scratchpad の `T0811/shots/`。`toolbar-*.png`・`license-*.png`。スクリプトは `T0811/shots.mjs`)。Escape で閉じることも確かめた。
- **やらなかったこと:** Rust のクレート (tauri・tungstenite など) のライセンスの表記は、ビューアとエディタと同じく載せていない。
- **確かめていないこと:** Tauri の実物 (WebKitGTK・WebView2・WKWebView) での表示。ビルドした配布物に `third-party-notices.txt` が入ること (vite build の dist には入ることを確かめた)。
- **後続の候補:** 3 つのアプリで、同梱する Rust のクレートのライセンスの表記を作る (cargo-about などで生成する)。
  JS の一覧を lockfile から作って版のずれを検査する (いまはビューアとスタジオで手で同じ版にそろえている)。ヘッダーの組み直し (言語・テーマ・ライセンスを、ビューアやエディタと同じく「設定」のダイアログにまとめる案など) は、3 つのアプリの見直し (T0907) で扱う。いまは 1280 幅で負荷の表示が出ると「ライセンス」が 2 行目に折り返す (team-lead の判断で A)。Rust のクレートの表記は T0005 (Release) で扱う。
