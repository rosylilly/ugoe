---
id: T0509
title: AI: Claude API のプロバイダ
status: todo
milestone: M4
crates: [ugoe-ai, docs]
depends_on: []
assignee:
---

# T0509: AI: Claude API のプロバイダ

## 目的

`ugoe_ai::Provider` の既定の実装として、Anthropic の Claude API (画像入力を含む) を呼ぶプロバイダを作る。レイヤーの意味分類 (T0503)・自動リギング (T0504) などが、実際のモデルで動くようにする。

## 受け入れ条件

- [ ] `Provider` の実装: 画像 (レイヤーの画像・合成画像) とテキストを送り、構造化された出力 (JSON Schema に沿った JSON) を受け取る。既定のモデルは最新の Claude (`claude-opus-5-5`)。モデル名は設定で変えられる
- [ ] API キーは環境変数か OS の資格情報の保存先から読み、ログやエラーに出さない。送る内容 (画像を外部に送ること) を `docs/ai-integration.md#セキュリティとプライバシー` に沿って利用者に示す
- [ ] 応答の録画と再生 (テストはネットワークなしで動く)。タイムアウト・再試行・レート制限の扱い
- [ ] 依存 (HTTP クライアントなど) のライセンスが MIT のクレートに入れてよいものであること (`just licenses`)。ツール層だけに置き、ランタイム層に入れない (`just layers`)
- [ ] `just ci` が通る

## 参考

- `docs/ai-integration.md`
- `docs/adr/0006-ai-proposals.md`
- https://docs.claude.com/

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
