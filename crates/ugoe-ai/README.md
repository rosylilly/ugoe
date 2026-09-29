# ugoe-ai

**層:** T3 自動化 (依存してよいクレートは `scripts/check-layers.mjs` を参照)

AI プロバイダの抽象 (`Provider`) と、自動リギングのパイプライン。

## 守ること

- モデルを直接変更せず、`Proposal` (Command の列) を返す
- CI で実際の API を呼ばない (`FakeProvider` か、録画を再生する `transport::ReplayTransport` を使う)
- API キーをログ・エラー・録画に出さない

## 構成

- `lib.rs`: `Provider` トレイト、`ProviderRequest`、`Proposal`、`Disclosure` (外部に送る内容の要約)
- `claude.rs`: Claude API のプロバイダ (`ClaudeProvider`・`ClaudeConfig`・`ApiKey`)
- `transport.rs`: HTTP の差し替え (`UreqTransport`・`ReplayTransport`・`RecordingTransport`・`Cassette`)
- `classify.rs`: レイヤーの意味分類 (`Label`・`Side`・`LayerLabel`、名前の規則による `classify_by_name`、`Provider` による `classify_layers`)。`classify/rules.rs` が名前の規則、`classify/image.rs` が送る画像
- `fake.rs`: テスト用の `FakeProvider`
- `depth.rs`: 深度推定 (`DepthEstimator`・`DepthMap`・`HttpDepthEstimator`・`FixedDepthEstimator`)
- `turn.rs`: 深度推定による振り向きリグの提案 (`propose_depth_turn`・`plan_turn`・`label_depth`・`mesh_labels`)
- `rig.rs`: ラベルのテンプレートによる自動リギングの提案 (`propose_rig`)、テンプレートで決められない値 (`RigParams`) を AI に決めさせ、検証エラーを添えて問い直す `propose_rig_with_ai`、分類の結果をメッシュに対応付ける `mesh_parts`
- `examples/auto_rig.rs`: PSD を取り込み、名前の規則で分類してテンプレートの提案を適用したバンドルを書き出す (`ugoe render` で目視確認する)
- `examples/depth_turn.rs`: 録画した深度マップから提案を作り、適用したモデルを書き出す (`ugoe render` で目視確認する)

## 仕様

- [docs/ai-integration.md](../../docs/ai-integration.md)
- [docs/adr/0006-ai-proposals.md](../../docs/adr/0006-ai-proposals.md)

## 関連タスク

- T0503 (`docs/tasks/T0503-*.md`)
- T0504 (`docs/tasks/T0504-*.md`)
- T0505 (`docs/tasks/T0505-depth-turn-rig.md`)
- T0509 (`docs/tasks/T0509-claude-provider.md`)
