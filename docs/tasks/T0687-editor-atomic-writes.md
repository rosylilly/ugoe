---
id: T0687
title: "エディタ: 書き出しと設定のファイルを一時ファイル経由で書く"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0669]
assignee: claude-worker-T0687
---

# T0687: エディタ: 書き出しと設定のファイルを一時ファイル経由で書く

## 目的

T0669 のレビューで見つけた。エディタのバックエンドは、書き出し (`export_bundle`・`export_module`・`write_png_file`・
`write_json_file`・MCP の `save_project` の `bundle`) と設定 (`ai-provider.json`・`mcp-bridge.json`) を `std::fs::write` や
`truncate(true)` で直接書く。書いている途中で失敗する (ディスクがいっぱい・強制終了) と、前に書き出した正しいファイルが
途中までのファイルに置き換わる。`mcp-bridge.json` ならトークンと有効の設定が消え、つないだクライアントの設定が使えなくなる。

## 受け入れ条件

- [x] 同じフォルダーの一時ファイルに書いてから名前を変える (Windows の置き換えを含む)。`mcp-bridge.json` は一時ファイルも
      利用者だけが読める権限で作る
- [x] 失敗したときに前のファイルが残るテスト
- [x] `.ugoeproj` の保存 (`ugoe_editor_core::Project::save`) が同じ形かを確かめ、違えば editor-core の側を起票する
- [x] `just ci` が通る

## 参考

- `apps/editor/src-tauri/src/project.rs` (`export_bundle`)・`module.rs` (`export_module`)・`mcp.rs` (`write_private`)・
  `ai_provider.rs` (`ProviderStore::set`)

## メモ

- T0008: 3 つのアプリに同じ所見がある (同期のコマンド: T0685・T0726・T0819、CSP とコマンドの許可: T0686・T0727・T0820、一時ファイル経由の保存: T0687・T0728)。
  先に終えたタスクの形 (非同期にする関数・CSP の中身・一時ファイルの書き方) に後のタスクが揃える (docs/workflow/code-review.md#m9-の結果)。
- T0687 (やったこと): `apps/editor/src-tauri/src/atomic_write.rs` を足した。形はビューア (T0728) と `ugoe-mcp` の `StagedFile`
  (T0552) に揃え、一時ファイル `.<名前>.<pid>.<連番>.tmp` を `create_new` で作る→書く→`sync_all`→閉じる→`rename`、失敗したら
  一時ファイルを消す。`StagedFile` (書いておいて後で `commit`、捨てると消す) と `write_atomic` を持つ。`Access::Private` は Unix で
  一時ファイルを作るときから 0600 にする (前のファイルが 0644 でも、置き換えた後は 0600)。Windows の権限は既定のまま (確かめていない)。
  - 置き換えた書き込み: `export_bundle`・`export_module`・`write_png_file`・`write_json_file`・`ProviderStore::set` (`ai-provider.json`)・
    `write_private` (`mcp-bridge.json`)・`EditorHost::save` の `bundle`。`EditorHost::save` は `ugoe-mcp` と同じ順 (書き出し→一時ファイル→
    プロジェクトの保存→置き換え) にし、エラーの文も揃えた。
  - テスト: `atomic_write::tests` (置き換え・途中で書けない・名前を変えられない・捨てる・0600)、
    `mcp::tests::saving_with_a_bundle_keeps_the_previous_bundle_when_it_fails` (MCP の保存の `bundle`)。
  - `docs/editor.md` の書き出しと MCP の節に書き方を足した。
  - `.ugoeproj` の保存 (`ugoe_editor_core::Project::save`、`project/files.rs` の `write_if_changed`) は確かめた: ファイルごとに一時ファイル
    へ書いて `sync_all` し `rename` する点は同じ。違いは (1) 一時ファイルの名前が `<名前>.ugoe-tmp` 固定 (pid・連番が無く、隠れない。
    2 つのプロセスが同時に同じプロジェクトを保存すると一時ファイルが重なる)、(2) `rename` の前に閉じていない (Rust の std は
    `FILE_SHARE_DELETE` で開くので Windows でもたぶん動くが確かめていない)、(3) プロジェクト全体ではなくファイルごとの置き換え
    (途中で失敗するとファイルの組み合わせが混ざる。`project.json` を最後に書くので新規の保存は完成の印で判る)。
    指示で新しいタスクは起票しないので、下の起票の候補に書いた。
- T0687 (やらなかったこと): `save_project` の書き込みをロックの外へ出すこと (T0685 から)。`save_to` は書いた後に指紋を取り直して
  `mark_saved` するので、ロックを放した間の編集を「保存した」と誤って印を付けないよう、世代 (`Document` の版) を比べる仕組みが要る。
  範囲は `apps/editor` だが設計の変更が要るので見送った。
- 起票の候補:
  - editor-core: `Project::save` の一時ファイルの名前を `.<名前>.<pid>.<連番>.tmp` にし、`rename` の前に閉じる (上の (1)(2))。
    共通の書き方を `ugoe-editor-core` (または小さな共有の場所) に置き、エディタ・ビューア・`ugoe-mcp` の 3 つの写しをまとめることも。
  - ugoe-mcp: `mcpb::Bundle::write` は `<名前>.tmp` に書いて `rename` するが `sync_all` せず、書く途中の失敗で一時ファイルを残す。
  - editor: `save_project` の書き込みをロックの外へ (上の「やらなかったこと」)。
