---
id: T0039
title: "e2e: 提案のプレビューの筋書きを、見本の着せ替えの本体に取り込んだ PSD で通す"
status: done
milestone: M9
crates: [tests/e2e, docs]
depends_on: []
assignee: claude-worker-T0039
---

# T0039: e2e: 提案のプレビューの筋書きを、見本の着せ替えの本体に取り込んだ PSD で通す

## 目的

T1627 で、見本の着せ替えの本体 (`dress-up.ugoeproj`) に `character.psd` を取り込んでから「Propose rig」すると、本体の物理の衝突体
`Shoulders` が消えて提案が作れない不具合を直した (`ugoe_ai::rig::propose_rig` がドキュメントを受け取るようにした)。
ところが e2e の「proposes a rig from a PSD with a fake AI provider and previews it」(T0029) は「New from PSD…」の新しいプロジェクトで
提案していて、この経路 (物理とスロットを持つ本体に提案を重ねる) を通らない (T1627 の候補)。

## 受け入れ条件

- [x] e2e の提案の筋書き (または新しい筋書き) が、見本を開き、着せ替えの本体に「Import PSD…」で `character.psd` を取り込み、偽の AI のプロバイダで
      「Propose rig」→「Preview」→「End preview」まで通る (提案の一覧が出る・本体の衝突体 `Shoulders` を含む物理が残る、を確かめる)
- [x] 「New from PSD…」の経路も、別の筋書きか同じ筋書きの前半で今までどおり確かめている (どちらを残したかをメモに書く)
- [x] T1627 の直しを戻すと、この筋書きが落ちることを確かめた (メモに書く。戻した変更はコミットしない)
- [x] tests/e2e/README.md の筋書きの表を直した
- [x] `just e2e --only editor` が通る
- [x] `just ci` が通る

## 参考

- T1627 のメモ (原因と直し方)、T0029 のメモ (偽の AI のプロバイダ・押す操作)
- tests/e2e/specs/editor.test.mjs、tests/e2e/lib/fake-ai.mjs、tests/e2e/README.md

## メモ

- T0038 で起票した (元の候補: T1627)。
- やったこと (tests/e2e/specs/editor.test.mjs):
  - 提案の筋書きを 3 つの手順に分けた。「points the AI provider at a fake one」(設定の AI の送り先を偽のプロバイダにする。
    前の筋書きの前半)、「proposes a rig on the dress-up base with an imported PSD and previews it」(新しい)、
    「proposes a rig in a new project from a PSD and previews it」(前の筋書きの「New from PSD…」の経路を残した)。
    提案 → Preview → End preview の確かめ方は 2 つの経路で同じなので `proposeAndPreview` にまとめた。
  - 着せ替えの本体の手順: モジュールのプロジェクトから「Open…」で `dress-up.ugoeproj` を開き直し、本体の物理に衝突体
    `Shoulders`・スロット `Hair` の `colliders` が `["Shoulders"]` であることを確かめる → 「Import PSD…」で `character.psd` →
    偽のプロバイダで「Propose rig」(提案の一覧が出る・提案に `add_physics_rig` がある) → Preview → End preview (ドキュメントは
    変わらない) → 「Adopt」で採用し、`get_physics` に本体のリグがすべて残り髪揺れのリグが足され、衝突体 `Shoulders` と
    スロット `Hair` の参照が残ることを確かめる。採用まで進めたのは、「物理が残る」をプレビューの複製でなくドキュメントで見るため。
  - 「New from PSD…」の手順は採用した変更を捨てる確認を通り、物理の無い (`get_physics` が null の) 新しいプロジェクトで提案する。
  - `countInvokes` の数は通算なので、2 回目のプレビューでは押す前との差で `get_proposal_preview_bundle` が 1 回成功したことを見る。
- T1627 の直しを戻した確認: `git revert --no-commit f2cb5fa1` (e2e の筋書きは今のものを残す) でビルドして `just e2e --only editor` を
  流すと、新しい手順が `the proposal failed: The model returned a proposal that cannot be used (no valid rig after 3 attempts:
  command would make the model invalid: slot `Hair`: colliders refers to unknown physics collider `Shoulders`).` で落ち、
  ほかの手順は通った。戻した変更は捨てた (コミットしていない)。
- 文書: tests/e2e/README.md (筋書きの表・「偽の AI のプロバイダ」の提案の元)、docs/testing.md (筋書きの一覧)。
- `just e2e --only editor` (直しを戻さないビルド) と `just ci` が通った。
- 確かめていないこと: CI (`e2e.yml`) での実行。
- 起票の候補: なし。
