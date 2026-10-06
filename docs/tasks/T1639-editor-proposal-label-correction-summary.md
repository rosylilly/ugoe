---
id: T1639
title: "エディタ: 提案の一覧の分類の直しの行に、残すラベルと左右を出す"
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0676]
assignee: claude-worker-T1639
---

# T1639: エディタ: 提案の一覧の分類の直しの行に、残すラベルと左右を出す

## 目的

T1631 で、エージェントの提案に添えた分類の直し (`set_layer_correction`) を提案の一覧の 1 つの行にまとめた。行はレイヤーの数を言うだけで、
どのレイヤーにどのラベルと左右を残すのかが一覧では分からない (T1631 の候補)。

## 受け入れ条件

- [x] バックエンド (`apps/editor/src-tauri`) の `ProposalCommand` に分類の直しの要約 (レイヤーの名前・ラベル・左右) があり、IPC の型 (`apps/editor/src/lib/ipc.ts`) を合わせた
- [x] 提案の一覧の分類の直しの行を開くと、レイヤーごとに `Face / Eye R → eye_white (right)` の形で出す (長い一覧の扱いをメモに書く)。部品のテストがある
- [x] src-tauri の要約の単体テストがある
- [x] docs/editor.md (提案の一覧) を直した
- [x] `just ci` が通る

## 参考

- T1631 のメモ (やったこと・起票の候補)
- apps/editor/src-tauri/src (`ProposalCommand`)、apps/editor/src/lib/ai (提案の一覧)

## メモ

- T0041 で起票した (元の候補: T1631 (分類の直しの要約))。T0676 (作業中) が AI のパネルを触るので、その後に着手する。
- やったこと:
  - `ProposalCommand` は `apps/editor/src-tauri` ではなく `crates/ugoe-editor-core/src/proposal.rs` にあるので、そこに
    `correction: Option<LayerCorrection>` (分類の直しが残すラベルと左右。直しを消すコマンドとほかの種類では JSON から省く。TS は
    `correction?: LayerCorrection`) を足し、`analyze` で `Command::SetLayerCorrection` から埋める。レイヤーのパスは今までどおり `subject`。
    `just gen` で `generated/types.ts` を作り直した (`ipc.ts` は生成した型をそのまま出すので書き換えは要らなかった。IPC のコマンドは変えていない)。
  - 画面: `proposals.ts` に `correctionLabel` (`Face / Eye R → eye_white (right)`。消すコマンドは `→ back to the classification`/
    「分類の答えに戻す」)。まとまりの中の行はこれ、1 つだけの直しの行は `commandLabel` が `set_layer_correction Face / Eye R → eye_white (right)` を出す。
    ラベルと左右はコマンドの値のまま (翻訳しない。行は code の表記)。
  - 長い一覧: まとまりは閉じて出し、開いたときだけ中の行を描く (T1631 のまま)。開いた中は全行を出し、長いパスやラベルは折り返す
    (`.commands code` に `overflow-wrap: anywhere`)。件数で切ったり送りを付けたりはしていない。
  - テスト: editor-core `proposal_tests::layer_correction_commands_summarize_the_kept_label`、src-tauri
    `proposals_tests::proposal_list_summarizes_layer_corrections` (`get_proposals` の JSON)、`proposals.test.ts` (`correctionLabel`・`commandLabel`)、
    部品のテスト `ProposalsPanel.svelte.test.ts` (開いた行がレイヤーごとに `→ label (side)`、消す直しの行)。
  - 文書: docs/editor.md (AI の提案の一部だけの採用・一覧に出す対象)、docs/manual/ai.md (一部だけ採用する)。
  - `just e2e --only editor` は通った (15 件)。`just ci` も通った。
- `crates:` の外の変更 (小さなもの):
  - `crates/ugoe-editor-core` (`ProposalCommand` に欄を足した。上記)。
  - `crates/ugoe-mcp`: MCP の提案の結果も同じ `ProposalCommand` なので `correction` が出る。`tests/stdio.rs` の期待値と README (エージェントの分類を残す) を直した。
- やらなかったこと:
  - `just ui-audit` は流していない (既存の行の文字を変えただけで、部品・色は足していない)。
- 起票の候補: なし。
