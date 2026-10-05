---
id: T1626
title: "エディタ: パラメータのグループを足す・消す・名前を変えるコマンドと画面"
status: in-progress
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

- [ ] `ugoe_editor_core::Command` にパラメータのグループを足す・消す (属するパラメータの割り当てを外す)・名前を変える (並べ替えも要れば) コマンドがあり、Undo できる。不正な入力 (重なる ID・無いグループ) はエラーのコードで拒む。テストがある
- [ ] エディタのパラメータのパネルから、グループを作る・名前を変える・消すことができる。部品のテストがある。文言は日英のカタログにある
- [ ] 見本 (`crates/ugoe-cli/src/sample.rs`) がモデルに直接書くのをやめてコマンドで足す (`committed_sample_is_up_to_date` が通る)
- [ ] docs/editor.md (コマンドの一覧・エラーのコード) と editor-core の README を直した。IPC を変えたら `apps/editor/src/lib/ipc.ts` も
- [ ] `just ci` が通る

## 参考

- T0468 のメモ (起票の候補)
- docs/spec/model.md (パラメータのグループ)、crates/ugoe-editor-core/src/command.rs (`SetParameterGroup`)

## メモ

- T0036 で起票した (元の候補: T0468 (グループのコマンド))。
