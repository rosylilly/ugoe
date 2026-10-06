---
id: T1631
title: "エディタ: 提案に添えた分類の直しを 1 つのまとまりで出し、まとめて外せるようにする"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1631
---

# T1631: エディタ: 提案に添えた分類の直しを 1 つのまとまりで出し、まとめて外せるようにする

## 目的

T0570 で、MCP のエージェントのリグの提案は、エージェントが決めたレイヤーの分類を `set_layer_correction` として先頭に添えるようになった。
エディタの提案の一覧では、その直しが 1 行ずつ出る (レイヤーの数だけ並び、リグのコマンドが下に押しやられる)。外すのも 1 行ずつ (T0570 のやらなかったこと・候補)。

## 受け入れ条件

- [x] 提案の一覧で、続く `set_layer_correction` を 1 つのまとまり (「分類を残す N 件」など) で出し、開くと各行が見える。まとめて外す・戻すことができる
      (外すのは既存の除外 (`exclude`) と同じ。添えた直しはリグのコマンドが頼らないので、外してもリグは残る)
- [x] 部品のテスト (まとめて出す・まとめて外す・1 行だけ外す・ほかの種類のコマンドと混ざったとき) がある
- [x] docs/editor.md と docs/manual/ai.md の提案の一覧の説明を直した
- [x] `just ci` が通る

## 参考

- T0570 のメモ (提案の形・やらなかったこと・起票の候補)
- apps/editor の提案の一覧 (AI のタブ)、crates/ugoe-mcp/README.md (エージェントの分類を残す)

## メモ

- T0040 で起票した (元の候補: T0570 (提案の一覧の分類の直し))。
- やったこと:
  - `apps/editor/src/lib/ai/proposals.ts` の `groupCommands`: 2 つ以上続いた `set_layer_correction` を、対象が違っても 1 つのまとまり
    (`CommandGroup.labels`、名前は「Keep the classification of N layers」/「N 個のレイヤーの分類を残す」) にする。ほかの種類のコマンドとは
    同じ対象でもまとめない。まとめて外す・戻すのは既存の `toggleGroup` (直しは `requires` に入らないのでリグは残る)、1 行は `toggleCommand`。
  - `ProposalsPanel.svelte`: まとまりの見出しは件数入りの文 (code にしない)、開いた行は `set_layer_correction <レイヤーのパス>`。
  - テスト: `proposals.test.ts` (まとめる・まとめて外す/戻す・1 行だけ外す・同じ対象のほかの種類と混ざったとき)、
    部品のテスト `ProposalsPanel.svelte.test.ts` (happy-dom に描いて、まとまりの見出しと各行・まとまりのチェックで 3 行が外れリグは残る・1 行だけ外すと中間の印)。
  - 文書: docs/editor.md (一部だけの採用・分類の直しのエージェントの提案から)、docs/manual/ai.md (一部だけ採用する)。
- やらなかったこと:
  - 1 つだけの `set_layer_correction` はまとめず、今までどおり 1 行 (`set_layer_correction <パス>`) で出す。
  - 行に直しのラベル・左右は出していない (`ProposalCommand` は `op`・`subject`・`requires` だけ。出すなら IPC に足す)。
  - `just ui-audit` は流していない (既存の行の構造を使い、新しい部品・色は足していない)。
- 起票の候補:
  - 提案の一覧の分類の直しの行に、残すラベルと左右 (例: `Face / Eye R → eye_white (right)`) を出す (`ProposalCommand` に要約を足す。src-tauri と apps/editor)。
