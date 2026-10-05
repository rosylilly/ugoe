---
id: T1620
title: "エディタ: 「名前を付けて保存」もドキュメントのロックの外でファイルを書く"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T1616]
assignee: claude-worker-T1620
---

# T1620: エディタ: 「名前を付けて保存」もドキュメントのロックの外でファイルを書く

## 目的

T0025 で `save_project` はドキュメントの写しを取ってから、ロックの外でファイルを書くようにした。`save_project_as`
(`apps/editor/src-tauri/src/project.rs`) は、まだロックを握ったまま書く (T0025 の候補)。大きなプロジェクトを別名で保存する間、ほかのコマンドが止まる。
メタ情報 (保存先・保存済みの印) の戻し方を写しの側で持ち、ロックの外で書く。保存のダイアログを触る T1616 の後に着手する。

## 受け入れ条件

- [x] `save_project_as` がロックの外でファイルを書き、書き終えてから保存先と保存済みの印をドキュメントへ戻す。書く途中の編集を失わない (保存済みの印は書いた版にだけ付く)
- [x] 書く間にほかのコマンドが進めることと、失敗したときに保存先が変わらないことのテストがある
- [x] docs/editor.md の保存の説明を更新した
- [x] `just ci` が通る

## 参考

- T0025 のメモ (起票の候補)
- docs/editor.md

## メモ

- T0032 で起票した (元の候補: T0025 (save_project_as))。
- やったこと: `ProjectSession::begin_save_as` が、名前 (ディレクトリ名) と着せ替えの一覧 (新しい保存先からの相対パス) を付け替えたメタ情報を
  写し (`PendingSave`) の側だけで作る。`save_project_as` はロックの中で写しを取り、ロックの外で書き、ロックを取り直して `WrittenSave::finish` で
  名前と一覧をセッションでも付け替えてから (保存先を替える前に) 保存先・指紋・保存の印を戻す。セッションは書けるまで変えないので、失敗時の
  巻き戻し (旧 `save_as` の `old_meta`) は不要になった。`dress_up::moved` (セッションを変えずに付け替えた値を返す) を足した。
  `ProjectSession::save_to` はコマンドから使わなくなったので `#[cfg(test)]` にした (テストの準備用)。
- テスト (`project.rs`): `save_as_writes_outside_the_lock_and_keeps_changes_made_while_writing` (書く間にロックを取って編集・一覧の変更ができ、
  書いた版だけが保存済み。Undo で保存済みに戻る。書く間に足した一覧も新しい保存先から同じファイルを指す)・
  `failed_save_as_keeps_the_path_and_name` (書けない場所・書き込みの失敗で名前・保存先・保存済みの印が元のまま)。
  `save_as_keeps_the_dress_up_modules` は `AppState` を通す形に直した。
- 書く間の入れ子: テストでは `save_as` の書き込みを閉包で受け取り (本番は `PendingSave::write`)、その中でロックを取って編集する。
- やらなかったこと: 書いている間に名前を変える経路はない (名前は Save As だけが変える) ので、書いた名前で上書きする。
