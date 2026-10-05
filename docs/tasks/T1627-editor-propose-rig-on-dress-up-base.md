---
id: T1627
title: "エディタ: 着せ替えの本体に PSD を取り込んでから Propose rig すると、スロットの衝突体の参照で提案が作れない"
status: in-progress
milestone: M9
crates: [ugoe-ai, ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T1627
---

# T1627: エディタ: 着せ替えの本体に PSD を取り込んでから Propose rig すると、スロットの衝突体の参照で提案が作れない

## 目的

T0029 の e2e を作る途中で、見本の着せ替えの本体 `assets/samples/dress-up/dress-up.ugoeproj` に `character.psd` を取り込んでから
「Propose rig」を押すと、提案が作れないことが分かった (T0029 の候補)。エラーは
`slot "Hair": colliders refers to unknown physics collider "Shoulders"`。見たのは偽の AI のプロバイダを使ったときだけで、
AI を使わないとき (名前の規則だけ) も同じかは確かめていない。

リグの提案が物理を作り直すときに、スロットが参照する衝突体 (`colliders`) を消すか名前を変えているのが原因と見込まれる。

## 受け入れ条件

- [ ] 再現する (AI あり・なしの両方で確かめ、どちらで起きるかをメモに書いた)。先に失敗するテスト (ugoe-ai か editor-core) がある
- [ ] 着せ替えの本体 (スロットが物理の衝突体を参照する) でも提案が作れる。スロットの参照を壊さない、または壊すなら分かる警告にする (どちらにしたかと理由をメモに)
- [ ] docs/editor.md (か docs/ai-integration.md) の該当の節を直した
- [ ] `just ci` が通る

## 参考

- T0029 のメモ (起票の候補)、tests/e2e/specs/editor.test.mjs
- crates/ugoe-ai/src/rig.rs・module_rig.rs、docs/spec/modules.md (スロットの colliders)

## メモ

- オーケストレーターが直接起票した (T0029 の候補。バグなので候補の一覧を待たずに起票)。
