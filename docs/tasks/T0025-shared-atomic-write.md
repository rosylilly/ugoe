---
id: T0025
title: 一時ファイルを経た置き換えを 1 か所にまとめ、MCP の保存の書き込みをロックの外へ出す
status: blocked
milestone: M9
crates: [ugoe-editor-core, ugoe-mcp, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0687]
assignee: claude-worker-T0025
---

# T0025: 一時ファイルを経た置き換えを 1 か所にまとめ、MCP の保存の書き込みをロックの外へ出す

## 目的

「同じフォルダーの一時ファイルに書いてから名前を変える」処理が、ビューア (`apps/viewer/src-tauri/src/atomic_write.rs`、T0728)・
エディタ (T0687)・スタジオ・`ugoe-mcp` (`StagedFile`、T0552)・`ugoe-editor-core` (`write_if_changed`) に別々にある (T0728 の候補)。
片方だけ直る (Windows の置き換え・権限・失敗時の後始末) のを避けるため、1 か所にまとめる。

あわせて、エディタの MCP の `save_project` は、ドキュメントのロックを握ったままファイルを書く (T0685 の候補。写しを取ってロックを放してから書き、
保存の印を付ける)。

**判断が要る:** 置き場所。候補は (a) `ugoe-editor-core` に置き、アプリと `ugoe-mcp` が使う (ビューア・スタジオが editor-core に依存してよいか)、
(b) ツール層の小さなクレートを新しく作る (`scripts/check-layers.mjs`・`ci-groups.mjs`・commitlint の SCOPES への登録が要る)、
(c) GPL のアプリで共有する場所を作る。ライセンス (MIT のクレートと GPL のアプリ) と依存方向から決め、決めた理由を docs に書く。

## 受け入れ条件

- [ ] 置き場所を決めた (上の判断)。利用者の判断が要るなら案と推奨をメモに書いて止まる
- [ ] 3 つのアプリ・`ugoe-mcp`・`ugoe-editor-core` の写しを、まとめた 1 つに置き換えた。失敗したときに前のファイルが残るテストと、利用者だけが読める権限で作るテストがまとめた側にある
- [x] エディタの MCP の `save_project` が、ロックを放してからファイルを書く (書いている間にほかの読むだけのツールが答えるテスト)
- [ ] `just ci` が通る

## 参考

- T0728・T0552・T0685 のメモ (起票の候補)、T0687
- apps/viewer/src-tauri/src/atomic_write.rs

## メモ

