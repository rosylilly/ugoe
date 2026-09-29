---
id: T0512
title: AI: OpenAI 互換のプロバイダと、エディタでのプロバイダの選択
status: done
milestone: M6
crates: [ugoe-ai, apps/editor, docs]
depends_on: [T0509, T0638]
assignee: claude-worker-T0512
---

# T0512: AI: OpenAI 互換のプロバイダと、エディタでのプロバイダの選択

## 目的

ADR 0022 の「エディタの中で API を呼ぶ経路」を広げる。OpenAI 互換の API (OpenAI・OpenRouter・Gemini の互換の窓口・手元の Ollama / LM Studio / vLLM など) を呼ぶ `Provider` を足し、エディタでプロバイダとモデルを選べるようにする。

## 受け入れ条件

- [x] `ugoe_ai` に OpenAI 互換のプロバイダ: 画像 (PNG) とテキストを送り、構造化出力 (`response_format` の JSON Schema) で JSON を受け取る。構造化出力に対応しないサーバでは、JSON の検証と再試行に頼る設定がある。ベースの URL・モデル・API キー (省略可) を設定できる
- [x] 送り先がループバックなら `Destination::Local` を返し、それ以外は送り先とモデルを `Disclosure` で示す。API キーをログやエラーに出さない
- [x] 録画した応答 (実際の API の応答の形に合わせる) でのテスト。タイムアウト・再試行・レート制限の扱い。新しい外部の依存を入れない (既存の HTTP クライアントを使う)
- [x] エディタの設定: プロバイダ (Claude / OpenAI 互換) とモデル・ベースの URL を選べる。API キーはプロバイダごとに OS の資格情報の保存先に置く。送信の同意 (T0638) はプロバイダに関わらず同じ流れを通る。既存の設定の既定 (Claude) は変えない
- [x] 分類・リギング・法線マップの提案が、選んだプロバイダで動く (録画で確かめる)。ヘッドレスのブラウザで設定の画面を確かめる
- [x] `docs/ai-integration.md` と `docs/editor.md` を更新し、`just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0022-multiple-ai-providers-and-agents.md`
- `docs/tasks/T0509-claude-provider.md`・`docs/tasks/T0638-editor-ai-consent.md` のメモ

## メモ

- やったこと
  - `ugoe_ai::openai::OpenAiProvider` (Chat Completions)。画像は data URL の `image_url`、システムプロンプトは `system`。
    答えの形は `StructuredOutput` で選ぶ: `JsonSchema` (既定。`response_format` の `json_schema` + `strict: true`)・`JsonObject`・
    `Prompt`。どれでも受け取った JSON をスキーマ (ugoe-ai が使う範囲のキーワード) で検証し、通らなければ前の答えと理由を添えて
    問い直す (`json_retries`。既定は構造化出力で 0 回、それ以外で 2 回)。`<think>`・コードフェンス・前置きは除いて読む。
    `refusal`・`finish_reason` の `length`/`content_filter` はエラーに。トークン数の上限は既定で送らない (`max_completion_tokens` か
    `max_tokens` を選べる)。キーは省略できる (`Authorization: Bearer`)。
  - 再試行の規則 (`RetryPolicy`・`send_with_retry`) を `transport.rs` に移して Claude と共有 (`ugoe_ai::claude::RetryPolicy` は再公開で残る。
    Claude の振る舞いは変えていない)。OpenAI 互換では 429 でも残高・利用上限の不足 (`insufficient_quota` など) は再試行しない。
    ループバックの判定 (`is_loopback_url`) も `transport.rs` に移して深度推定器と共有 (`127.example.com`・`user@host` などを正しく扱うよう直した)。
    録画は data URL の画像も長さだけにし、`x-request-id` を残す。
  - 録画 `tests/cassettes/openai_*.json` (公式の文書の応答の形に合わせて手で作った。`openai_ollama_prompt_retry.json` は Ollama の形) と
    `tests/openai_replay.rs`。分類とリギングは Claude の録画と同じ答えを OpenAI の形にしたもので同じ結果になることを確かめた
    (`tests/classify.rs`・`tests/auto_rig.rs`・エディタの `proposals_tests.rs`)。
  - エディタ: `src-tauri/src/ai_provider.rs` (`ProviderSettings`・`ProviderStore`。`app_config_dir` の `ai-provider.json`。既定は Claude)、
    `ai.rs` の `prepare_claude(_requests)` を `prepare_requests` (プロバイダに関わらず同じ同意の流れ) に一般化、IPC `set_ai_provider` を追加。
    キーの置き場はプロバイダごと (`KeySlot`。Claude は前からの `anthropic-api-key`、OpenAI 互換は `openai-compatible:<host>`)、
    `OPENAI_API_KEY` は送り先が api.openai.com のときだけ使う。平文の http で外部にキーを送らない。
    ループバックは `Destination::Local` なので同意を求めず、「外部に送らない」のプロジェクトでも使える。
    送り直す回数の表示には、プロバイダの問い直しを含める (`(r + 1)(j + 1) − 1`)。
  - 画面: 設定の画面にプロバイダ・送り先の選択肢 (OpenAI・OpenRouter・Gemini・Ollama・LM Studio)・ベースの URL・モデル・JSON の出力の形、
    選んだプロバイダのキー。確認の画面の規約のリンクを送り先ごとに (`termsFor`。知らない送り先は促す文だけ)。提案のパネルは
    「Use <プロバイダ>」と、使えない理由 (`aiBlockedReason`)。
  - 法線マップの提案は `Provider` を使わない (決定的な方法と深度推定器) ので、プロバイダの選択に関わらない (受け入れ条件の
    「法線マップ」は、プロバイダに依存しないことを確かめた扱い)。
  - 文書: `docs/ai-integration.md#openai-互換の-api` (出典の URL 付き)・`docs/editor.md#ai-の設定と送信の同意`・`docs/manual/ai.md` (画像 1 枚)・ugoe-ai の README。
