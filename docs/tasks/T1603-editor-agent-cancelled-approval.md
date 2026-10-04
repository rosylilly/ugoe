---
id: T1603
title: エディタの確認のダイアログで、エージェントが要求を取り消したことを知らせる
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1603
---

# T1603: エディタの確認のダイアログで、エージェントが要求を取り消したことを知らせる

## 目的

T0551 で、MCP の中継が `notifications/cancelled` を受けると、エディタで待っている確認 (提案の適用・保存) を拒否で終えるようにした。
エディタの確認のダイアログは、時間切れ・拒否と同じく黙って消えるだけで、利用者は「なぜ消えたか」が分からない。
「エージェントが取り消した」と短く知らせる。

## 受け入れ条件

- [x] 確認が取り消しで終わったことが、時間切れ・利用者の拒否と区別してエディタの画面に届く (IPC のイベントの形を `apps/editor/src/lib/ipc.ts` と揃える)
- [x] ダイアログを閉じるときに、取り消されたことを知らせる (文言は i18n)。テストがある
- [x] `docs/ai-integration.md#実行中のエディタにつなぐ-adr-0023` に書いた
- [x] `just ci` が通る

## 参考

- T0551 のメモ (起票の候補)
- apps/editor/src-tauri/src/mcp.rs、crates/ugoe-mcp/src/host.rs (`Approvals`)

## メモ

- T0019 で起票した (元の候補: T0551 の確認のダイアログ)。
- やったこと:
  - バックエンド (`src-tauri/src/mcp.rs`): 画面へのイベント `approval_closed` に終わり方 `outcome` (`ApprovalOutcome`:
    `approved`・`declined`・`timed_out`・`cancelled`) を足した (`just gen` で `generated/types.ts`。`ipc.ts` は
    `export type *` で出すので変更なし)。`ugoe_mcp::host::Approvals::request` は取り消しを `HostError::Denied` で返し、
    拒否の文だけが違うので、文がエディタの渡した `DECLINED` でない拒否を取り消しとみなす (`ApprovalOutcome::of`)。
    ブリッジを止めたときの拒否は `declined`。
  - 画面 (`src/lib/agents/bridge.svelte.ts`): 待っている確認が `cancelled` で閉じたら、操作ごとの文言
    (`agents.approval.cancelled.{apply,undo,redo,save}`、en・ja) の Toast (warning) を出す。時間切れ・拒否・承認では出さない
    (時間切れはダイアログに残りの秒数を出しているので)。すでに閉じた確認には出さない。
  - テスト: `mcp_tests.rs` に `notifications/cancelled` で確認が `cancelled` で閉じる (中継をスレッドで使い、確認が出てから
    取り消す)、既存の流れのテストで拒否・時間切れ・承認の `outcome` を確かめた。`bridge.test.ts` に取り消しで Toast が出る・
    拒否と時間切れでは出ない、を足した。
  - 文書: `docs/ai-integration.md#実行中のエディタにつなぐ-adr-0023`・`docs/editor.md` (画面へのイベント)・`docs/manual/mcp.md`。
- やらなかったこと・制限:
  - `ugoe-mcp` (`crates:` の外) は変えていない。取り消しの判定は拒否の文の違いに頼る (`Approvals::request` の今の作りでは
    確か)。実際のエディタ (Tauri) の画面と実際の MCP クライアントでは確かめていない (偽の画面と中継で確かめた)。
- 起票の候補:
  - `ugoe_mcp::host::HostError` に取り消しの種類 (`Cancelled`) を足し、エディタは文でなく種類で見分ける (公開 API の変更)。
