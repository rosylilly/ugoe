# AI 連携設計

コードは `crates/ugoe-psd`、`crates/ugoe-mesher`、`crates/ugoe-ai`。判断の経緯は [ADR 0006](adr/0006-ai-proposals.md)。

## 目標

PSD を 1 枚渡すと、「顔の角度・目・口・呼吸・髪揺れが一通り動く」モデルが数分でできること。人間は、それをエディタで直すことに集中できるようにします。

## 原則

1. **AI は提案するだけです。** AI は `Proposal { title, rationale, commands: Vec<Command> }` を返します。エディタはそれを差分としてプレビューし、ユーザーが採用して初めて `Document::apply` で適用します。適用は `Batch` として 1 回の Undo で取り消せます。
2. **AI でなくてもできることは、決定的なアルゴリズムで行います。** メッシュ生成は `ugoe-mesher` の幾何アルゴリズムで行います。AI が決めるのは「どのレイヤーが何か」「どのパラメータでどう動かすか」といった意味の判断と、アルゴリズムのパラメータ選びだけです。
3. **プロバイダを差し替えられるようにします。** LLM や画像モデルの呼び出しは `ugoe_ai::Provider` トレイトの裏に隠します。既定のプロバイダは Anthropic の Claude API ですが、ローカルモデルも使えるようにします。
4. **ネットワークなしでテストできるようにします。** テストでは、録画した応答を返す `FakeProvider` を使います。CI では実際の API を呼びません。
5. **外部のエージェントからも操作できるようにします。** 将来、`ugoe-mcp` (MCP サーバ) で `Command` の JSON Schema をツールとして公開し、Claude Code などの外部エージェントがエディタを操作できるようにします。
   - `Command` の JSON Schema は Rust から生成してリポジトリに置いてあります (`crates/ugoe-editor-core/schema/command.schema.json`、[型の生成](editor.md#型の生成))。`ugoe-ai` などからは `ugoe_editor_core::codegen::COMMAND_SCHEMA_JSON` (ファイルの中身) か `command_schema()` (その場で生成) で参照します。

## パイプライン

```
PSD
 │ ugoe-psd::import                       (決定的)
 ▼
LayerTree (レイヤー名・グループ・画像・位置)
 │ ugoe-ai::classify_layers               (AI: 画像 + レイヤー名 → 意味ラベル)
 ▼
意味ラベル付きのレイヤー (face, eye_white_l, iris_l, eyelash_upper_l, mouth, hair_front, ...)
 │ ugoe-mesher::generate                   (決定的。オプションはラベルごとのプリセット)
 ▼
メッシュ (Command::AddMesh ...)
 │ ugoe-ai::propose_rig                    (AI + テンプレート)
 │   - 標準パラメータの作成
 │   - デフォーマ構造のテンプレート適用 (顔 → 回転 + ワープ、目 → ワープ …)
 │   - キーフォームの生成 (目の開閉 = まぶたメッシュの頂点を閉じた形へ …)
 │   - 物理プリセット (髪の前・横・後ろ。ADR 0011 の動的物理を使う)
 │   - 深度推定による振り向きリグ、ミラーと加算合成によるキーフォームの推定 (ADR 0010)
 ▼
Proposal (Command 列) ──► ユーザーの確認 ──► Document::apply
```

### PSD 取り込み

- 出力は PSD の形式に依存しない `LayerTree` です。後段は PSD の詳細を知りません。
- 対応するもの: ラスタレイヤー、グループ、表示/非表示、不透明度、ブレンドモードの名前 (記録するだけ)、レイヤーマスク (アルファに掛ける)、8/16 ビットの RGB (16 ビットは 8 ビットに丸める)。
- 対応しないもの (ラスタライズ済みの画像が PSD にない場合は警告を出す): 調整レイヤー、スマートオブジェクト、レイヤー効果。
- PSD のパースに外部クレートを使うか自前で書くかは T0501 で決め、その判断は `ugoe-psd` の中に閉じ込めます。

### 自動メッシュ

`ugoe_mesher::generate(rgba, width, height, &MeshOptions)`:

1. アルファを `alpha_threshold` で二値化する
2. 輪郭を抽出する (marching squares)。穴がある場合は穴の輪郭も抽出する
3. 輪郭を `margin` だけ外側に広げ、`edge_spacing` の間隔で頂点を並べ直す (Douglas–Peucker で簡略化してから再サンプリングする)
4. 内部に `interior_spacing` の間隔で頂点を置く (ポアソンディスクサンプリングまたは格子)
5. 制約付きドロネー三角形分割 (候補: `spade` クレート) で三角形を作る
6. 頂点数が `u16` の範囲を超えないように、パラメータを自動で調整する

ラベルごとのプリセット (例: 目は細かく、胴体は粗く) は `ugoe-ai` 側が持ちます。

### レイヤーの意味分類

- 入力: レイヤーのサムネイル (個別の画像と、全体の中での位置を示す画像)、レイヤー名、グループの構造
- 出力: JSON Schema で制約した、レイヤーごとの `{ layer_path, label, side: left|right|center, confidence }`
- ラベルの語彙は `ugoe-ai` に定数として定義し、標準パラメータとテンプレートに対応させます。
- `confidence` が低いものは、エディタで人間に確認を求めます。

### 自動リギング

- テンプレート (Rust で書くか、宣言的なデータで書く) が、ラベルの組み合わせから、デフォーマの構造とキーフォームの作り方を決めます。
- AI は、テンプレートでは決められない値 (目の閉じ方のカーブ、顔の向きによる変形量、パーツの前後関係) を画像から推定します。
- 生成したキーフォームは必ず `ugoe_model::validate` を通します。検証に失敗した提案は捨てて、エラーを AI にフィードバックして再試行します (上限回数あり)。

### 深度推定による振り向きリグ

判断の経緯は [ADR 0010](adr/0010-keyform-inference.md)、推定の式は [spec/keyform-inference.md#深度からの回転](spec/keyform-inference.md#深度からの回転)。

1. 原画 (PSD を合成した画像、またはレイヤーごとの画像) から、単眼の深度推定モデルで深度マップを作る (`Provider` の裏に隠す)。
2. 顔・体のワープデフォーマの制御点 (またはメッシュの頂点) で深度マップを標本化し、`depth` 属性の提案を作る。レイヤーの意味ラベル (前髪は顔より手前、など) で補正する。
3. `InferKeyforms { method: depth_rotation }` のコマンド列を提案する。回転の中心と `angle_scale` もラベルから推定する。
4. ユーザーは提案をプレビューで確認し、採用後に必要な箇所を手で直す (手で直したセルは `authored` になる)。

深度の推定には AI を使いますが、回転と投影は決定的なアルゴリズム (`ugoe-rigging`) で行います。

### 法線マップの生成

判断の経緯は [ADR 0015](adr/0015-lighting.md)。

- **決定的な方法 (既定)**: アルファの形から、輪郭からの距離に応じてふくらんだ高さを作り (距離変換 + なめらかな関数)、その勾配を法線にする。パーツごとに「ふくらみ」の強さを調整できる。
- **AI を使う方法**: 深度推定の結果 (上の節と共通) から法線を作る。
- どちらも `ugoe-rigging` (決定的な方法) と `ugoe-ai` (AI を使う方法) に置き、結果は提案としてエディタに出す。

## Claude API のプロバイダ

既定のプロバイダは `ugoe_ai::claude::ClaudeProvider` で、Anthropic の Messages API (`POST /v1/messages`) を呼びます。

- **入力と出力:** `ProviderRequest` の画像 (PNG) を base64 の画像ブロックとして、プロンプトをテキストのブロックとして 1 つのユーザーメッセージに入れて送ります。出力は構造化出力 (`output_config.format` に `{"type": "json_schema", "schema": ...}`) で、呼び出し側の JSON Schema に沿った JSON を受け取ります。スキーマのオブジェクトには `additionalProperties: false` を書きます (構造化出力の制約。`minimum` などの数値の制約や再帰するスキーマは使えません)。
- **設定 (`ClaudeConfig`):** モデル (既定は `claude-opus-5-5`)、`max_tokens` (既定は 16000。考える分を含む)、考える量 `effort` (既定は API の既定)、API の URL、1 回のリクエストの時間の上限 (既定は 600 秒)、再試行の方針。
- **応答の扱い:** テキストのブロックだけをつないで JSON として読みます (`thinking` のブロックは読み飛ばします)。`stop_reason` が `refusal` なら `AiError::Refused`、`max_tokens` なら `AiError::Truncated` を返します。
- **再試行:** HTTP 408・409・429・5xx (529 overloaded を含む)・接続の失敗・タイムアウトは、指数バックオフ (0.5 秒から倍々、上限 8 秒) で 2 回まで再試行します。`retry-after` ヘッダがあればその秒数を待ち、長すぎる (既定で 60 秒を超える) ときは待たずに `AiError::RateLimited` を返します。400・401・403・404 などは再試行しません。
- **HTTP クライアント:** 同期の `ureq` (TLS は rustls) を使い、`Transport` トレイトの裏に隠します。

### 録画と再生

テストはネットワークを使わず、録画 (`Cassette`、JSON ファイル) を `ReplayTransport` で再生します。録画は `crates/ugoe-ai/tests/cassettes/` にあります。

- 録画は往復の列で、リクエストは method・URL・本文 (JSON)、応答は状態コード・一部のヘッダ (`content-type`・`request-id`・`retry-after`)・本文を持ちます。応答の代わりに通信の失敗 (`timeout`・`connect`) も書けます。
- 録画のリクエストに本文が書いてあれば、再生のときに送った本文と一致するかを確かめます。
- `RecordingTransport` で実際の通信を包むと録画を作れます (`tests/claude_replay.rs` の `record_live`。既定では動かしません)。

## セキュリティとプライバシー

- API キーは OS のキーチェーンに保存します (Tauri の stronghold または keyring 系のプラグイン)。プロジェクトファイルには書き込みません。
  - `ugoe-ai` はキーチェーンを直接読みません。ホスト (エディタ) が読んだ値を `ApiKey::new` に渡します。CLI や開発では環境変数 `ANTHROPIC_API_KEY` から読みます (`ApiKey::from_env`)。
  - `ApiKey` の `Debug` は伏せ字で、エラーの文言にキーが紛れ込んだ場合も伏せます。録画にはリクエストのヘッダ (キーを含む) を残さず、送った画像のデータも長さだけに置き換えます。
- 外部の API に画像を送る前に、ユーザーに確認します。プロジェクト単位で「送信しない」を設定できるようにします。
  - 確認に示す内容は `Provider::disclosure` で得ます。送り先 (サービス名・URL・モデル) と、画像の枚数・バイト数、テキストのバイト数を返します。ローカルで完結するプロバイダは `Destination::Local` を返します。
  - Claude API に送ったデータの扱い (保持期間など) は Anthropic の規約に従います。確認の画面ではその旨と規約へのリンクを示します。
