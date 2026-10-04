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
- `openai.rs`: OpenAI 互換の API (Chat Completions) のプロバイダ (`OpenAiProvider`・`OpenAiConfig`・`StructuredOutput`)。OpenAI・OpenRouter・Gemini・Ollama・LM Studio・vLLM など
- `transport.rs`: HTTP の差し替え (`UreqTransport`・`ReplayTransport`・`RecordingTransport`・`Cassette`)、プロバイダで共通の再試行 (`RetryPolicy`)、ループバックの判定 (`is_loopback_url`)
- `classify.rs`: レイヤーの意味分類 (`Label`・`Side`・`LayerLabel`、名前の規則による `classify_by_name`、`Provider` による `classify_layers`)。`classify/rules.rs` が名前の規則、`classify/image.rs` が送る画像
- `fake.rs`: テスト用の `FakeProvider`
- `cancel.rs`: 提案の取り消しと時間の合計の上限 (`CancelToken`)。プロバイダと深度推定器に `with_cancel` で渡し、`transport::send_with_retry` が送る前・応答を待つ間・再試行の待ちの間に確かめる (`docs/ai-integration.md#取り消しと時間の上限`)
- `depth.rs`: 深度推定 (`DepthEstimator`・`DepthMap`・`HttpDepthEstimator`・`FixedDepthEstimator`)。HTTP の推論サーバへはプロバイダと同じ規則で送る (再試行・任意の `Authorization: Bearer`)
- `turn.rs`: 深度推定による振り向きリグの提案 (`propose_depth_turn`・`plan_turn`・`label_depth`・`mesh_labels`)
- `instructions.rs`: 利用者の注文 (`Instructions`) をプロンプトに区切って入れる書き方と、反映できない注文 (`Not applied:`) の取り出し (`split_unapplied`)。分類・リグの `*_guided` が使う (`docs/ai-integration.md#利用者の注文`)
- `grid.rs`: キーフォームの格子の補助 (キーを誤差 1e-5 で比べる `has_key`、セルと線形インデックスの変換 `cell_index`・`cell_at`)。`rig.rs`・`module_rig.rs`・`turn.rs` が使う
- `rig.rs`: ラベルのテンプレートによる自動リギングの提案 (`propose_rig`)、テンプレートで決められない値 (`RigParams`) を AI に決めさせ、検証エラーを添えて問い直す `propose_rig_with_ai`、分類の結果をメッシュに対応付ける `mesh_parts`
- `module_rig.rs`: 着せ替えのモジュールのプロジェクト向けの自動リギング (`propose_module_rig`・`propose_module_rig_with_ai_guided`)。パラメータを足さず、衣装を本体のワープに追従させ (`Follow…` のワープのキーフォームを本体の変形から推定するか、組み方 `FollowMode::Mesh` で衣装のメッシュに直接写す)、本体のメッシュのキーフォーム (目の開閉など) と、従うワープより下の本体のデフォーマの動きにも、ワープの動きと合わせて従わせ (`module_rig/mesh.rs`)、揺れるラベルの衣装に振り子の物理を付けてスロットが公開する本体の衝突体に当て、衣装の差分 (名前の末尾の A / B、AI の分類が同じ部品の別の絵とした組 (`LayerLabel::variant`)、または絵が重なり意味ラベルが同じで PSD で片方を隠したもの) をポーズにし、本体のポーズの要素に追従させる (名前の文字か、見えている / 隠してあるで。`module_rig/pose.rs`)。組み方は `ModuleRigOptions` (`docs/ai-integration.md#モジュールの自動リギング`)。ugoe-mcp の `get_module_rig_guide`・`propose_module_rig` も同じ関数を呼ぶ (値はエージェントが選ぶ)
- `normal.rs`: 法線マップの生成の提案。アルファの形から (`propose_bulge_normal_maps`。意味ラベルから部位のまとまり `SurfaceGroup` を決め、まとまりごとに 1 つの形としてふくらませる。決定的な計算は `ugoe_rigging::group_bulge_normal_maps`。設定は `BulgeNormalMapOptions`、1 枚のまとまりと既定の設定は `normal_map_group`、すべてのまとまりの一覧は `normal_map_groups`。眼鏡は顔のまとまりの向きだけを受ける `SurfaceGroup::orientation_only`) と、深度推定から (`propose_depth_normal_maps`・`plan_depth_normal_maps`。テクスチャごとの設定は `NormalMapOptions::per_texture`)。メッシュの名前だけからのラベルは `classify::label_meshes_by_name`
- `examples/auto_rig.rs`: PSD を取り込み、名前の規則で分類してテンプレートの提案を適用したバンドルを書き出す (`ugoe render` で目視確認する)
- `examples/depth_turn.rs`: 録画した深度マップから提案を作り、適用したモデルを書き出す (`ugoe render` で目視確認する)

## テスト

- `cargo test -p ugoe-ai`: ネットワークと GPU を使わないテスト
- feature `gpu-tests` (`just test-gpu` に含まれる): 生成した法線マップを参照レンダラ (`ugoe-render`) で描く (`tests/normal_render.rs`)。
  `UGOE_NORMAL_RENDER_DUMP=<dir>` を付けると描いた画像を PNG で書き出す (目視確認用)

## 仕様

- [docs/ai-integration.md](../../docs/ai-integration.md)
- [docs/adr/0006-ai-proposals.md](../../docs/adr/0006-ai-proposals.md)
- [docs/adr/0022-multiple-ai-providers-and-agents.md](../../docs/adr/0022-multiple-ai-providers-and-agents.md)

## 関連タスク

- T0503 (`docs/tasks/T0503-*.md`)
- T0504 (`docs/tasks/T0504-*.md`)
- T0505 (`docs/tasks/T0505-depth-turn-rig.md`)
- T0509 (`docs/tasks/T0509-claude-provider.md`)
- T0506 (`docs/tasks/T0506-normal-map-gen.md`)
- T0517 (`docs/tasks/T0517-ai-module-proposals.md`)
- T0519 (`docs/tasks/T0519-ai-module-rig-followups.md`)
- T0541 (`docs/tasks/T0541-ai-request-cancel.md`)
