---
id: T0512
title: AI: OpenAI 互換のプロバイダと、エディタでのプロバイダの選択
status: in-progress
milestone: M6
crates: [ugoe-ai, apps/editor, docs]
depends_on: [T0509, T0638]
assignee: claude-worker-T0512
---

# T0512: AI: OpenAI 互換のプロバイダと、エディタでのプロバイダの選択

## 目的

ADR 0022 の「エディタの中で API を呼ぶ経路」を広げる。OpenAI 互換の API (OpenAI・OpenRouter・Gemini の互換の窓口・手元の Ollama / LM Studio / vLLM など) を呼ぶ `Provider` を足し、エディタでプロバイダとモデルを選べるようにする。

## 受け入れ条件

- [ ] `ugoe_ai` に OpenAI 互換のプロバイダ: 画像 (PNG) とテキストを送り、構造化出力 (`response_format` の JSON Schema) で JSON を受け取る。構造化出力に対応しないサーバでは、JSON の検証と再試行に頼る設定がある。ベースの URL・モデル・API キー (省略可) を設定できる
- [ ] 送り先がループバックなら `Destination::Local` を返し、それ以外は送り先とモデルを `Disclosure` で示す。API キーをログやエラーに出さない
- [ ] 録画した応答 (実際の API の応答の形に合わせる) でのテスト。タイムアウト・再試行・レート制限の扱い。新しい外部の依存を入れない (既存の HTTP クライアントを使う)
- [ ] エディタの設定: プロバイダ (Claude / OpenAI 互換) とモデル・ベースの URL を選べる。API キーはプロバイダごとに OS の資格情報の保存先に置く。送信の同意 (T0638) はプロバイダに関わらず同じ流れを通る。既存の設定の既定 (Claude) は変えない
- [ ] 分類・リギング・法線マップの提案が、選んだプロバイダで動く (録画で確かめる)。ヘッドレスのブラウザで設定の画面を確かめる
- [ ] `docs/ai-integration.md` と `docs/editor.md` を更新し、`just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0022-multiple-ai-providers-and-agents.md`
- `docs/tasks/T0509-claude-provider.md`・`docs/tasks/T0638-editor-ai-consent.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
