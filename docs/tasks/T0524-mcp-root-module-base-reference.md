---
id: T0524
title: MCP: --root のとき、モジュールのプロジェクトが参照する本体も root の下に限る
status: done
milestone: M6
crates: [ugoe-mcp, ugoe-editor-core, docs]
depends_on: [T0515, T0662]
assignee: claude-worker-T0524
---

# T0524: モジュールの本体の参照を --root で限る

## 目的

T0662 の報告で見つかった穴を塞ぐ。`--root` を付けても、`open_project` で開いたモジュールのプロジェクトが本体の参照で root の外を
指していると、その本体を読んでしまう (T0515 の前から)。`--http` で外のエージェントに開くときに、root の外のファイルが読まれうる。

## 受け入れ条件

- [x] `--root` のとき、モジュールのプロジェクトの本体の参照 (`.ugoeproj`・`.ugoe`) が root の外を指していたら、読まずに分かるエラーを返す
      (どの参照が外を指しているかを書く。シンボリックリンクで外へ出る場合も含む)
- [x] `import_psd` の `base`・そのほか本体やモジュールを読むツールで、パスの検査に漏れが無いことを洗い出してテストする
- [x] `--root` を付けないとき (stdio の既定) の挙動は変えない
- [x] ugoe-mcp の README の「パスを限る」と `docs/manual/mcp.md` を更新する。`just ci` が通る

## 参考

- `docs/tasks/T0515-mcp-root-dir.md`・`docs/tasks/T0662-mcp-module-autorig-and-sample-shoulders.md` のメモ

## メモ

- **やったこと**
  - **editor-core に本体を読む前の検査の口:** `project::BaseCheck` (`&dyn Fn(&Path) -> Result<PathBuf, String>`)、
    `Project::load_checking_base(dir, check)`・`load_base_checked(path, check)`。解決した本体の絶対パスを、何も開く前に (存在の確認より前に)
    1 度だけ渡し、返したパスから読む。エラーなら新しい `ProjectError::BaseRefused { path, reason }`。保存する参照の元 (`ModuleBase::path`) は
    元のパスのままなので、保存しても `module.base` は変わらない。`Project::load`・`load_base` は確かめない関数で同じ道を通る (挙動は同じ)。
    editor-core は MCP やパスの規則を知らない。
  - **本体がモジュールのとき:** `load_base` は `.ugoeproj` の `project.json` の種類を先に見て `BaseIsModule` にする (前は `Project::load` で
    そのモジュールの本体まで読んでから断っていた。root の外へたどる道になる)。エディタでも同じエラーで、たどらなくなっただけ。
  - **ugoe-mcp:** 検査は `Root` に集めた。`Root::check_base` (正規化 → root の下か → `.ugoeproj` なら中のリンク)、`Root::check_project_dir`
    (host.rs から移した。プロジェクトの中のリンクの検査)。`FileHost::load` (open_project・開いていないプロジェクトを指す全ツールの遅延読み込み・
    save_project) は `--root` のとき `load_checking_base` に `check_base` を渡す。`import_psd` の `base` は `resolve` の後に `check_base` も通す
    (本体の `.ugoeproj` の中の外へのリンク)。外のまだ無い本体も「外」と答える (外のファイルの有無を探らせない)。
    エラーは `base model <解決した本体のパス>: outside the root directory `<root>` (... `module.base` in project.json must point under it)`。
  - **洗い出し:** tools.rs・autorig.rs でファイルを読む箇所はすべて `resolve` を通る (path・psd・bundle・motion・expression・base)。本体を読む
    経路は `FileHost::load` と `import_psd` (`Project::new_module`) だけ。保存・書き出しはメモリの本体を使い、読み直さない。
  - **テスト:** editor-core `base_check_runs_before_the_base_is_read` (参照の解決・断ると読まない・無い本体でも先に確かめる・返したパスから読み参照は
    元のまま・モデルのプロジェクトでは呼ばない)、`missing_base_and_slot_are_reported` を `BaseIsModule` だけに締めた。ugoe-mcp
    `root_confines_module_bases` (相対 `..`・絶対・途中の `..`・外のまだ無い本体・外を指すリンク・中のファイルが外を指す本体の `.ugoeproj`・
    本体がモジュール、を open_project・get_model_summary・save_project・import_psd の base で。`--root` なしでは外の本体を読める)。
  - 文書: ugoe-mcp の README「パスを限る」、docs/manual/mcp.md、docs/ai-integration.md「パスを限る」、docs/editor.md「本体」。
  - `just ci` が通る (main は 52c6b77 のまま。先に `just wasm` 2 つを実行)。
- **確かめていないこと:** Windows (検査は T0515 の `canonicalize_lenient` を使うので同じ扱いのはず。テストは unix のリンクの部分を `cfg(unix)`)。
  `just test-gpu` (描画は触っていない)。
- **防がないこと (T0515 と同じ):** 判定と読み込みの間の差し替え (TOCTOU)。返した正規化したパスから読むので、判定した場所を読む。
  ハードリンクは見分けられない。
- **後続の候補:** エディタに組み込んだ MCP (`--editor`・エディタの持ち主) には root が無い (エディタの利用者の権限で開く) ので対象外。
  エディタの `load_module_with`・`reload_base` などの付け替えの口には検査を渡していない (MCP から呼ばないため)。