- T0019 で起票した (元の候補: T0728 の 1 つにまとめる、T0552 の `EditorHost::save` (書き方そのものは T0687 の範囲)、T0685 の `save_project` のロック)。
- エディタの一時ファイル経由の書き方は T0687 が入れるので、その後に着手する。
- T0025 (止めた理由、**判断が要る: 置き場所**): 今の依存方向の規則 (`scripts/check-layers.mjs`) では、既存のクレートのどれに置いても
  5 か所 (エディタ・ビューア・スタジオ・`ugoe-mcp`・`ugoe-editor-core`) から使えない。
  - ビューアの許された依存は `RUNTIME_TIER` だけ、スタジオは `RUNTIME_TIER` と `ugoe-headless` だけ。ランタイム層はファイル I/O を持てない
    (AGENTS.md のアーキテクチャの 3.。`ugoe-format` はバイト列だけ)。`ugoe-headless` は editor-core が依存できない (GPU を引き込む) うえ、ビューアにも許されていない。
  - 写しは 6 つ: `apps/editor/src-tauri/src/atomic_write.rs` (`StagedFile`・`Access::Private`、T0687)、`apps/viewer/src-tauri/src/atomic_write.rs` (T0728)、
    スタジオの `profile.rs` (`<名前>.tmp` に書いて rename。`sync_all` なし) と `api/store.rs`、`crates/ugoe-mcp/src/host.rs` の `StagedFile` (T0552。
    rename の前に閉じない)、`crates/ugoe-editor-core/src/project/files.rs` の `write_if_changed` (`<名前>.tmp`。rename の前に閉じない)。
  - 案:
    - **(b) 推奨: ツール層に内部の依存を持たない小さな MIT のクレート `ugoe-fs` (仮。`ugoe-atomic-file` も可) を作る。** 中身はエディタの
      `atomic_write.rs` (`write_atomic`・`StagedFile`・`Access`) をそのまま移し、テスト (失敗で前のファイルが残る・`0o600` で作る) も移す。
      `check-layers.mjs` に `"ugoe-fs": new Set()` を足し、ビューア・スタジオ・editor-core・mcp・editor の許す依存に加える (内部のクレートに
      依存しない葉なので、ランタイム層へ流れ込む向きはできない)。ランタイム層 (wasm) からは使わない。`ci-groups.mjs` は `tools` か `assets`、
      commitlint の SCOPES に `fs` を足す。外部の依存は増えない (`std` だけ。テストに `tempfile`)。T0011 の `read_file.rs` (3 つのアプリの写し) も
      同じクレートに寄せられる。公開はしない (`publish = false`)。
    - (a) `ugoe-editor-core` に置く: ビューアとスタジオが editor-core (PSD・メッシャ・リギングまで) に依存することになり、`check-layers.mjs` の
      「スタジオはエディタの中核に依存しない」(ADR 0028 の 1.) と、ビューアはランタイム層だけ、という決めを覆す。ビルドも重くなる。勧めない。
    - (c) GPL のアプリだけで共有する場所 (例: `apps/shared-tauri/` のクレート): `ugoe-mcp`・`ugoe-editor-core` (MIT) からは使えないので、
      写しが 2 つ残り、目的 (1 か所にまとめる) を満たさない。勧めない。
  - 決まれば、残りの受け入れ条件 (6 つの写しの置き換えと、まとめた側のテスト) を続けられる。置き換えの際に、editor-core の `write_if_changed` と
    MCP の `StagedFile` は rename の前に閉じる・一時ファイルの名前を `.<名前>.<プロセス ID>.<連番>.tmp` に揃える (T0687 の候補)、`mcpb::Bundle::write` も
    同じ関数で書く (`sync_all`。T0552 の候補) を入れる。
- T0025 (やったこと、置き場所と関係のない部分):
  - **保存の書き込みをロックの外へ (T0685・T0687 の候補):** `ProjectSession::begin_save` (ロックの中で写しを取る) → `PendingSave::write`
    (ロックの外で書いて指紋を取る) → `WrittenSave::finish` (ロックを取り直して印を付ける) に分けた。`save_to` もこれを続けて呼ぶ。
    エディタの MCP の `save` (ブリッジの `save_project`) と IPC の `save_project` がロックの外で書く。`.ugoe` の書き出し (MCP の `bundle`) もロックの外。
    `save_project_as` は名前と保存先を変えて失敗時に戻すので、ロックを持ったまま書く。
  - editor-core の `Document` に `begin_save` (状態の番号を返し、まとめを切る) と `mark_saved_state(state)` を足した (`mark_saved` はこの 2 つを続けて呼ぶ)。
    書いている間の編集は変更ありのまま、Undo で保存した状態に戻れば変更なし。書いている間にメタ情報が変われば未保存の印を残し、プロジェクトを
    替えていれば (セッションの番号で比べる) 印を付けない。
  - 保存が重ならないよう `AppState::saving` (`Mutex<()>`) を足した (順序は `saving` → `document` → `project`)。保存・`check_external_change`・
    `reload_project` はこれを取るので、書きかけのファイルを外の変更と見て自動で開き直すことはない。
  - テスト: `mcp_tests::saving_writes_the_files_outside_the_document_lock` (写しを取った後で止め、別のクライアントの `get_model_summary` が
    答え、書いている間の編集が未保存で残る。ロックを持ったまま止めると失敗するのを確かめた)、`project::tests::saving_outside_the_lock_keeps_changes_made_while_writing`、
    `undo_tests::saving_a_copy_keeps_edits_made_while_writing`。docs/editor.md の「外での書き換え」に書いた。
- T0025: ここまでの変更で `just ci` は通った (main 6e173503 の上)。置き換えが残るので受け入れ条件の `just ci` には印を付けていない。
- 起票の候補: `save_project_as` もロックの外で書く (メタ情報の戻し方を写しの側で持つ)。
