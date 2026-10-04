---
id: T0685
title: "エディタ: 重い同期コマンドをメインのスレッドの外へ"
status: in-progress
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

- [ ] 上のコマンドをメインのスレッドの外で動かす (`#[tauri::command(async)]` か `ai::blocking` と同じ `spawn_blocking`)。
      借用する引数 (`State<'_>`・`Request<'_>`) との組み合わせを確かめる
- [ ] 外すと IPC の呼び出しが並んで動くようになる。フロントエンドが `await` せずに続けて呼んでいるところ (`apply_command` の直後の
      `get_model_json`・`get_model_bundle` など) で、順序の前提が崩れないことを確かめる (崩れるなら直すか、順序を保つ仕組みを入れる)
- [ ] 文書のロックを握ったまま重い計算をしない (`generate_texture_normal_map` は提案と同じく複製で作り、適用だけロックの中で)
- [ ] 実機 (`just editor` と T0006 の e2e) で、開く・保存・PSD の取り込み・書き出し・プレビューが今までどおり動く
- [ ] docs/editor.md に、どのコマンドがメインのスレッドの外で動くかを書く
- [ ] `just ci` が通る

## 参考

- `apps/editor/src-tauri/src/ai.rs` (`blocking`)、`proposals.rs` (`propose_normal_maps_in` の複製とロック)
- スタジオの同じ所見: T0819

## メモ

- T0008: 3 つのアプリに同じ所見がある (同期のコマンド: T0685・T0726・T0819、CSP とコマンドの許可: T0686・T0727・T0820、一時ファイル経由の保存: T0687・T0728)。
  先に終えたタスクの形 (非同期にする関数・CSP の中身・一時ファイルの書き方) に後のタスクが揃える (docs/workflow/code-review.md#m9-の結果)。
