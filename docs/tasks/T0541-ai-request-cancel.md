---
id: T0541
title: AI のリクエストのキャンセルと全体の時間の上限
status: done
milestone: M9
crates: [ugoe-ai, editor, docs]
depends_on: [T0527]
assignee: claude-worker-T0541
---

# T0541: AI のリクエストのキャンセルと全体の時間の上限

## 目的

T0527 のレビューの所見。`Provider::complete_json`・`DepthEstimator::estimate` は同期で、途中で止める手段がない。
1 回のリクエストの上限は 600 秒 (`ClaudeConfig::timeout`・`OpenAiConfig::timeout`) で、再試行 (既定 2 回、`retry-after` は 60 秒まで待つ) と、
OpenAI 互換の問い直し (`json_retries`)、リギングの問い直しが重なると、最悪で数十分かかる。エディタは `spawn_blocking` で待つだけで、利用者は止められない。

利用者が提案の作成を取り消せるようにし、1 回の提案にかける時間の合計にも上限を設ける。

## 受け入れ条件

- [x] 取り消しの手段を決める (例: `Provider` に渡す取り消しの印 (`Arc<AtomicBool>` など) を、再試行の待ち (`sleep`) と問い直しの間で確かめる。通信中の取り消しは `Transport` の時間の上限で抑える)。公開 API の変更なので、ADR が要るかを判断する
- [x] 取り消したときは専用のエラー (`AiError::Cancelled` など) を返し、エディタのエラーの文言 (ADR 0029 のコード) を足す
- [x] 1 回の提案の時間の合計の上限 (再試行と問い直しを含む) を設定に持つ
- [x] エディタの AI の操作に「取り消す」を付ける (見た目と配置は M10 の範囲に合わせる)
- [x] 取り消しと上限のテスト (`ReplayTransport` と差し替えた `sleep` で、実際には待たない)
- [x] `docs/ai-integration.md` に書く
- [x] `just ci` が通る

## 参考

- docs/ai-integration.md (Claude API のプロバイダ・OpenAI 互換の API)
- crates/ugoe-ai/src/transport.rs (`send_with_retry`)
- apps/editor/src-tauri/src/ai.rs (`blocking`)

## メモ

- T0019: T0551 の起票の候補「自動リギングの途中の取り消し (必要が出たら)」はこのタスクの取り消しの範囲に入る。取り消しの印は、MCP の
  `notifications/cancelled` からも渡せる形にしておく (`ugoe-mcp` から渡すのは `crates:` の外なので、要るなら起票の候補に。長い `render_png` の取り消しは見送り)。

### やったこと

- **取り消しの手段 (ugoe-ai):** `ugoe_ai::CancelToken` (`src/cancel.rs`。複製は同じ印を共有、`cancel()`・`check()`・`with_time_limit(limit)`)。プロバイダ (`ClaudeProvider`・`OpenAiProvider`) と `HttpDepthEstimator` に `with_cancel(token)` で渡す。`Provider`・`DepthEstimator` トレイトの形は変えない。
  - `transport::send_with_retry` が送る前・応答を待つ間・再試行の前の待ちの間に確かめる。応答は別のスレッドで待ち (50 ms ごとに印を見る)、取り消したら応答を待たずに返る (例に挙げられた「通信中は Transport の上限で抑える」より一歩進めた。通信そのものはそのスレッドで上限まで続いて捨てられる)。待ちは差し替えた `sleep` がなければ取り消しで起きる (Condvar)。
  - 各リクエストの上限を時間の上限の残りまでに縮める (`HttpRequest::timeout` を足し、`UreqTransport` が自分の上限と短い方を使う)。待ち時間が残りを超えるなら待たずに諦める。上限で切れた通信は `Network` ではなく上限のエラーにする。
  - 問い直し (OpenAI 互換の `json_retries`・リギングの `ask_until_valid`・分類の分けたリクエスト) は次に送る前に確かめるので止まる。リギングは `InvalidProposal` だけを問い直すので、取り消しは問い直さない。
- **エラー:** `AiError::Cancelled`・`AiError::TimeLimitExceeded { limit }`。エディタのコード `ai.cancelled`・`ai.time_limit` (引数 `minutes`)・`ai.invalid_time_limit` (`min`・`max`) と ja/en の文言、`docs/editor.md` のコードの表。
- **時間の上限の設定:** エディタの `ProviderSettings::time_limit_minutes` (既定 15 分、1〜120 分。`ai-provider.json`、前の版のファイルは既定で読む)。設定の画面の「AI provider」に入力欄。
- **エディタ:** `prepare_requests` が上限付きの印を作り、実行中の一覧 (`AppState::ai_jobs`・`AiJobs`) に載せた `AiJob` を `Prepared::Ready` で返す。結果は `AiJob::finish` を通し、取り消した後に届いた結果は捨てる (提案の一覧に置かない)。IPC `cancel_ai_requests` (acl・capabilities・`ipc.ts`) が実行中をすべて取り消す。画面は AI の提案のパネルの進み具合の隣と、設定の「接続を確かめる」の隣に「取り消す」(既存の `TipButton`・`Button` と進み具合の行の配置。新しい見た目の部品は足していない)。取り消したらエラーではなく「取り消しました」。
- **ADR:** 書かなかった。公開 API に足すだけ (`CancelToken`・`with_cancel`・`AiError` の 2 つ・`HttpRequest::timeout`) で、ADR 0006 (提案だけ)・ADR 0022 (プロバイダの抽象) の判断を変えないため。理由は `docs/ai-integration.md#取り消しと時間の上限` に書いた。
- **テスト:** `crates/ugoe-ai/tests/cancel.rs` (取り消し済みなら送らない・再試行の待ちの間の取り消し・上限 0・待ちが残りを超えたら待たない・リクエストの上限を残りに縮める・通信の途中の取り消し (応答を返さない `Transport`)・問い直しの間の取り消し・深度推定)、`src/cancel.rs` の単体テスト、エディタの `ai_tests.rs` (一覧・取り消し・`finish`・設定の範囲)、`proposalsView.test.ts` (取り消しは知らせでエラーでない・AI を使わない操作は取り消せない)。

### やらなかったこと

- `FakeProvider` は印を持たない (テスト用)。MCP サーバ (ugoe-mcp) は `Provider` を呼ばないので触っていない。
- 取り消しは「実行中のすべて」で、送信ごとには選べない (画面で同時に走る AI の操作は 1 つずつなので足りる)。
- 実際の API・遅いサーバでの取り消しの体感は確かめていない (テストはネットワークを使わない)。エディタの画面は svelte-check と Vitest で確かめ、起動しての目視はしていない。

### 起票の候補

- CLI (`ugoe-cli`) の AI のコマンドに Ctrl+C での取り消しと時間の上限のオプション (今は上限なし・取り消しなし)。
- `send_cancellable` は応答を待つたびにスレッドを 1 つ作る。ureq に中断の手段ができたら、通信そのものを止める形に替える。

### just ci

`git rebase main` (T0689 などのマージの後。衝突なし) の後に `just ci` を前面で流して通った。
