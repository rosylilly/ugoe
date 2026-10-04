---
id: T0557
title: "ugoe-mcp: ツールのエラーの構造 (code・args)・Undo の可否・確認の取り消しの種類を出す (判断が要る)"
status: done
milestone: M9
crates: [ugoe-mcp, apps/editor, docs]
depends_on: [T0556]
assignee: claude-worker-T0557
---

# T0557: ugoe-mcp: ツールのエラーの構造 (code・args)・Undo の可否・確認の取り消しの種類を出す (判断が要る)

## 目的

MCP サーバの公開の約束 (ツールの結果と `ugoe_mcp::host`) に、機械が読める情報が足りない。

- ツールのエラーは文の末尾の `(code: ...)` だけで、`structuredContent` の `code`・`args` で返していない (T0554 の候補)。
- ツールの状態に `can_undo`・`can_redo` が無く、エージェントが戻れないのに Undo を求めて利用者に確認を出させる (T1605 の候補)。
- `Approvals::request` は取り消しを `HostError::Denied` で返し、エディタは拒否の文の違いで取り消しを見分けている (T1603)。
  `HostError` に `Cancelled` を足し、種類で見分ける (T1603 の候補)。

**判断が要る:** どれも公開 API (MCP のツールの結果の形・`ugoe_mcp::host` の型) の追加・変更。足すだけで互換を保つ形にするかを決める。

## 受け入れ条件

- [x] 3 つの変更の形を決めた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [x] ツールのエラーの `structuredContent` に `code`・`args` が入るテストがある (今の文は残す)
- [x] ツールの状態に `can_undo`・`can_redo` があり、ファイルの持ち主とエディタの持ち主の両方で正しい値になるテストがある
- [x] `HostError::Cancelled` があり、エディタ (`apps/editor/src-tauri/src/mcp.rs` の `ApprovalOutcome::of`) が文でなく種類で取り消しを見分ける。テストがある
- [x] crates/ugoe-mcp/README.md・docs/ai-integration.md・docs/manual/mcp.md を更新した
- [x] `just ci` が通る

## 参考

- T0554・T1605・T1603 のメモ (起票の候補)
- crates/ugoe-mcp/src/host.rs

## メモ

- T0026 で起票した (元の候補: T0554 の `structuredContent`、T1605 の `can_undo`・`can_redo`、T1603 の `HostError::Cancelled`)。
  同じクレートを触る T0556 の後に着手する。

### やったこと (T0557)

- **形の判断:** 3 つとも **足すだけ** にした (team-lead の指示「既存の形を壊さず足す」)。今の文・今の項目は変えていない。
  - **エラー:** コードの分かる失敗は `isError: true` の結果に `structuredContent` を足し、エディタの `Message` と同じ形
    (`code`・`args`・`message`・省略できる `cause`・`items`) で載せる。`message` は `content` の文と同じ。コードの無い失敗は今までどおり
    文だけ (`structuredContent` を持たない)。新しい形を作らず、エディタの IPC で公開している `Message` を使い回した。
    コードを付けたのは: 提案 (`proposal.*`。`proposal_number` の読めない ID も `proposal.not_found`)、PSD の読み込み・取り込み
    (`psd.*`・`project.*`・`module.*`・`edit.*`)、持ち主の失敗 `host.denied`・`host.timed_out`・`host.cancelled` (MCP だけの新しいコード。
    エディタのカタログには足していない。`HostError::Failed` はコードを持たないので文だけ)。
    実装は `tools.rs` の `ToolError` (`From<String>` で今の `?` をそのまま通す。`Message` は `Box`)。
  - **状態:** プロジェクトの状態 (`Tools::status`) に `can_undo`・`can_redo` を足した (`Document::can_undo`・`can_redo`。T1605)。
  - **取り消し:** `HostError::Cancelled` を足し、`Approvals::request` は取り消しをそれで返す (文は前と同じ "the client cancelled the request")。
    `HostError` は `#[non_exhaustive]` にしていない (crates.io に出さないクレートで、エディタの `match` に新しい種類を必ず扱わせたいので)。
    エディタの `ApprovalOutcome::of` は種類で見分ける (`Denied`・`Failed` → `declined`、`Cancelled` → `cancelled`)。
- テスト: `tests/stdio.rs` (`proposal.stale`・`proposal.not_found` の `args.id` (数と文字列)・`proposal.rejected` の `cause`・
  コードの無いエラーに `structuredContent` が無いこと・開いた/適用/Undo/Redo の `can_undo`・`can_redo`)、`tests/editor.rs`
  (拒否 `host.denied`・時間切れ `host.timed_out`・エディタの持ち主での `can_undo`・`can_redo`)、`autorig.rs` (PSD の上限の
  `code`・`args.max`)、`host.rs` (取り消しが `HostError::Cancelled`)、エディタの `mcp_tests.rs`
  `approval_outcomes_follow_the_error_kind` (どんな拒否の文でも取り消しにならない)。
- 文書: `crates/ugoe-mcp/README.md` (状態の項目・エラーのコードの表・取り消し)・`docs/ai-integration.md`・`docs/manual/mcp.md`・
  `docs/editor.md` (画面へのイベントの 1 行)。
- `just ci` が通った (main に rebase した後)。

### やらなかったこと・確かめていないこと

- `host.cancelled` は、取り消した要求に答えない (MCP の仕様) ので、ふつうクライアントには届かない。HTTP で切断したときの答えには載るが、
  それをテストでは確かめていない (`host.rs` の単体テストで種類を確かめた)。
- エディタの持ち主の `HostError::Failed` (`EditorHost` の `failed(Message)`) はコードを捨てて文だけにしている (`HostError` の形を変えないため)。
- 実際の MCP クライアント (Claude Code など) で `structuredContent` 付きのエラーがどう見えるかは確かめていない。
- ツールの定義に `outputSchema` は無いので、エラーの `structuredContent` の形は README に書いただけ。

### 起票の候補

- `HostError::Failed` にコード (`Message`) を持たせ、エディタの持ち主の失敗 (保存の失敗 `project.*` など) のコードも MCP に載せる。
- ほかのコードの無いツールのエラー (引数の誤り・パスの誤り・`--root` の外・要素が無いなど) に MCP のコード (`mcp.invalid_arguments` など) を付けるか。
