---
id: T0566
title: "ugoe-mcp: レイヤーの分類に project.json の editor.classification の直しを重ねる"
status: done
milestone: M9
crates: [ugoe-mcp, docs]
depends_on: []
assignee: claude-worker-T0566
---

# T0566: ugoe-mcp: レイヤーの分類に project.json の editor.classification の直しを重ねる

## 目的

T1615 で、エディタで直したレイヤーの分類 (ラベル・左右) を `.ugoeproj` の `project.json` の `editor.classification` に残すようにした
(`ugoe_editor_core::classification`、`Command::SetLayerCorrection`)。エディタは分類に直しを重ねて使うが、MCP サーバの分類
(`get_psd_layers`・`set_layer_labels`・`propose_rig`・`propose_module_rig`) は自分のメモリの分類だけを使い、直しを読まない (T1615 の候補)。
外部のエージェントの提案が、利用者がエディタで直した答えと食い違う。

## 受け入れ条件

- [x] MCP サーバが開いたプロジェクトの `editor.classification` の直しを、名前の規則とエージェントの答えの分類の上に重ねる (エディタの `corrected_labels` と同じ規則: 直したレイヤーは確からしさ 1・語彙にないラベルは重ねない)
- [x] `get_psd_layers` の結果で直したレイヤーが分かる。`set_layer_labels` が直したレイヤーを上書きしないか、上書きするなら規則を README に書いた
- [x] テスト (`tests/stdio.rs` か autorig のテスト) で、直しのあるプロジェクトで分類とリグの提案が直しに従うことを確かめた
- [x] crates/ugoe-mcp/README.md と docs/ai-integration.md を直した
- [x] `just ci` が通る

## 参考

- T1615 のメモ (決めたこと・起票の候補)
- crates/ugoe-mcp/src/autorig.rs、crates/ugoe-editor-core/src/classification.rs、apps/editor/src-tauri/src/proposals.rs (`corrected_labels`)

## メモ

- T0036 で起票した (元の候補: T1615 (MCP))。
- 重ねる処理をエディタ (GPL) と MCP (MIT) で共有するなら、置き場は MIT の側 (`ugoe-editor-core` か `ugoe-ai`) になる。範囲の外なので、写すか移すかをメモに書く。
- **置き場 (範囲の外の小さな変更):** 重ねる処理は MIT の `ugoe-ai` に **移した** (`ugoe_ai::classify::corrected_labels`・`correction_label`)。
  ugoe-ai は既に editor-core に依存し、ラベルの語彙 (`Label`) を持つので、語彙にないラベルを捨てる規則まで 1 か所に置ける
  (editor-core は ugoe-ai に依存できないので置けない)。エディタ (`apps/editor/src-tauri/src/proposals.rs`) は自分の `corrected_labels`・
  `correction_of` を消してこれを使う (振る舞いは同じ。既存の `corrections_overlay_the_classification_and_undo` が通る)。
  ほかに範囲の外で直したのは、ugoe-ai の README の 1 文と、editor-core の `classification.rs` の冒頭のコメント (重ねる場所の参照) だけ。
- **やったこと:**
  - ugoe-mcp (`src/autorig.rs`): `PsdSource::labels_for` で、覚えている分類 (名前の規則 + エージェントの答え) に今の文書の
    `Document::layer_corrections` を重ねる。直しは文書の状態 (Undo の対象) なので、分類のメモリには入れず呼ぶたびに重ねる
    (エディタにつないだときはエディタの文書の直し)。`get_psd_layers` は直したレイヤーを `set_by: "user"`
    (確からしさ 1・食い違いの印なし)。`set_layer_labels` は **直したレイヤーを上書きしない** (答えは覚えるが直しが優先。
    結果の `kept_corrections` にその番号、`corrected_by_user` に直したレイヤーの番号、`set_by_agent` は直していないものだけ)。
    `get_rig_guide`・`propose_rig`・`get_module_rig_guide`・`propose_module_rig` は重ねた分類で組み、ガイドの `labels.corrected_by_user`
    と提案の理由 (`the user corrected N ...`) に数を出す。ツールの説明文にも書いた。
  - テスト: `tests/stdio.rs` の `layer_corrections_from_the_project_overlay_the_classification` (保存した `.ugoeproj` の `project.json` に
    直し (目の左右の入れ替えと語彙にないラベル) を書いて開き直し、一覧・`set_layer_labels`・ガイドの位置 (エディタと同じ関数で組んだものと一致)・
    リグの提案が直しに従うこと)。
  - 文書: crates/ugoe-mcp/README.md (新しい節「利用者の直し」・手順・ツールの表)、docs/ai-integration.md (「利用者の直し (T0566)」・
    覚えておくもの・分類の直しの重ね方)、docs/editor.md (重ね方の置き場・MCP も重ねる)、docs/manual/mcp.md (直しはエージェントにも効く)。
- **やらなかったこと:** MCP から直しを書くツール (直すのはエディタの利用者とした)。モジュールのリグの提案が直しに従うことは
  stdio のテストでは確かめていない (同じ `labels_for` を通るのでコードの上では同じ)。`corrected_labels` 自体の ugoe-ai の単体テストは
  足していない (エディタと MCP のテストで確かめている)。
- **起票の候補:**
  - MCP のエージェントが直しを提案できるようにする (`set_layer_correction` を提案のコマンドに入れる。今は `Batch` の中で拒むので、提案の形から考える)。
- `just ci` は通った (rebase main の後)。
