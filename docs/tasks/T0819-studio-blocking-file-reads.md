---
id: T0819
title: "スタジオ: ファイルを読むコマンドをメインのスレッドと舞台のロックの外へ"
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0814]
assignee:
---

# T0819: スタジオ: ファイルを読むコマンドをメインのスレッドと舞台のロックの外へ

## 目的

T0814 のレビューで見つけた。Tauri v2 では `async` でないコマンドはメインのスレッド (イベントループ) で動く。スタジオの
`activate_scene`・`update_scene`・`open_profile`・`restore_profile`・`module_slot` はモデル・モジュール (最大 512 MiB)・背景の
画像をファイルから読み、`scene_model`・`scene_module`・`scene_background` は今のシーンのバイト列を写して返し、
`stop_virtual_camera` は仮想カメラのスレッドの終わりを待つ。どれもメインのスレッドを塞ぐので、配信中にシーンを切り替えると
両方のウィンドウのイベントの処理 (と `Channel` で届くフレーム) が止まりうる。ホットキーの経路 (`lib.rs` の `run_hotkey`) は
既に `spawn_blocking` で外してあり、扱いが揃っていない。

また外部 API の `dressup.list` (`api/methods.rs`) は、着せ替えに使えるモジュールを呼ばれるたびにファイルから読み直し、
その間は舞台のロックを握る (シーンの切り替えを待たせる)。

## 受け入れ条件

- [ ] ファイルを読む・大きなバイト列を返す・スレッドを待つコマンドを、メインのスレッドの外で動かす (`#[tauri::command(async)]` など。
      借用する引数との組み合わせを確かめる)
- [ ] `dressup.list` が舞台のロックを握ったままファイルを読まない (読んだモジュールの中身を覚える、ロックの外で読むなど)
- [ ] 実機 (`just studio` と T0006 の e2e) で、シーンの切り替え・プロファイルを開く・仮想カメラの停止が今までどおり動く
- [ ] `just ci` が通る

## 参考

- `apps/studio/src-tauri/src/lib.rs` (`run_hotkey` の `spawn_blocking`)、`apps/studio/src-tauri/src/api/methods.rs` (`dressup_list`)
- docs/studio.md#隠れても止まらない

## メモ
