---
id: T0726
title: ビューアのファイルの読み書きのコマンドを非同期にする
status: todo
milestone: M9
crates: [apps/viewer]
depends_on: [T0724]
assignee:
---

# T0726: ビューアのファイルの読み書きのコマンドを非同期にする

## 目的

ビューアのバックエンドのコマンド (`read_model_file`・`write_json_file`・`write_png_file`。`apps/viewer/src-tauri/src/lib.rs`) は
同期の `#[tauri::command]` なので、Tauri のメインスレッドで動く。`read_model_file` は上限 512 MiB までのファイルを読むので、
大きなバンドルを開く・ネットワークのドライブから読む間、ウィンドウ (イベントのループ) が止まる。
コードレビュー (T0724) で見つけた。エディタ (T0685・T0686) とスタジオ (T0819・T0820) の同じ形の所見と揃えて直す。

## 受け入れ条件

- [ ] `read_model_file`・`write_json_file`・`write_png_file` が、メインスレッドでファイルを読み書きしない
  (`async` のコマンドにして `tauri::async_runtime::spawn_blocking` で読み書きするなど。エディタ・スタジオと同じ形に揃える)
- [ ] 拡張子・大きさ・PNG の検査はいまと同じに働き、テストがある
- [ ] `apps/viewer/README.md` の「仕組み」を直す
- [ ] `just ci` が通る

## 参考

- T0724 のメモの所見の表
- エディタの T0685・T0686、スタジオの T0819・T0820 (同じ所見)
- https://v2.tauri.app/develop/calling-rust/#async-commands

## メモ

- T0008: 3 つのアプリに同じ所見がある (同期のコマンド: T0685・T0726・T0819、CSP とコマンドの許可: T0686・T0727・T0820、一時ファイル経由の保存: T0687・T0728)。
  先に終えたタスクの形 (非同期にする関数・CSP の中身・一時ファイルの書き方) に後のタスクが揃える (docs/workflow/code-review.md#m9-の結果)。
