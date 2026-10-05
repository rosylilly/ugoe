---
id: T1629
title: "エディタ: 分類のキャッシュのあるプロジェクトを開くときの PSD の読み直しを、開いた後のバックグラウンドに回す"
status: in-progress
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1629
---

# T1629: エディタ: 分類のキャッシュのあるプロジェクトを開くときの PSD の読み直しを、開いた後のバックグラウンドに回す

## 目的

T0567 で、保存先のあるプロジェクトを開くとき (`ProjectSession::from_project`) に、分類のキャッシュ (`.ugoe-cache/classification.json`) の
PSD の大きさ・ハッシュ・レイヤーが合えば分類と PSD を戻すようになった。合うかを確かめるために開くたびに PSD を読み直してハッシュを求めるので、
大きな PSD のプロジェクトでは開くのがその分遅くなる (T0567 の候補)。

## 受け入れ条件

- [ ] プロジェクトを開く処理は PSD の読み直しを待たずに終わり、確かめと分類の戻しは開いた後にバックグラウンドで行う (ロックの外。AGENTS.md の Tauri のコマンドの規則)
- [ ] 戻す前にほかのプロジェクトを開いた・閉じた・分類し直したときに、遅れて終わった戻しが今の状態を上書きしない (世代の番号。AGENTS.md の「非同期の開始と停止」)。テストがある
- [ ] 戻したことを画面が知る方法 (`get_proposals` を読み直すきっかけのイベントなど) を決め、IPC を変えたら `apps/editor/src/lib/ipc.ts` も直した
- [ ] 既存の `the_classification_is_restored_when_the_project_is_reopened` が (待ち方を変えて) 通る
- [ ] docs/editor.md の「分類の答えのキャッシュ」を直した
- [ ] `just ci` が通る

## 参考

- T0567 のメモ (やらなかったこと・起票の候補)
- apps/editor/src-tauri (`ProjectSession::from_project`)、crates/ugoe-editor-core/src/classification_cache.rs

## メモ

- T0038 で起票した (元の候補: T0567 (開くときの読み直し))。