- 調べたこと (出典は `docs/ai-integration.md` に書いた)
  - OpenAI: `max_tokens` は非推奨で `max_completion_tokens` (推論モデルは後者だけ)。`strict` はすべてのプロパティが `required`・
    `additionalProperties: false` が必要 (ugoe-ai のスキーマは満たす)。429 には残高不足などの再試行しても直らないものがある。
  - Ollama は `max_tokens`・`response_format`・base64 の画像に対応 (URL の画像は不可)。LM Studio は `json_schema` に対応。
    OpenRouter はモデルによって構造化出力を強制しない。Gemini の互換の窓口は `.../v1beta/openai`。
- 確かめたこと
  - `just ci` が通る (main に rebase した後に `just wasm` → `just gen` → `just ci`)。`cargo check -p ugoe-editor` は ci のテストのビルドに含まれる。
  - ヘッドレス Chrome (ビルドした画面 + scratchpad の使い捨ての HTTP ブリッジ。本物の `Document`・`Proposals`・`ugoe_ai` を使い、
    AI は `openai_*` の録画を選んだ送り先の URL に書き換えて `ReplayTransport` で再生。設定・同意の規則は `ai.rs` を真似た):
    既定は Claude → OpenAI 互換・OpenRouter に切り替えて Apply → キーを保存 (ページの HTML にキーがない) → Test connection で
    OpenRouter の確認 (規約のリンク付き。同意の前は 0 回、後に 1 回送信) → New from PSD → 「Use OpenRouter」で Propose rig
    (確認 2 回・送信 3 回) → Ollama (プロンプトだけ) + 「外部に送らない」でも Test connection が使え、分類・提案は確認なしで送る
    (送信 3 回・提案が出る) → Claude に戻すと「外部に送らない」で「Use Claude」が無効。
- 確かめていないこと: 実際の API (OpenAI・OpenRouter・Gemini) と手元の Ollama / LM Studio への接続 (キーもサーバもない。
  `tests/openai_replay.rs` の `record_live` で録り直せる)、実際の Tauri のウィンドウ (`app_config_dir` への保存)、実際のキーチェーン。
  既定のモデル名 (`gpt-6-sol`・`gemini-3.8-flash`) は公式の文書の今の名前から選んだ。
- 範囲外の変更: なし (ugoe-ai・apps/editor・docs だけ)。ugoe-ai の `lib.rs` は `pub mod openai;` と doc の 1 行だけ。
- 後続の候補
  - エディタで `max_tokens`・考える量 (`reasoning_effort` / Claude の `effort`)・タイムアウトを選べるように。
  - 手元の推論サーバのモデル一覧 (`GET /v1/models`) から選ぶ。
  - 分類の結果がどのプロバイダ・モデルによるかを記録して画面に出す (今は「by AI」だけ)。
  - OpenAI の Responses API (Chat Completions より新しい窓口) への対応。
