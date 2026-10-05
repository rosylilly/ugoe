---
id: T0570
title: "MCP のエージェントの分類の答えをエディタに残す方法 (直しの提案・キャッシュへの書き込み。判断が要る)"
status: done
milestone: M9
crates: [ugoe-mcp, ugoe-ai, ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0570
---

# T0570: MCP のエージェントの分類の答えをエディタに残す方法 (直しの提案・キャッシュへの書き込み。判断が要る)

## 目的

MCP のエージェントが `set_layer_labels` で答えた分類は、サーバのメモリにしか残らない。

- T0566 で、MCP は利用者の直し (`editor.classification` の `corrections`) を読んで重ねるようになったが、エージェントは直しを書けない
  (`set_layer_correction` は提案の `Batch` の中で拒む)。直すのはエディタの利用者とした (T0566 の候補)。
- T0567 の利用者の判断 (案 C) で、分類の答えはプロジェクトの中のキャッシュ (`.ugoe-cache/classification.json`) に書き、MCP サーバも同じファイルを読む。
  T0567 は書き手をエディタだけにした。エージェントの答えもキャッシュに書けば、エディタで開いたときに戻る (書き手が 2 つになるので、`by_ai` に加えて
  誰の答えかを持たせる。T0567 の候補)。

2 つの候補は「エージェントの答えをエディタの側に残す」別の道なので、1 つのタスクで決める。

**判断が要る:** 案:

- A. 直しの提案: `set_layer_correction` を提案のコマンドに入れ、利用者が承認したらエージェントの答えが直しとして `project.json` に残る (Undo の対象。
  AGENTS.md の「AI はモデルを直接書き換えない」に沿う)。提案の形 (分類の直しだけの提案・リグの提案に添える) も決める。
- B. キャッシュへの書き込み: MCP の `set_layer_labels` がキャッシュに書き、エディタが開いたときに戻す。キャッシュに書き手 (`editor`・`agent`) を持たせる。
  エディタにつながっている (同じ文書を共有している) ときに誰が書くかを決める。T0567 の判断 (MCP は読む) を広げることになる。
- C. 両方。D. 今のまま (エージェントの答えは残さない)。

推奨は A (承認を経て残り、直しの仕組みと Undo をそのまま使える。キャッシュの書き手を増やさない)。

## 受け入れ条件

- [x] 案と推奨 (ファイル形式・Undo・エディタにつないだときの振る舞い・AI の答えの扱い) をメモに書き、利用者の判断を得た
- [x] 決めた形で実装し (D なら文書だけ)、エージェントの答えが開き直した後に残ることを MCP の `tests/stdio.rs` (とエディタのテスト) で確かめる
- [x] crates/ugoe-mcp/README.md・docs/ai-integration.md・docs/editor.md (と、キャッシュの形を変えるなら docs/spec/file-formats.md) を直した
- [x] `just ci` が通る

## 参考

- T0566・T0567 のメモ (起票の候補・利用者の判断)
- crates/ugoe-mcp/src/autorig.rs、crates/ugoe-editor-core/src/classification_cache.rs、crates/ugoe-ai (`classify::corrected_labels`・`cached_labels`)

## メモ

- T0038 で起票した (元の候補: T0566 (直しの提案)・T0567 (キャッシュの書き手))。
- 判断 (2026-10-06、利用者の判断): 案 A。エージェントは分類の直しを提案し (`set_layer_correction` を提案のコマンドに入れる)、利用者が承認したら `project.json` の直しとして残る (Undo できる)。キャッシュの書き手は増やさない。提案の形は案と理由をメモに書いて決める。
- 提案の形 (決めたこと): **リグの提案に添える。** `propose_rig`・`propose_module_rig` は、エージェントが決めたレイヤー (利用者が直したものを除く)
  の今の分類を `set_layer_correction` として提案の先頭に添える (`keep_labels: false` で添えない。結果の `label_commands` が番号)。理由:
  - リグはその分類で組むので、承認したリグと残る分類が食い違わず、承認は 1 回で済む。
  - 分類だけの提案を別に置くと、同じ状態に対する 2 つの提案のどちらかを適用したとき、もう一方が古く (stale) なって出し直しになる。
  - 添えた直しはリグのコマンドが頼らない (`requires` に入らない) ので、`exclude` で外してリグだけを入れられる。
  - リグなしで分類だけを残す・直す・消すときは、新しいツールを作らず `propose_edit` で `set_layer_correction` を提案する (コアの変更で `Batch` の中でも使えるようになった)。
- やったこと:
  - ugoe-editor-core: `SetLayerCorrection` を `Batch` の中でも使えるようにした (提案の採用は `Batch`。直しもその `Batch` と同じ 1 回の Undo。
    失敗した `Batch` は直しも残さない)。提案の一覧の対象 (`subject`) はレイヤーのパスを ` / ` でつないだもの。スキーマの説明を `just gen` で更新。
  - ugoe-ai: `classify::label_correction` (`correction_label` の逆)。
  - ugoe-mcp: リグの提案に分類の直しを添える・`keep_labels`・`label_commands`、差分 (`changes`) に `layer_corrections`、ツールの説明と使い方の文。
  - テスト: editor-core (`corrections_in_a_batch_share_its_undo_step`・`a_rejected_batch_keeps_the_corrections`・`proposals_can_carry_layer_corrections`)、
    ugoe-ai (`a_label_turns_into_a_correction_and_back`)、MCP の `tests/stdio.rs` (`agent_labels_are_kept_when_the_rig_is_approved`: 外す・Undo・
    保存して開き直すと `set_by=user`・`propose_edit` で消す。`agent_driven_auto_rig_from_psd` も開き直した後に `set_by=user`)、
    `diff.rs` の単体テスト、エディタの `an_agent_proposal_keeps_its_labels_after_reopening` (つないだエージェントの提案を採用 → 保存 → 開き直して「直した」)。
  - 文書: crates/ugoe-mcp/README.md (「エージェントの分類を残す」)・docs/ai-integration.md・docs/editor.md (`batch` の中でも使える。エラーのコードの説明)・
    docs/spec/file-formats.md (`editor.classification` の説明の言葉だけ。形は変えていない)。
- やらなかったこと:
  - 直しに出どころ (利用者かエージェントか) を持たせない。承認した答えは利用者の答えとして扱う (`set_by=user`)。ファイル形式は変えていない。
  - 差分の組 (`variant_sets`) と確からしさは残らない (直しはラベルと左右だけ)。
  - エディタの提案の一覧で、多数の `set_layer_correction` の行をまとめて出す表示は作っていない (1 行ずつ出る)。
- 起票の候補:
  - エディタの提案の一覧で、添えた分類の直しを 1 つのまとまり (「分類を残す N 件」) として出し、まとめて外せるようにする (apps/editor)。
  - 差分の組 (`variant_sets`) も承認を経て残す形 (直しに組を持たせるか、別の項目にするか。判断が要る)。

