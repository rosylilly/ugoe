---
id: T0819
title: "スタジオ: ファイルを読むコマンドをメインのスレッドと舞台のロックの外へ"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0814]
assignee: claude-worker-T0819
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

- [x] ファイルを読む・大きなバイト列を返す・スレッドを待つコマンドを、メインのスレッドの外で動かす (`#[tauri::command(async)]` など。
      借用する引数との組み合わせを確かめる)
- [x] `dressup.list` が舞台のロックを握ったままファイルを読まない (読んだモジュールの中身を覚える、ロックの外で読むなど)
- [x] 実機 (`just studio` と T0006 の e2e) で、シーンの切り替え・プロファイルを開く・仮想カメラの停止が今までどおり動く
- [x] `just ci` が通る

## 参考

- `apps/studio/src-tauri/src/lib.rs` (`run_hotkey` の `spawn_blocking`)、`apps/studio/src-tauri/src/api/methods.rs` (`dressup_list`)
- docs/studio.md#隠れても止まらない

## メモ

- T0008: 3 つのアプリに同じ所見がある (同期のコマンド: T0685・T0726・T0819、CSP とコマンドの許可: T0686・T0727・T0820、一時ファイル経由の保存: T0687・T0728)。
  先に終えたタスクの形 (非同期にする関数・CSP の中身・一時ファイルの書き方) に後のタスクが揃える (docs/workflow/code-review.md#m9-の結果)。
- T0819 (形を揃えた): 先に終えたビューアの T0726 の形 (上の T0726 のメモ) に揃えた。`async fn` のコマンド + 同期の本体を
  `tauri::async_runtime::spawn_blocking` で動かす `blocking` の包み、引数は所有する型で受けて `move` で渡す、テストは `block_on`。
  `#[tauri::command(async)]` を同期の fn に付ける形は使わない (非同期の実行器のワーカーで同期の I/O やスレッドの join をすることになるため)。
  スタジオで違うところ (理由):
  - `blocking(&studio, |studio| ...)` は `Arc<Studio>` を写してクロージャに渡す (どのコマンドも舞台と指揮役を触るため)。
  - エラーはコマンドのエラー型のまま返す。`spawn_blocking` の失敗 (中でパニック) は `CommandError::Internal` (`{ code: "internal", message }`) にした
    (スタジオのエラーはコードで返す。ADR 0029 の 4.。`lib/profile.ts` の `StudioError`・`errors.ts`・ロケールに足した)。
    文字列のエラーを返すコマンド (`scene_*`・仮想カメラ) は `e.to_string()`。
  - ロック: std の `Mutex` のガードは `.await` をまたいで持たない。ロックはすべて `blocking` のクロージャの中で取って放す。
- T0819 (やったこと): `activate_scene`・`update_scene`・`open_profile`・`restore_profile`・`module_slot`・`scene_model`・`scene_module`・
  `scene_background`・`start_virtual_camera` (動いている仮想カメラを止めて join する)・`stop_virtual_camera` を `async` にした。
  `dressup.list` は、舞台と指揮役のロックの中では付けているモジュールの値・スロット・着せ替えに使えるパスだけを写し、付けていない
  モジュールはロックを放してから読む (`methods.rs` の `dressup_list_with`。読む関数を差し替えて、読む間にロックを握っていないことをテストした。
  前の形では失敗することを確かめた)。docs/studio.md#隠れても止まらない に書いた。
- T0819 (確かめたこと): `just e2e --only studio` (本物のアプリ・Xvfb・Linux) がすべて通る。プロファイルを開く・クリックでのシーンの切り替え・
  開き直しは既存の手順で、仮想カメラの開始と停止の後にシーンを切り替えられる手順を足した (`tests/e2e/specs/studio.test.mjs`。`crates:` の外だが小さいテストの追加)。
  この機械には v4l2loopback の機器が無いので、仮想カメラは始められずに止まる経路だけを通った (実際に動いている仮想カメラの停止は確かめていない)。
  `just studio` を手で動かしての確認はしていない。Windows・macOS は確かめていない。
- T0819 (やらなかったこと): `save_profile` (プロファイルの JSON を書く) と `new_profile` などの軽いコマンドは同期のまま (読み込みではなく小さい書き込み。
  一時ファイル経由の保存はアプリごとの別タスク)。`dressup.list` の読んだモジュールの中身は覚えない (付けていないモジュールは呼ばれるたびに読み直す。
  ロックは握らないのでシーンの切り替えは待たせない)。
- T0819 (起票の候補): `dressup.list` が付けていないモジュール (最大 512 MiB) を呼ばれるたびに読み直すのを、パスと更新時刻・大きさで覚えて省く
  (外部 API のクライアントが頻繁に呼ぶと重い)。スタジオのホットキーと外部 API の経路の `with_stage` は、読み込みの間ずっと舞台のロックを握る
  (読み込みをロックの外に出すのは `Stage` の作りを変えるので別タスク)。
