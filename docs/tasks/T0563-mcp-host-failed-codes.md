---
id: T0563
title: "ugoe-mcp: 持ち主の失敗 (HostError::Failed) と、コードの無いツールのエラーにもコードを付ける (判断が要る)"
status: todo
milestone: M9
crates: [ugoe-mcp, apps/editor, docs]
depends_on: []
assignee:
---

# T0563: ugoe-mcp: 持ち主の失敗 (HostError::Failed) と、コードの無いツールのエラーにもコードを付ける (判断が要る)

## 目的

T0557 で MCP のツールのエラーに構造 (`structuredContent` の `code`・`args`) を付けた。次はまだコードが無い (T0557 の候補)。

1. `HostError::Failed(String)` (`crates/ugoe-mcp/src/host.rs`) は文だけを持つので、エディタの持ち主の失敗 (保存の失敗 `project.*` など) のコードが MCP の結果に載らない。
   `Failed` にコード (`Message`) を持たせる。`HostError` は ugoe-mcp の公開の型で、エディタ (`apps/editor`) が持ち主を実装している。
2. **判断が要る:** ほかのコードの無いツールのエラー (引数の誤り・パスの誤り・`--root` の外・要素が無いなど) に MCP のコード (`mcp.invalid_arguments` など) を付けるか。
   付けるなら、コードの名前の付け方と ADR 0029 の訳の表 (`codes.json`) に入れるか。

## 受け入れ条件

- [ ] `HostError::Failed` がコードと引数を持ち、エディタの持ち主の保存などの失敗のコードが MCP の結果の `structuredContent` に載る。そのテストがある
- [ ] コードの無いツールのエラーにコードを付けるかの案と推奨をメモに書き、利用者の判断を得た。付けるなら、ツールのエラーの一覧と crates/ugoe-mcp/README.md・docs を揃えた
- [ ] `just ci` が通る

## 参考

- T0557 のメモ (起票の候補)
- ADR 0029、crates/ugoe-mcp/README.md

## メモ

- T0032 で起票した (元の候補: T0557 (2 つ))。
