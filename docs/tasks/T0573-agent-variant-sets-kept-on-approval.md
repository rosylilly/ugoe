---
id: T0573
title: "MCP: エージェントの差分の組 (variant_sets) も承認を経て残す (判断が要る)"
status: in-progress
milestone: M9
crates: [ugoe-mcp, ugoe-editor-core, docs]
depends_on: []
assignee: claude-worker-T0573
---

# T0573: MCP: エージェントの差分の組 (variant_sets) も承認を経て残す (判断が要る)

## 目的

T0570 で、エージェントが答えた分類 (ラベルと左右) は、リグの提案に添えた `set_layer_correction` を利用者が承認すると `project.json` の直しとして残るようになった
(利用者の判断: 案 A)。差分の組 (`variant_sets`) と確からしさは直しに含まれないので残らない (T0570 のやらなかったこと・候補)。
差分の組も承認を経て残す形を決めて入れる。

**判断が要る:** 案:

- A. 直し (`editor.classification` の `corrections`) に組の欄を持たせ、`set_layer_correction` で一緒に提案する (提案の数は増えない。直しの形が変わる)。
- B. 組の直しを別の項目とコマンド (`set_variant_set_correction` など) にし、リグの提案に添える (直しの形はそのまま。コマンドが増える)。
- C. 今のまま (組は残さない。エディタで開き直した後は、自動の分類か利用者の分類の組を使う)。

推奨は判断の前に、組を使う処理 (自動リギングの差分の扱い・エディタの提案の一覧) を調べて書く。

## 受け入れ条件

- [ ] 案と推奨 (ファイル形式・Undo・エディタの提案の一覧での見え方・自動の分類との重ね方) をメモに書き、利用者の判断を得た
- [ ] 決めた形で実装し (C なら文書だけ)、エージェントの組が承認して保存し開き直した後に残ることを MCP の `tests/stdio.rs` で確かめる
- [ ] crates/ugoe-mcp/README.md・docs/ai-integration.md・docs/editor.md (形を変えるなら docs/spec/file-formats.md) を直した
- [ ] `just ci` が通る

## 参考

- T0570 のメモ (判断・提案の形・やらなかったこと)
- crates/ugoe-mcp、crates/ugoe-editor-core (`SetLayerCorrection`)、crates/ugoe-ai (`classify`)

## メモ

- T0040 で起票した (元の候補: T0570 (`variant_sets`))。
