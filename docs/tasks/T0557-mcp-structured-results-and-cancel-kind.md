---
id: T0557
title: "ugoe-mcp: ツールのエラーの構造 (code・args)・Undo の可否・確認の取り消しの種類を出す (判断が要る)"
status: todo
milestone: M9
crates: [ugoe-mcp, apps/editor, docs]
depends_on: [T0556]
assignee:
---

# T0557: ugoe-mcp: ツールのエラーの構造 (code・args)・Undo の可否・確認の取り消しの種類を出す (判断が要る)

## 目的

MCP サーバの公開の約束 (ツールの結果と `ugoe_mcp::host`) に、機械が読める情報が足りない。

- ツールのエラーは文の末尾の `(code: ...)` だけで、`structuredContent` の `code`・`args` で返していない (T0554 の候補)。
- ツールの状態に `can_undo`・`can_redo` が無く、エージェントが戻れないのに Undo を求めて利用者に確認を出させる (T1605 の候補)。
- `Approvals::request` は取り消しを `HostError::Denied` で返し、エディタは拒否の文の違いで取り消しを見分けている (T1603)。
  `HostError` に `Cancelled` を足し、種類で見分ける (T1603 の候補)。

**判断が要る:** どれも公開 API (MCP のツールの結果の形・`ugoe_mcp::host` の型) の追加・変更。足すだけで互換を保つ形にするかを決める。

## 受け入れ条件

- [ ] 3 つの変更の形を決めた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [ ] ツールのエラーの `structuredContent` に `code`・`args` が入るテストがある (今の文は残す)
- [ ] ツールの状態に `can_undo`・`can_redo` があり、ファイルの持ち主とエディタの持ち主の両方で正しい値になるテストがある
- [ ] `HostError::Cancelled` があり、エディタ (`apps/editor/src-tauri/src/mcp.rs` の `ApprovalOutcome::of`) が文でなく種類で取り消しを見分ける。テストがある
- [ ] crates/ugoe-mcp/README.md・docs/ai-integration.md・docs/manual/mcp.md を更新した
- [ ] `just ci` が通る

## 参考

- T0554・T1605・T1603 のメモ (起票の候補)
- crates/ugoe-mcp/src/host.rs

## メモ

- T0026 で起票した (元の候補: T0554 の `structuredContent`、T1605 の `can_undo`・`can_redo`、T1603 の `HostError::Cancelled`)。
  同じクレートを触る T0556 の後に着手する。
