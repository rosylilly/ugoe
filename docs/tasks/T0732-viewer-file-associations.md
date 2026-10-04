---
id: T0732
title: ビューアを OS の「このアプリで開く」に対応させる (fileAssociations と macOS の RunEvent::Opened)
status: todo
milestone: M9
crates: [apps/viewer, docs]
depends_on: []
assignee:
---

# T0732: ビューアを OS の「このアプリで開く」に対応させる (fileAssociations と macOS の RunEvent::Opened)

## 目的

`apps/viewer/src-tauri/src/lib.rs` の `initial_file` の doc コメントは「OS の『このアプリで開く』」と書くが、`tauri.conf.json` に
`bundle.fileAssociations` が無い。macOS はファイルを起動の引数ではなく `RunEvent::Opened` で渡すので、対応しても届かない (T0724 の所見と起票の候補)。

## 受け入れ条件

- [ ] `tauri.conf.json` の `bundle.fileAssociations` に `.ugoe`・`.ugoemod` を登録した
- [ ] macOS の `RunEvent::Opened` を受け、起動時と起動中のどちらでもファイルを開く (起動中に開いたときの扱いを決めて docs に書いた)。開いたパスは `FileAccess` に許可として加える
- [ ] 受け取ったパスを開く部分 (引数・`Opened` の両方) のテストがある
- [ ] `initial_file` の doc コメントと apps/viewer/README.md・docs/manual を実際に合わせた
- [ ] `just ci` が通る (Windows・macOS の実機で確かめていないことはメモに書き、T0731 の手動確認の項目に足した)

## 参考

- T0724 のメモ (所見の表と起票の候補)、T0718 のメモ、T0731

## メモ

- T0026 で起票した (元の候補: T0724 の「このアプリで開く」)。オーケストレーターの集計には無かったが、M9 の done のタスクのメモの起票の候補にあった。
