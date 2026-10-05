---
id: T0733
title: "ビューア: Windows・Linux で起動中に開いたファイルを今のウィンドウで開く (single-instance)"
status: in-progress
milestone: M9
crates: [apps/viewer, docs]
depends_on: []
assignee: claude-worker-T0733
---

# T0733: ビューア: Windows・Linux で起動中に開いたファイルを今のウィンドウで開く (single-instance)

## 目的

T0732 でビューアを OS の「このアプリで開く」に対応させた。macOS は起動中に開くと同じウィンドウで開く (`RunEvent::Opened`) が、
Windows・Linux は新しいプロセスが起動して新しいウィンドウになる (T0732 の候補)。`tauri-plugin-single-instance` で 2 つ目の起動の引数を 1 つ目へ渡し、
`open_from_os` を通して今のウィンドウで開く。新しい外部の依存なので、ライセンス (`just licenses`) と同梱の表記 (`third-party-notices.txt`) を確かめる。

## 受け入れ条件

- [ ] Windows・Linux で 2 つ目の起動が 1 つ目にファイルを渡して終わり、1 つ目のウィンドウで開く (前に出す)。引数の解釈は今の起動時の経路と同じ
- [ ] 引数から開くファイルを選ぶ処理のテストがある
- [ ] docs/manual-check.md の V08 を、起動中に開いたときの期待 (すべての OS で同じウィンドウ) に直した
- [ ] 実機で確かめていないことをメモに書いた (T0731 の手動確認に回す)
- [ ] `just ci` が通る

## 参考

- T0732 のメモ (起票の候補)
- apps/viewer/README.md、docs/manual-check.md

## メモ

- T0032 で起票した (元の候補: T0732 (single-instance))。
