---
id: T1616
title: "エディタ: ファイルのダイアログをバックエンドで開き、コマンドが読み書きできるパスを利用者が選んだものに絞る"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1616
---

# T1616: エディタ: ファイルのダイアログをバックエンドで開き、コマンドが読み書きできるパスを利用者が選んだものに絞る

## 目的

ビューアは T0727 でファイルのダイアログをバックエンドで開き、コマンドが読み書きできるパスを利用者が選んだものに絞った (`FileAccess`)。
エディタの `write_json_file` (`src-tauri/src/tracking.rs`)・`write_png_file` (`src-tauri/src/screenshot.rs`) などは、webview から渡されたパスに拡張子だけを確かめて書く (T0024 の候補)。

## 受け入れ条件

- [x] ファイルを読み書きするエディタのコマンドを一覧にし、ダイアログで選んだパス (と開いているプロジェクトの中) だけを受け付ける。一覧と例外をメモに書いた
- [x] 選んでいないパスを拒むテストがある
- [x] `apps/editor/src/lib/ipc.ts` と docs/editor.md を更新した
- [x] `just ci` が通る (実際の OS のダイアログで確かめていないことはメモに)

## 参考

- T0024 のメモ (起票の候補)、apps/viewer/src-tauri/src/acl.rs・lib.rs (`FileAccess`)

## メモ

- T0026 で起票した (元の候補: T0024 のエディタ)。スタジオの側は T0838。
- T0032 (3 回目の起票のまとめ) から: T0025 の候補「T0730 の `write_png_file` (パーセント符号の復号・PNG の検査) をビューアとエディタで共有する」は、
  このタスクでエディタの `write_png_file` をビューアと同じく選んだパスだけを受ける形にするときに、あわせて検討する
  (共有の場所: `ugoe-fs` はファイルの読み書きの補助なので、パスの復号と PNG の検査を入れるかは判断。入れないなら写しのままでよく、理由をメモに)。
- **やったこと (T1616):**
  - ファイルのダイアログをバックエンドで開く `open_file_dialog` (`directory` でフォルダーも)・`save_file_dialog` を足し
    (`src-tauri/src/dialog.rs`)、選ばれたパスを `FileAccess` (`src-tauri/src/access.rs`、`AppState.files`) に覚える。
    capability から `dialog:allow-open`・`dialog:allow-save` を外し (`acl.rs` のテストが `dialog:` の許可が無いことを確かめる)、
    画面の `@tauri-apps/plugin-dialog` の依存を外した (third-party-notices と pnpm-lock も)。画面の `fileDialogs.ts` は
    `ipc.openFileDialog`・`ipc.saveFileDialog` を呼ぶ (`FileDialogs` の形と `window.__UGOE_FILE_DIALOGS__` の差し替えはそのまま)。
  - エラーのコードはビューア・スタジオと同じ `file.not_chosen`・`file.not_chosen_for_save`・`dialog.failed` (訳・docs/editor.md の表・messageCodes.ts)。
  - 保存のダイアログの拡張子はバックエンドが付ける (最初のフィルタの最初の拡張子。画面の `withExtension` と同じ規則の文字列の比べ方なので、
    画面が付け直しても書けるパスと食い違わない)。
  - **コマンドの一覧と規則** (docs/editor.md#読み書きできるパス に表):
    - 読む (開くダイアログ・ドロップ・開いているプロジェクトのフォルダーの中 (`..` を含むものは除く)・プロジェクトが指すもの
      (モジュールの本体・プレビューの着せ替えの一覧・提案の元の PSD)): `open_project`・`new_project_from_psd`・`import_psd`・
      `open_sample_project` (`parent`)・`read_module_base`・`new_module_project`・`open_module_with_base`・`module_open_problem`・
      `reload_base`・`classify_psd_layers` (`psd`)・`assign_texture_normal_map`・`set_preview_modules`・`read_module_file`。
    - 書く (保存のダイアログで選んだパスに 1 回だけ): `save_project_as`・`export_bundle`・`export_module`・`write_png_file`・
      `write_json_file`・`save_claude_desktop_bundle`。
    - 例外: `open_sample_project` は選んだフォルダー (読める) の中に見本のフォルダーを新しく作って書く (既にあるものには書かない)。
      `module_open_problem` が返した本体 (選んだモジュールのプロジェクトの `project.json` が指すもの) は読めるパスに加える
      (問題の枠の「Open base project」)。`save_project`・`reload_project` はバックエンドの保存先だけを使うので選び直さない。
      ドロップは `on_webview_event` で読めるパスに加える (ビューアと同じ)。覚えるのはセッションの間だけ (エディタには最近のファイルが無いので保存しない)。
  - `write_json_file` は同期のコマンドだったので `async` にした (AGENTS.md の Tauri のコマンドの規則)。`new_module_project`
    (本体を読む) も `async` にした。
  - テスト: `access.rs` (選んでいない・フォルダーの外・`..`・書けるのは 1 回・コマンドの入口の `readable_path`/`writable_path`)、
    `dialog.rs` (拡張子)、`acl.rs` (dialog の許可が無い)。e2e (`just e2e --only editor`、11 手順すべて通った): 画面の
    `__UGOE_FILE_DIALOGS__` の差し替えではバックエンドが選んだパスを知らないので、デバッグビルドだけのバックエンドの差し替え
    (環境変数 `UGOE_EDITOR_DIALOG_ANSWERS` のディレクトリの答えのファイル) に替え、選んでいないパスの `import_psd`・`export_bundle` が
    断られる手順を足した (tests/e2e/run.mjs・lib/harness.mjs・specs/editor.test.mjs・README.md。`crates:` の外だが、指示どおり e2e を直した)。
- **`write_png_file` の共有 (T0032 の検討):** 写しのままにした。パーセント符号の復号は IPC のヘッダーの受け方 (アプリの IPC の都合) で、
  PNG の先頭の検査は 8 バイトの比べだけなので、`ugoe-fs` (MIT のファイルの読み書きの補助) に入れる利点が小さい。エラーもアプリごとの
  型 (`EditorError`・ビューアの `CommandError`) で返すので、共有しても写しの量はほとんど減らない。`ugoe-fs` はこのタスクの `crates:` の外でもある。
- **確かめていないこと:** 実際の OS のダイアログ (Linux の GTK・Windows・macOS) を手で開いての確認はしていない (e2e はバックエンドの差し替えを通る)。
  特に `default_path` の扱い (ディレクトリならそこ、そうでなければ親とファイル名) は `@tauri-apps/plugin-dialog` の `defaultPath` を写したもので、
  Windows のパスの区切りの直し (`components().collect()`) はしていない。`set_parent` は開くダイアログでは Windows・macOS だけ、保存では常に付ける (ビューア・スタジオと同じ)。
- **起票の候補:**
  - 3 つのアプリの `FileAccess` (読める・1 回だけ書ける) と `with_extension`・`path_string`・ダイアログの組み立てが写しになっている。
    共有する置き場 (アプリ間で共有する Rust のクレート) を作るか検討する。
  - 画面の `withExtension` を保存のダイアログの後に呼んでいる箇所 (fileMenu・saveScreenshot・trackingHost・ConnectAgents) は、バックエンドが
    付けるので本物のアプリでは何もしない (ブラウザの差し替えのために残した)。差し替えの側で付けるようにして画面から外せる。
- **検査:** main (T1615 まで) に rebase した後で `just ci` が通った (exit 0)。`just e2e --only editor` も rebase の後でもう一度通した (11 手順)。
  rebase では `proposals.rs` (T1615 の `layer_labels(document)`) と衝突し、両方を残して解いた。
