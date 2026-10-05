---
id: T0570
title: "MCP のエージェントの分類の答えをエディタに残す方法 (直しの提案・キャッシュへの書き込み。判断が要る)"
status: in-progress
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

- [ ] 案と推奨 (ファイル形式・Undo・エディタにつないだときの振る舞い・AI の答えの扱い) をメモに書き、利用者の判断を得た
- [ ] 決めた形で実装し (D なら文書だけ)、エージェントの答えが開き直した後に残ることを MCP の `tests/stdio.rs` (とエディタのテスト) で確かめる
- [ ] crates/ugoe-mcp/README.md・docs/ai-integration.md・docs/editor.md (と、キャッシュの形を変えるなら docs/spec/file-formats.md) を直した
- [ ] `just ci` が通る

## 参考

- T0566・T0567 のメモ (起票の候補・利用者の判断)
- crates/ugoe-mcp/src/autorig.rs、crates/ugoe-editor-core/src/classification_cache.rs、crates/ugoe-ai (`classify::corrected_labels`・`cached_labels`)

## メモ

- T0038 で起票した (元の候補: T0566 (直しの提案)・T0567 (キャッシュの書き手))。
- 判断 (2026-10-06、利用者の判断): 案 A。エージェントは分類の直しを提案し (`set_layer_correction` を提案のコマンドに入れる)、利用者が承認したら `project.json` の直しとして残る (Undo できる)。キャッシュの書き手は増やさない。提案の形は案と理由をメモに書いて決める。
