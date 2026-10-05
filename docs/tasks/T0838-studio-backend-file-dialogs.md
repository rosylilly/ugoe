---
id: T0838
title: "スタジオ: ファイルのダイアログをバックエンドで開き、コマンドが読み書きできるパスを利用者が選んだものに絞る"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0837]
assignee: claude-worker-T0838
---

# T0838: スタジオ: ファイルのダイアログをバックエンドで開き、コマンドが読み書きできるパスを利用者が選んだものに絞る

## 目的

ビューアは T0727 でファイルのダイアログをバックエンドで開き、コマンドが読み書きできるパスを利用者が選んだものに絞った (`FileAccess`)。
スタジオのコマンドは webview から渡されたパスを読み書きする (T0024 の候補)。プロファイルが参照するパス (シーンのモデル・モジュール・背景) は、
利用者が選んだもの (と読み込んだプロファイルにあるもの) として扱う。

## 受け入れ条件

- [x] ファイルを読み書きするスタジオのコマンドを一覧にし、ダイアログで選んだパスとプロファイルにあるパスだけを受け付ける。一覧と例外をメモに書いた
- [x] 選んでいないパスを拒むテストがある
- [x] 外部の操作 API (docs/studio-api.md) から渡すパスの扱いを決め、docs に書いた
- [x] `just ci` が通る (実際の OS のダイアログで確かめていないことはメモに)

## 参考

- T0024 のメモ (起票の候補)、apps/viewer/src-tauri/src/acl.rs・lib.rs (`FileAccess`)、T1616 (エディタ)

## メモ

- T0026 で起票した (元の候補: T0024 のスタジオ)。同じ apps/studio のバックエンドを触る T0837 の後に着手する。
- T0838 (やったこと):
  - ファイルのダイアログをバックエンドで開くコマンド `open_file_dialog(filterName, extensions)`・`save_file_dialog(filterName, defaultName)` を
    足し、操作ウィンドウの capability から `dialog:allow-open`・`dialog:allow-save` を外した (`acl.rs` のテスト
    `webview_cannot_open_file_dialogs_by_itself` が 2 つの capability を検査)。名前と形はビューアに揃えた (複数選択・`title` は無い。
    スタジオは 1 つずつ選び、題は OS の既定のまま)。保存は `.json` だけで、拡張子が無ければバックエンドが付ける (`host.ts` と
    `profile.ts` の `withExtension` は消し、テストを Rust に移した)。
  - 許可は `src-tauri/src/access.rs` の `FileAccess`。読める = ダイアログで選んだパス (セッションの間・覚えない)・今のプロファイルにあるパス
    (シーンのモデル・モジュール・背景の画像、ホットキーの着せ替えのモジュール)・最近のプロファイル。書ける = 保存のダイアログで選んだパスに
    1 回 (今の保存先は選び直さずに書ける)。エラーのコードはビューアと同じ `file.not_chosen`・`file.not_chosen_for_save`・`dialog.failed`
    (カタログに文を足した)。
  - 一覧 (README の「ファイルの読み書きの許可」に表):
    `open_profile(path)` (選んだ・最近)・`save_profile(path?)` (保存のダイアログで選んだもの。同期のコマンドだったので `async` にした)・
    `update_scene(scene)` (モデル・モジュール・背景のすべて)・`set_hotkeys(hotkeys)` (着せ替えのモジュール。`Result` を返すようにした)・
    `module_slot(path)`。
  - 例外: `forget_recent_profile(path)` (一覧から外すだけで読み書きしない)。`restore_profile`・`activate_scene`・`duplicate_scene`・
    ホットキーの操作・受け手の `scene_*` はパスを受けず、プロファイルにあるものだけを読む。
  - 操作 API: パスを受けない (シーンは ID、モジュールは `dressup.list` の ID = 今のプロファイルにあるものだけ。既に `wardrobe` で絞っていた)
    と決め、docs/studio-api.md の Security と docs/studio.md#外部-api に書いた。外からモデルやモジュールを足すメソッドを作るなら、
    操作ウィンドウで利用者が確かめる手順を挟む、と書いた。
  - テスト: `access.rs` (選んでいない・プロファイルに無いパスを拒む・シーンの 3 種類・ホットキーのモジュール・保存は 1 回)、
    `error.rs` (コードとカタログ)、e2e。
  - 文書: apps/studio/README.md (コマンドの表・権限・許可の節・エラーの表)、docs/studio.md、docs/studio-api.md、docs/architecture.md
    (dialog の表と違いの理由。エディタは T1616 として残す)。
  - npm の `@tauri-apps/plugin-dialog` をスタジオから外し、`third-party-notices.txt` を生成し直した (Rust の `tauri-plugin-dialog` は使う)。
- 範囲の外の小さな変更: `tests/e2e/` (run.mjs が `E2E_CONFIG_HOME` を渡し、harness.mjs に `configHome()`)。スタジオの e2e は
  `open_profile` を選んでいないパスで呼んでいたので、起動の前に見本のプロファイルを最近のプロファイル (`recent-profiles.json`) に置き、
  起動時の `restore_profile` で開くようにした。選んでいないパスの `open_profile` が `file.not_chosen` で拒まれることも確かめる。
- 確かめたこと: `just e2e --only studio` (Linux・WebKitGTK) が 12 件とも通った。
- 確かめていないこと: 実際の OS のダイアログ (Linux・Windows・macOS) でファイルを選んで開く・保存する操作 (e2e はダイアログを開けない)。
  Windows・macOS で親のウィンドウを付けたダイアログ (`set_parent`。ビューアと同じ書き方)。
- エディタ (T1616) と揃えるときの形: コマンドの名前・エラーのコードはビューアと同じ。許可の型は各アプリに置いた (ビューアは読めるパスを
  ファイルに覚え、スタジオはプロファイルが覚えるので覚えない、と違うため)。
- `git rebase main` (ff04d194) の後に `just ci` が通った。
