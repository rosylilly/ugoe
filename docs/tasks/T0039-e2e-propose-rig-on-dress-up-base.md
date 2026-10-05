---
id: T0039
title: "e2e: 提案のプレビューの筋書きを、見本の着せ替えの本体に取り込んだ PSD で通す"
status: in-progress
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

- [ ] e2e の提案の筋書き (または新しい筋書き) が、見本を開き、着せ替えの本体に「Import PSD…」で `character.psd` を取り込み、偽の AI のプロバイダで
      「Propose rig」→「Preview」→「End preview」まで通る (提案の一覧が出る・本体の衝突体 `Shoulders` を含む物理が残る、を確かめる)
- [ ] 「New from PSD…」の経路も、別の筋書きか同じ筋書きの前半で今までどおり確かめている (どちらを残したかをメモに書く)
- [ ] T1627 の直しを戻すと、この筋書きが落ちることを確かめた (メモに書く。戻した変更はコミットしない)
- [ ] tests/e2e/README.md の筋書きの表を直した
- [ ] `just e2e --only editor` が通る
- [ ] `just ci` が通る

## 参考

- T1627 のメモ (原因と直し方)、T0029 のメモ (偽の AI のプロバイダ・押す操作)
- tests/e2e/specs/editor.test.mjs、tests/e2e/lib/fake-ai.mjs、tests/e2e/README.md

## メモ

- T0038 で起票した (元の候補: T1627)。
