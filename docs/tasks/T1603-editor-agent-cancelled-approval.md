---
id: T1603
title: エディタの確認のダイアログで、エージェントが要求を取り消したことを知らせる
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee:
---

# T1603: エディタの確認のダイアログで、エージェントが要求を取り消したことを知らせる

## 目的

T0551 で、MCP の中継が `notifications/cancelled` を受けると、エディタで待っている確認 (提案の適用・保存) を拒否で終えるようにした。
エディタの確認のダイアログは、時間切れ・拒否と同じく黙って消えるだけで、利用者は「なぜ消えたか」が分からない。
「エージェントが取り消した」と短く知らせる。

## 受け入れ条件

- [ ] 確認が取り消しで終わったことが、時間切れ・利用者の拒否と区別してエディタの画面に届く (IPC のイベントの形を `apps/editor/src/lib/ipc.ts` と揃える)
- [ ] ダイアログを閉じるときに、取り消されたことを知らせる (文言は i18n)。テストがある
- [ ] `docs/ai-integration.md#実行中のエディタにつなぐ-adr-0023` に書いた
- [ ] `just ci` が通る

## 参考

- T0551 のメモ (起票の候補)
- apps/editor/src-tauri/src/mcp.rs、crates/ugoe-mcp/src/host.rs (`Approvals`)

## メモ

- T0019 で起票した (元の候補: T0551 の確認のダイアログ)。
