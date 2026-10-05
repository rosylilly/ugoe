---
id: T1627
title: "エディタ: 着せ替えの本体に PSD を取り込んでから Propose rig すると、スロットの衝突体の参照で提案が作れない"
status: done
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

- [x] 再現する (AI あり・なしの両方で確かめ、どちらで起きるかをメモに書いた)。先に失敗するテスト (ugoe-ai か editor-core) がある
- [x] 着せ替えの本体 (スロットが物理の衝突体を参照する) でも提案が作れる。スロットの参照を壊さない、または壊すなら分かる警告にする (どちらにしたかと理由をメモに)
- [x] docs/editor.md (か docs/ai-integration.md) の該当の節を直した
- [x] `just ci` が通る

## 参考

- T0029 のメモ (起票の候補)、tests/e2e/specs/editor.test.mjs
- crates/ugoe-ai/src/rig.rs・module_rig.rs、docs/spec/modules.md (スロットの colliders)

## メモ

- オーケストレーターが直接起票した (T0029 の候補。バグなので候補の一覧を待たずに起票)。
- 原因: `ugoe_ai::rig::propose_rig` (と `propose_rig_with_ai(_guided)`) がモデルだけを受け取り、物理の設定の無い
  ドキュメントに提案を試していた。髪揺れの `AddPhysicsRig` が既定の物理の設定を新しく作るので、本体の衝突体 `Shoulders` が
  無くなり、スロット `Hair` の `colliders` が何も指さなくなって検証 (`validate_physics`) を通らなかった。本当のドキュメントに
  適用すれば通る提案でも、提案を作る段階で落ちていた。
- 再現: AI なし (名前の規則・テンプレートの既定値) と AI あり (偽のプロバイダ) の **両方** で起きる。AI ありでは問い直しの末に
  `no valid rig after N attempts: ...`、AI なしでは同じ検証のエラーがそのまま出る。先に失敗するテストを入れた
  (ugoe-ai の `tests/auto_rig.rs` の `a_dress_up_base_whose_slot_exposes_a_collider_is_rigged(_with_ai)`。最初のコミット)。
  エディタでも見本の本体 (`dress-up.ugoe`) に `character.psd` を取り込んで AI なし・ありで提案するテスト
  (`proposals_tests.rs` の `a_dress_up_base_with_an_imported_psd_is_rigged`) を足し、直す前のコードで同じエラーで落ちることを確かめた。
- 直し方: **スロットの参照を壊さない** ほうにした。`propose_rig`・`propose_rig_with_ai`・`propose_rig_with_ai_guided` の引数を
  `&Model` から `&Document` に変え (モジュールのリグ `propose_module_rig` と同じ形)、ドキュメントの物理の設定に髪揺れのリグを足す。
  本体のリグ・衝突体は残り、リグの ID は本体のリグと重なれば `unique_id` で `_2` などを付ける。理由: 提案は実際に適用する
  ドキュメント (物理を含む) で検証するのが正しく、警告にする理由が無い。
  - 例外: 物理の設定が無いのにスロットが衝突体を公開している本体 (仕様上は有効。スロットの参照は何も指さない) では、物理を
    足すとスロットの参照が壊れて検証を通らない。そこでは髪揺れを組まず、理由に
    `Not rigged: hair sway physics, because the model has no physics but its slots (...) expose physics colliders; ...` と書く
    (分かる警告。テスト `hair_sway_is_skipped_when_slots_expose_colliders_without_physics`)。
- 文書: docs/ai-integration.md の「テンプレート」(入力がドキュメントであること、本体の物理に足すこと、上の例外) と、
  docs/editor.md の AI タブの流れを直した。
- `crates:` の外の小さな変更: `crates/ugoe-mcp/src/autorig.rs` (MCP の `propose_rig` も同じ関数を呼ぶので、引数をドキュメントに
  した。MCP でも同じ不具合が直る)、`tests/e2e/specs/editor.test.mjs` (「見本の本体では提案が作れない」という古いコメントを直した。
  筋書きは変えていない)。ugoe-editor-core は変えていない。
- 確かめていないこと: e2e (`just e2e`) は流していない (筋書きを変えていないため)。
- `just ci` は通った。
- 起票の候補:
  - e2e: 「proposes a rig ... with a fake AI provider」の筋書きを、見本の着せ替えの本体に取り込んだ PSD で提案する形にする
    (今は「New from PSD…」の新しいプロジェクトで通している)。

