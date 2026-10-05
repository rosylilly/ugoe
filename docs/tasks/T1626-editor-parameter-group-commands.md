---
id: T1626
title: "エディタ: パラメータのグループを足す・消す・名前を変えるコマンドと画面"
status: done
milestone: M9
crates: [ugoe-editor-core, apps/editor, ugoe-cli, docs]
depends_on: []
assignee: claude-worker-T1626
---

# T1626: エディタ: パラメータのグループを足す・消す・名前を変えるコマンドと画面

## 目的

エディタ中核には、パラメータを既存のグループに割り当てる `Command::SetParameterGroup` しかなく、グループ (`ParameterGroup`) そのものを
足す・消す・名前を変えるコマンドが無い。T0468 では見本のアバターのグループの定義を、組み始める前の空のモデルに直接書いた (T0468 の候補)。
エディタの画面でもグループを作れない。

## 受け入れ条件

- [x] `ugoe_editor_core::Command` にパラメータのグループを足す・消す (属するパラメータの割り当てを外す)・名前を変える (並べ替えも要れば) コマンドがあり、Undo できる。不正な入力 (重なる ID・無いグループ) はエラーのコードで拒む。テストがある
- [x] エディタのパラメータのパネルから、グループを作る・名前を変える・消すことができる。部品のテストがある。文言は日英のカタログにある
- [x] 見本 (`crates/ugoe-cli/src/sample.rs`) がモデルに直接書くのをやめてコマンドで足す (`committed_sample_is_up_to_date` が通る)
- [x] docs/editor.md (コマンドの一覧・エラーのコード) と editor-core の README を直した。IPC を変えたら `apps/editor/src/lib/ipc.ts` も
- [x] `just ci` が通る

## 参考

- T0468 のメモ (起票の候補)
- docs/spec/model.md (パラメータのグループ)、crates/ugoe-editor-core/src/command.rs (`SetParameterGroup`)

## メモ

- T0036 で起票した (元の候補: T0468 (グループのコマンド))。
- やったこと (claude-worker-T1626):
  - editor-core: `Command` の末尾に `AddParameterGroup { group }`・`RemoveParameterGroup { id }` (属するパラメータの `group` を `null` にする)・
    `RenameParameterGroup { id, name }`・`ReorderParameterGroup { id, before }` を足した。重なる ID は `edit.duplicate_id`、無いグループは
    `edit.not_found`、`before` が自分なら `edit.self_reference` (どれも `kind` は `parameter group`)。並べ替えは `reorder_mesh` と同じ処理を
    `document/reorder.rs` の汎用の `reorder` にまとめて使う。テストは `tests/parameter.rs` (各コマンド・拒否・Undo で割り当てごと戻る)、
    モジュールのプロジェクトで `edit.read_only_base` になることを `module_project_tests/workspace.rs` に足した。`just gen` で TS の型・JSON Schema・見本を作り直した。
  - エディタ: パラメータのパネルの一覧の下に「Groups (N)」の折りたためる節 (`panels/parameters/ParameterGroups.svelte`。作る・名前の変更・削除・▲▼ の並べ替え・
    行にパラメータの数。モジュールのプロジェクトでは `BaseLock` で無効)、各パラメータの編集欄に「Group」の選択 (`ParameterGroupSelect.svelte`、
    `set_parameter_group`) を足した。コマンドの組み立ては `groups.ts` (Vitest)、部品のテストは `ParameterGroups.test.ts`。文言は日英のカタログに足し、
    エラーの引数の `parameter group` も `errorArgs` で訳す。`just ui-audit --only editor --quick` で新しい指摘は無い (節は閉じた状態でだけ撮られる)。
  - 見本: `sample.rs` は空のモデルにグループを書くのをやめ、`add_parameters` で `AddParameterGroup` を送る (`committed_sample_is_up_to_date` は通り、`avatar.ugoe` は変わらない)。
    `assets/samples/README.md` の同じ説明も 1 行直した (`crates:` の外だが文書の 1 行だけ)。着せ替えの本体 (`sample/dress_up.rs`) はモデルを丸ごとデータで組むのでそのまま。
  - IPC は変えていない (既存の `apply_command` で送る) ので `ipc.ts` は変えていない。
- やらなかったこと: グループのドラッグでの並べ替え (▲▼ だけ)。ビューアなど別のアプリの変更は無い。
- 起票の候補: エディタのパラメータのパネルの一覧そのものをグループごとに区切って見せる (今はプレビュー・ビューアだけが区切る)。
