---
id: T0685
title: "エディタ: 重い同期コマンドをメインのスレッドの外へ"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0669]
assignee: claude-worker-T0685
---

# T0685: エディタ: 重い同期コマンドをメインのスレッドの外へ

## 目的

T0669 のレビューで見つけた。Tauri v2 では `async` でないコマンドはメインのスレッド (イベントループ) で動く。エディタの
バックエンドは AI・`.mcpb` のコマンドだけを `spawn_blocking` で外し (`ai::blocking`)、ほかの重いコマンドは同期のまま。

- ファイルを読み書きする: `open_project`・`reload_project`・`save_project`・`save_project_as`・`new_project_from_psd`・`import_psd`
  (PSD のデコードと自動メッシュ)・`open_sample_project`・`open_module_with_base`・`module_open_problem`・`reload_base`・
  `read_module_base`・`export_bundle`・`export_module`・`assign_texture_normal_map`・`read_module_file`・`write_png_file`
- 大きなバイト列を作る: `get_model_bundle` (編集のたびに呼ぶ。PNG を含む `.ugoe` を作る)・`get_preview_base_bundle`・
  `get_preview_module`・`get_proposal_preview_bundle`・`preview_proposal`
- 計算が重い: `generate_texture_normal_map` (`whole_group` でまとまりのすべてのテクスチャ。提案の `propose_normal_maps` は
  外してあるのに、こちらは文書のロックを握ったままメインのスレッドで作る)
- 一定の間隔で呼ぶ: `check_external_change`・`check_base_change` (保存先のファイルの指紋を取り直す)

大きなプロジェクトではこの間ウィンドウのイベントの処理が止まる。また、つないだエージェント (`crate::mcp` の `EditorHost`) は
HTTP のスレッドで文書のロックを握ってツールを動かすので、その間に同期のコマンドを呼ぶとメインのスレッドがロックを待って止まる。

## 受け入れ条件

- [x] 上のコマンドをメインのスレッドの外で動かす (`#[tauri::command(async)]` か `ai::blocking` と同じ `spawn_blocking`)。
      借用する引数 (`State<'_>`・`Request<'_>`) との組み合わせを確かめる
- [x] 外すと IPC の呼び出しが並んで動くようになる。フロントエンドが `await` せずに続けて呼んでいるところ (`apply_command` の直後の
      `get_model_json`・`get_model_bundle` など) で、順序の前提が崩れないことを確かめる (崩れるなら直すか、順序を保つ仕組みを入れる)
- [x] 文書のロックを握ったまま重い計算をしない (`generate_texture_normal_map` は提案と同じく複製で作り、適用だけロックの中で)
- [ ] 実機 (`just editor` と T0006 の e2e) で、開く・保存・PSD の取り込み・書き出し・プレビューが今までどおり動く
- [x] docs/editor.md に、どのコマンドがメインのスレッドの外で動くかを書く
- [x] `just ci` が通る

## 参考

- `apps/editor/src-tauri/src/ai.rs` (`blocking`)、`proposals.rs` (`propose_normal_maps_in` の複製とロック)
- スタジオの同じ所見: T0819

## メモ

- T0008: 3 つのアプリに同じ所見がある (同期のコマンド: T0685・T0726・T0819、CSP とコマンドの許可: T0686・T0727・T0820、一時ファイル経由の保存: T0687・T0728)。
  先に終えたタスクの形 (非同期にする関数・CSP の中身・一時ファイルの書き方) に後のタスクが揃える (docs/workflow/code-review.md#m9-の結果)。
- T0685 (やったこと):
  - 上の 26 コマンドを `async fn` にし、本体を `commands::blocking` (`AppHandle` からアプリの状態を取る。`ai::blocking` を移した) か
    `commands::off_main` (状態を使わない) で `spawn_blocking` へ渡した。エラーは今までどおり `Message`、スレッドの失敗は
    `editor.task_failed`。形はビューア (T0726) の `blocking` と同じ (async fn + spawn_blocking。借用の `Request<'_>` はバイト列を写して渡す)。
    `State<'_>` は使わず `AppHandle` を受け取る (`'static` の閉包に渡すため)。`lock`・`lock_both` は `&AppState` を取るようにした。
  - ロック: `get_model_bundle`・`get_preview_module`・`get_proposal_preview_bundle`・`export_bundle`・`export_module` はロックの中では
    `Project` の写しを取るだけで、バンドルはロックの外で作る。`generate_texture_normal_map` と `import_psd` は写しから編集を作り、
    `proposals::Base` (セッションと状態の番号) で変わっていないことを確かめてから適用する。変わっていれば新しいコード
    `editor.model_changed` (en/ja のカタログ・docs/editor.md の表・`messageCodes.ts` を更新)。
  - 順序: メインのスレッドの同期コマンドは届いた順に終わるので、`await` した `apply_command` の後の `get_model_json` (同期)・
    `get_model_bundle` (非同期) は適用の後の姿を読む。重なった `refresh` は既存の世代の番号で最後のものだけが置く。
    フロントエンドで `await` せずに続けて呼ぶところを洗い、崩れるのは提案のプレビュー (`showPreview` を続けて呼ぶと先に始めたものが
    後で終わりうる。`endPreview` の後に作りかけが出る) だけだったので、世代の番号を足した (Vitest 2 件。直す前は失敗)。
  - テスト: `heavy_commands_run_off_the_calling_thread`・`texture_normal_maps_are_made_from_a_copy` (モデルが変われば
    `editor.model_changed`)・`psd_imports_are_made_from_a_copy`。
  - 実機: `just e2e --only editor` (T0006) が通った (起動・見本のプロジェクトを開く・パラメータ・編集して保存・言語)。
- T0685 (確かめていないこと): `just editor` を手で操作しての PSD の取り込み・書き出し・プレビューの着せ替え・提案のプレビューは
  確かめていない (この環境では画面を操作できず、e2e にもその流れがない)。そのため受け入れ条件の「実機」は付けていない。
- T0685 (振る舞いの変化): 法線マップを作る・PSD を取り込む間に、利用者やエージェントが別の編集をすると、作った編集は適用されずに
  `editor.model_changed` になる (前はその編集が終わるまで待たされていた)。保存・開く・`preview_proposal` はロックを持ったまま
  別のスレッドで動くので、その間に呼んだメインのスレッドの同期コマンドは待つ (前と同じ長さ。画面のほかのイベントは止まらない)。
- 起票の候補:
  - e2e に PSD の取り込み・`.ugoe` の書き出し・プレビューの着せ替えの流れを足す (今回の変更の実機の確かめ)。
  - `save_project` の書き込みもロックの外へ (写しを書いてから保存の印を付ける。T0687 の一時ファイル経由の保存と一緒に)。

