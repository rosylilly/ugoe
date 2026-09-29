---
id: T0509
title: AI: Claude API のプロバイダ
status: done
milestone: M6
crates: [ugoe-ai, docs, Cargo.toml, scripts]
depends_on: []
assignee: claude-worker-T0509
---

# T0509: AI: Claude API のプロバイダ

## 目的

`ugoe_ai::Provider` の既定の実装として、Anthropic の Claude API (画像入力を含む) を呼ぶプロバイダを作る。レイヤーの意味分類 (T0503)・自動リギング (T0504) などが、実際のモデルで動くようにする。

## 受け入れ条件

- [x] `Provider` の実装: 画像 (レイヤーの画像・合成画像) とテキストを送り、構造化された出力 (JSON Schema に沿った JSON) を受け取る。既定のモデルは最新の Claude (`claude-opus-5-5`)。モデル名は設定で変えられる
- [x] API キーは環境変数か OS の資格情報の保存先から読み、ログやエラーに出さない。送る内容 (画像を外部に送ること) を `docs/ai-integration.md#セキュリティとプライバシー` に沿って利用者に示す
- [x] 応答の録画と再生 (テストはネットワークなしで動く)。タイムアウト・再試行・レート制限の扱い
- [x] 依存 (HTTP クライアントなど) のライセンスが MIT のクレートに入れてよいものであること (`just licenses`)。ツール層だけに置き、ランタイム層に入れない (`just layers`)
- [x] `just ci` が通る

## 参考

- `docs/ai-integration.md`
- `docs/adr/0006-ai-proposals.md`
- https://docs.claude.com/

## メモ

- やったこと
  - `ugoe_ai::claude::ClaudeProvider`: Messages API に PNG 画像 (base64) とプロンプトを送り、構造化出力 (`output_config.format` の `json_schema`) で JSON を受け取る。設定は `ClaudeConfig` (モデル・`max_tokens`・`effort`・URL・タイムアウト・再試行)。`refusal` は `AiError::Refused`、`max_tokens` は `AiError::Truncated`
  - 再試行: 408・409・429・5xx・タイムアウト・接続の失敗を指数バックオフで 2 回まで。`retry-after` に従い、長すぎれば `AiError::RateLimited` で諦める
  - API キー: `ApiKey` (`Debug` は伏せ字)。`ApiKey::from_env` (`ANTHROPIC_API_KEY`) か、ホストがキーチェーンから読んだ値を `ApiKey::new` に渡す。エラーの文言からもキーを取り除く
  - 送る内容の提示: `Provider::disclosure` (送り先・モデル・画像の枚数とバイト数)。既定の実装は `Destination::Local`
  - 録画と再生: `transport` モジュール (`Transport`・`UreqTransport`・`ReplayTransport`・`RecordingTransport`・`Cassette`)。録画はヘッダ (キー) を残さず、画像のデータを長さだけにする。録画は `crates/ugoe-ai/tests/cassettes/` に Messages API の応答の形で手で作った
  - `FakeProvider` (応答を積んで順に返し、呼び出しを記録する)。T0503・T0504 のテスト用
  - `AiError` に variant を追加 (`MissingApiKey`・`InvalidRequest`・`Api`・`RateLimited`・`Network`・`Refused`・`Truncated`・`InvalidResponse`)
- 範囲外の変更 (team-lead の承認済み。`crates:` に追加した)
  - ルートの `Cargo.toml` に `base64` と `ureq` を追加。`Provider` が同期なので、非同期ランタイムを持ち込まない同期の `ureq` にした。TLS は rustls + ring で、OpenSSL (システムのライブラリ) は不要
  - `scripts/check-licenses.mjs` の PERMISSIVE に `CDLA-Permissive-2.0` を追加。`ureq` の rustls が使う `webpki-roots` (Mozilla のルート証明書) のライセンスで、コピーレフトのないデータ用ライセンス (条件は再配布時に条文を示すことだけ)
- やらなかったこと・後続の候補
  - OS のキーチェーンからの読み出しと、送る前の確認のダイアログ・プロジェクト単位の「送信しない」設定は、エディタ側のタスクで行う (`disclosure` を使う)
  - 実際の API で録り直した録画はない (キーがない)。`tests/claude_replay.rs` の `record_live` (`--ignored`) で録れる
  - ストリーミング・プロンプトキャッシュ・使用量 (`usage`) の報告は未対応。大きな `max_tokens` (16000 超) が要るならストリーミングを入れる
  - 構造化出力はスキーマに制約がある (`additionalProperties: false` が必須、数値・文字列長の制約は不可)。T0503 以降でスキーマを作るときに注意
