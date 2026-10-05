---
id: T0566
title: "ugoe-mcp: レイヤーの分類に project.json の editor.classification の直しを重ねる"
status: todo
milestone: M9
crates: [ugoe-mcp, docs]
depends_on: []
assignee:
---

# T0566: ugoe-mcp: レイヤーの分類に project.json の editor.classification の直しを重ねる

## 目的

T1615 で、エディタで直したレイヤーの分類 (ラベル・左右) を `.ugoeproj` の `project.json` の `editor.classification` に残すようにした
(`ugoe_editor_core::classification`、`Command::SetLayerCorrection`)。エディタは分類に直しを重ねて使うが、MCP サーバの分類
(`get_psd_layers`・`set_layer_labels`・`propose_rig`・`propose_module_rig`) は自分のメモリの分類だけを使い、直しを読まない (T1615 の候補)。
外部のエージェントの提案が、利用者がエディタで直した答えと食い違う。

## 受け入れ条件

- [ ] MCP サーバが開いたプロジェクトの `editor.classification` の直しを、名前の規則とエージェントの答えの分類の上に重ねる (エディタの `corrected_labels` と同じ規則: 直したレイヤーは確からしさ 1・語彙にないラベルは重ねない)
- [ ] `get_psd_layers` の結果で直したレイヤーが分かる。`set_layer_labels` が直したレイヤーを上書きしないか、上書きするなら規則を README に書いた
- [ ] テスト (`tests/stdio.rs` か autorig のテスト) で、直しのあるプロジェクトで分類とリグの提案が直しに従うことを確かめた
- [ ] crates/ugoe-mcp/README.md と docs/ai-integration.md を直した
- [ ] `just ci` が通る

## 参考

- T1615 のメモ (決めたこと・起票の候補)
- crates/ugoe-mcp/src/autorig.rs、crates/ugoe-editor-core/src/classification.rs、apps/editor/src-tauri/src/proposals.rs (`corrected_labels`)

## メモ

- T0036 で起票した (元の候補: T1615 (MCP))。
- 重ねる処理をエディタ (GPL) と MCP (MIT) で共有するなら、置き場は MIT の側 (`ugoe-editor-core` か `ugoe-ai`) になる。範囲の外なので、写すか移すかをメモに書く。
