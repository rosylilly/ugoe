---
id: T1629
title: "エディタ: 分類のキャッシュのあるプロジェクトを開くときの PSD の読み直しを、開いた後のバックグラウンドに回す"
status: done
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

- [x] プロジェクトを開く処理は PSD の読み直しを待たずに終わり、確かめと分類の戻しは開いた後にバックグラウンドで行う (ロックの外。AGENTS.md の Tauri のコマンドの規則)
- [x] 戻す前にほかのプロジェクトを開いた・閉じた・分類し直したときに、遅れて終わった戻しが今の状態を上書きしない (世代の番号。AGENTS.md の「非同期の開始と停止」)。テストがある
- [x] 戻したことを画面が知る方法 (`get_proposals` を読み直すきっかけのイベントなど) を決め、IPC を変えたら `apps/editor/src/lib/ipc.ts` も直した
- [x] 既存の `the_classification_is_restored_when_the_project_is_reopened` が (待ち方を変えて) 通る
- [x] docs/editor.md の「分類の答えのキャッシュ」を直した
- [x] `just ci` が通る

## 参考

- T0567 のメモ (やらなかったこと・起票の候補)
- apps/editor/src-tauri (`ProjectSession::from_project`)、crates/ugoe-editor-core/src/classification_cache.rs

## メモ

- T0038 で起票した (元の候補: T0567 (開くときの読み直し))。
- **やったこと:**
  - `ProjectSession::from_project` はキャッシュを戻さない (PSD を読まない)。開くコマンド (`open_project`・`reload_project`・
    `open_module_with_base`・`open_sample_project`) は、開いた後に `ProjectSession::classification_restore` の仕事を
    `proposals::restore_in_background` (`spawn_blocking`。ロックの外で PSD を読み、置くときだけ project のロックを取る) に渡す。
  - 世代の番号: 仕事はセッションの番号 (`ProjectSession::id`。開く・閉じるで変わる) と `ProposalSession` の `epoch`
    (`set_psd` で増える。分類 (`set_classification`)・PSD の取り込み・提案の元の PSD の選び直しで変わる) を覚え、
    PSD を読み終えた後でどちらかが違えば置かない (`restore_classification_in`)。
  - 画面への知らせ: 戻したらイベント `classification-restored` (中身なし) を送る。`ipc.ts` に `onClassificationRestored` を足し、
    `App.svelte` で受けて `proposals.refresh()` (`get_proposals` の読み直し)。IPC のコマンドの型は変えていない。
  - テスト: `the_classification_is_restored_when_the_project_is_reopened` を、開いた時点では戻っておらず
    `restore_classification_in` で戻る形に書き換えた (準備は `classified_project`・`reopen_in` に分けた)。新しく
    `a_late_classification_restore_does_not_overwrite_the_current_state` (ほかのプロジェクトを開いた・閉じた・分類し直した・
    PSD を選んだ後の遅い戻しは置かない)。
  - 文書: docs/editor.md の「分類の答えのキャッシュ」の「戻すとき」、docs/manual/ai.md (大きな PSD では少し遅れて出る)。
- **やらなかったこと:** 実際のエディタの画面での確認 (イベントを受けて AI タブが更新されること) はしていない (Rust のテストと svelte-check まで)。
  `restore_in_background` (Tauri の `AppHandle` を使う部分) 自体のテストはない (本体の `restore_classification_in` をテストした)。
  戻している最中であることを画面に出す表示は作っていない。
- rebase main の後、T0570 の `an_agent_proposal_keeps_its_labels_after_reopening` も開き直しの後に `restore_classification_in` で戻す形に直した。`just ci` は通った。
