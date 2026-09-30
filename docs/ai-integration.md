# AI 連携設計

コードは `crates/ugoe-psd`、`crates/ugoe-mesher`、`crates/ugoe-ai`。判断の経緯は [ADR 0006](adr/0006-ai-proposals.md)。

## 目標

PSD を 1 枚渡すと、「顔の角度・目・口・呼吸・髪揺れが一通り動く」モデルが数分でできること。人間は、それをエディタで直すことに集中できるようにします。

## 原則

1. **AI は提案するだけです。** AI は `Proposal { title, rationale, commands: Vec<Command> }` を返します。エディタはそれを差分としてプレビューし、ユーザーが採用して初めて `Document::apply` で適用します。適用は `Batch` として 1 回の Undo で取り消せます。
2. **AI でなくてもできることは、決定的なアルゴリズムで行います。** メッシュ生成は `ugoe-mesher` の幾何アルゴリズムで行います。AI が決めるのは「どのレイヤーが何か」「どのパラメータでどう動かすか」といった意味の判断と、アルゴリズムのパラメータ選びだけです。
3. **プロバイダを差し替えられるようにします。** LLM や画像モデルの呼び出しは `ugoe_ai::Provider` トレイトの裏に隠します。既定のプロバイダは Anthropic の Claude API で、OpenAI 互換の API (OpenAI・OpenRouter・Gemini・手元の Ollama / LM Studio / vLLM など) も使えます ([ADR 0022](adr/0022-multiple-ai-providers-and-agents.md)、[OpenAI 互換の API](#openai-互換の-api))。プロンプトと出力の JSON Schema は特定のモデルに依存させません。
4. **ネットワークなしでテストできるようにします。** テストでは、録画した応答を返す `FakeProvider` を使います。CI では実際の API を呼びません。
5. **外部のエージェントからも操作できるようにします。** `ugoe-mcp` (MCP サーバ) で `Command` の JSON Schema をツールとして公開し、Claude Code などの外部エージェントがプロジェクトを読み、編集を提案・適用できるようにします ([MCP サーバ](#mcp-サーバ))。
   - `Command` の JSON Schema は Rust から生成してリポジトリに置いてあります (`crates/ugoe-editor-core/schema/command.schema.json`、[型の生成](editor.md#型の生成))。`ugoe-ai` などからは `ugoe_editor_core::codegen::COMMAND_SCHEMA_JSON` (ファイルの中身) か `command_schema()` (その場で生成) で参照します。

## パイプライン

```
PSD
 │ ugoe-psd::import                       (決定的)
 ▼
LayerTree (レイヤー名・グループ・画像・位置)
 │ ugoe-ai::classify_layers               (AI: 画像 + レイヤー名 → 意味ラベル)
 ▼
意味ラベル付きのレイヤー (face, eye_white + 左, iris + 左, eyelash_upper + 左, mouth, hair_front, ...)
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

コードは `ugoe_ai::classify`。

- 入力: レイヤーのサムネイル (個別の画像と、全体の合成画像)、レイヤー名、グループの構造。全体の中での位置は、合成画像の座標での外接矩形としてプロンプトに書きます。
- 出力: レイヤーごとの `LayerLabel { layer_path, label, side: left|right|center, confidence }`。モデルには `{ "layers": [{ index, label, side, confidence }] }` の形 (`classify::output_schema`) で答えさせ、`index` はプロンプトに書いたレイヤーの番号です。
- ラベルの語彙は `ugoe_ai::classify::Label` (40 個) で、左右は `Side` で別に持ちます。左右は **キャラクター自身の左右** です ([avatar-profile.md#向きと左右の約束](spec/avatar-profile.md#向きと左右の約束))。

| 分類 | ラベル |
| --- | --- |
| 顔 | `face` `ear` `nose` `cheek` `neck` |
| 目 | `eye` `eye_white` `iris` `eye_highlight` `eyelash_upper` `eyelash_lower` `eye_closed` `eyebrow` |
| 口 | `mouth` `mouth_open` `mouth_closed` `mouth_inner` `teeth` `tongue` |
| 髪 | `hair_front` `hair_side` `hair_back` |
| 体 | `body` `arm` `hand` `leg` `foot` `tail` |
| 服 | `topwear` `bottomwear` `legwear` `footwear` `handwear` `neckwear` |
| 装飾 | `headwear` `earwear` `eyewear` `accessory` |
| その他 | `background` `other` |

分類の経路は 2 つあります。

- **名前の規則 (`classify_by_name`、API を使わない):** レイヤー名を語に分け (区切り文字・camelCase・数字)、英語のキーワード (連続する 1〜3 語をつないだもの) と日本語の部分文字列で、具体的な規則から順に比べます。レイヤー名で決まらなければ内側のグループの名前から受け継ぎ (確からしさを下げる)、どれにも当たらなければ `other` (確からしさ 0) です。左右は名前 (`l` / `r` / `left` / `right` / 「左」「右」) から決め、書かれていない左右の対の部位は、顔 (なければキャンバス) の中心との位置で決めます。
- **AI (`classify_layers`):** `Provider` に画像と名前を送り、モデルが答えたレイヤーは答えを使い、答えなかったレイヤー (画素のないレイヤーを含む) は名前の規則の結果を使います。リクエストにない番号や同じレイヤーへの 2 度の答えは `AiError::InvalidResponse` にします。
- **送る画像 (`ClassifyOptions`):** 全体の合成画像 (見えているレイヤーを通常の合成で重ねたもの) は長い辺 1024 画素まで、レイヤーのサムネイルは長い辺 256 画素まで縮め、64 画素に満たないものは整数倍に拡大します。透明な部分は明るい灰色で塗ります。1 回のリクエストで分類するレイヤーは 32 枚まで (画像は合成画像と合わせて 33 枚) で、超えれば複数のリクエストに分け、それぞれに合成画像を付けます。送る前の確認には `classification_requests` の結果を `Provider::disclosure` に渡します。
- `confidence` が低いもの (既定で 0.6 未満、`LayerLabel::needs_review`) は、エディタで人間に確認を求めます。

### 自動リギング

- テンプレート (Rust で書くか、宣言的なデータで書く) が、ラベルの組み合わせから、デフォーマの構造とキーフォームの作り方を決めます。
- AI は、テンプレートでは決められない値 (目の閉じ方のカーブ、顔の向きによる変形量、パーツの前後関係) を画像から推定します。
- 生成したキーフォームは必ず `ugoe_model::validate` を通します。検証に失敗した提案は捨てて、エラーを AI にフィードバックして再試行します (上限回数あり)。

#### テンプレート (`ugoe_ai::rig::propose_rig`)

入力は PSD を取り込んだ直後のモデル (メッシュだけでデフォーマのないもの) と、メッシュごとの意味ラベル (`mesh_parts` で、レイヤーの分類の結果をメッシュの名前と `layer_path` の最後の名前で対応付ける)。
アバタープロファイルのレベル 1 のパラメータ (`AngleX` `AngleY` `AngleZ` `EyeLOpen` `EyeROpen` `MouthOpenY`) を動かすリグを提案します。`face` のラベルのメッシュがなければ組めません (`InvalidRequest`)。

| 部位 | 対象のラベル | 組み方 |
| --- | --- | --- |
| 頭の傾き (`AngleZ`) | 顔・目・口・髪の分類と `headwear` `earwear` `eyewear` (首は除く) | 回転デフォーマ。中心は顔の外接矩形の下端の中央 (あご)。±30 で ±10 度 |
| 顔の向き (`AngleX` `AngleY`) | 同上 | 頭のメッシュを包むワープ (既定 6×6) を深度からの回転で振り向かせる ([深度の決め方](#深度の決め方-ugoe_aiturn))。深度は推定した深度マップ (`TurnDepth::Map`) か、なければ顔の外接矩形に内接する楕円体 (`TurnDepth::Rules`)。どちらもラベルで補正する。角度の係数は `turn_angle_scale`。キーの間は曲線補間 |
| 目の開閉 (`EyeLOpen` `EyeROpen`) | `eye` `eye_white` `iris` `eye_highlight` `eyelash_upper` `eyelash_lower` (開いた目)、`eye_closed` (閉じた目) | 左右はラベルの左右 (中央のものは顔の中心との位置で決める)。開いた目を包むワープ (2×4) を閉じる線へ縦につぶす (0.35 で 22%、0 で 4% の高さ。弧は `eye_close_bend`)。閉じる線は、閉じた目の絵があればその中心の高さ、なければ `eye_close_line`。閉じた目の絵があれば、閉じた目を 0.36→0.3 で不透明にしてから開いた目を 0.3→0.22 で消す |
| 口の開閉 (`MouthOpenY`) | `mouth_open` (と `mouth_inner` `teeth` `tongue`)・`mouth_closed`、または `mouth` | 開いた口と閉じた口の絵があれば、開いた口を閉じた口の幅・高さの半分の隙間から開き (0→0.12 で現れる)、閉じた口を 0.12→0.3 で消す。1 枚の口なら、口を包むワープ (2×2) の下側を口の幅の `mouth_open_height` 倍下げ、少しすぼめる |

眉は目の開閉では動かしません (顔のワープと一緒に動く)。組めなかった部位 (片目がない、左右にまたがる目など) は提案の理由に書きます。

#### AI が決める値 (`ugoe_ai::rig::propose_rig_with_ai`)

| 値 | 範囲 | 既定 | 意味 |
| --- | --- | --- | --- |
| `turn_angle_scale` | 0.3〜1.2 | 0.7 | 深度からの回転の角度の係数 (`DepthRotation::angle_scale`) |
| `eye_close_line` | 0.3〜0.9 | 0.7 | 目が閉じる線の高さ (開いた目の外接矩形の上端 0〜下端 1)。閉じた目の絵がない目だけ |
| `eye_close_bend` | −0.15〜0.15 | 0.04 | 閉じた目の弧 (目の高さに対する割合。正で中ほどが下がる) |
| `mouth_open_height` | 0.1〜1.0 | 0.35 | 1 枚の口を開いたときの下唇の下がり (口の幅に対する割合) |

1. 原画 (PNG、`classify::overview_png`) と、部位の外接矩形 (画像に対する割合)・閉じた目や開いた口の絵の有無をプロンプトに書いて送り、構造化出力で 4 つの値と理由 (`reason`) を受け取る。範囲は構造化出力で書けないので説明に書く。
2. 範囲の外の値は丸めずにエラーにする。範囲の中なら複製したドキュメントに組んで適用し、`validate` を通るか確かめる。
3. 通らなければ (スキーマに合わない答えを含む)、前の答えとエラーをプロンプトに添えて問い直す。上限 (既定 3 回) まで通らなければ `InvalidProposal` を返し、呼び出し側は既定値のテンプレート (`propose_rig`) に戻せる。送信の失敗などのエラーは問い直さずに返す。
4. 採用した値・捨てた答えとエラーは結果 (`AiRig`) に残し、採用した値と理由は提案の理由に書く。

### 深度推定による振り向きリグ

判断の経緯は [ADR 0010](adr/0010-keyform-inference.md)、推定の式は [spec/keyform-inference.md#深度からの回転](spec/keyform-inference.md#深度からの回転)。

1. 原画 (PSD を合成した画像。キャンバス全体に対応する) から、単眼の深度推定モデルで相対的な深度マップ (値が大きいほど手前。尺度とずれは決まらない) を作る。
   推定は言語モデルの `Provider` ではできないので、別のトレイト `ugoe_ai::depth::DepthEstimator` の裏に隠す (送る内容の提示 `Disclosure` は `Provider` と同じ)。
2. 顔・体のワープデフォーマの制御点 (基準の姿のキャンバス座標) で深度マップを標本化し、`depth` 属性の提案を作る。レイヤーの意味ラベル (前髪は顔より手前、など) で補正する。
3. 振り向きの軸のキー (パラメータの最小・既定・最大) と、深度からの回転の設定 (`SetKeyformInference`) と、`InferKeyforms { method: depth_rotation }` のコマンド列を提案する。回転の中心と `angle_scale` もラベルから推定する。
4. ユーザーは提案をプレビューで確認し、採用後に必要な箇所を手で直す (手で直したセルは `authored` になる)。

深度の推定には AI を使いますが、回転と投影は決定的なアルゴリズム (`ugoe-rigging`) で行います。

#### 深度推定器

| 推定器 | 内容 |
| --- | --- |
| `HttpDepthEstimator` | HTTP の推論サーバ (Depth Anything などを載せたもの) に送る。送り先がループバックでなければ外部に送る扱い |
| `FixedDepthEstimator` | 決めておいた深度マップを返す (録画した深度マップでのテスト、ホストが別の手段で作った深度マップ) |

推論サーバとの約束:

- リクエスト: `POST <endpoint>`、本文は `{"image": {"type": "base64", "media_type": "image/png", "data": "..."}}`。
- 応答: `{"width": W, "height": H, "depth": [W*H 個の数]}` (行優先、x が速い)。深度マップの大きさは画像と違ってよい (キャンバスに引き伸ばして使う)。
- 値の向きは設定 (`DepthPolarity`) で指定する。既定は値が大きいほど手前 (視差・逆深度。Depth Anything・MiDaS の相対深度の出力)。
- テストは `Transport` を録画の再生 (`ReplayTransport`) に差し替えて行う (録画は `crates/ugoe-ai/tests/cassettes/depth_face.json`)。

#### 深度の決め方 (`ugoe_ai::turn`)

対象はワープデフォーマで、部位 (頭・体) はラベルから決める (子孫のメッシュに `face` があれば頭、`body` / `topwear` があれば体。指定もできる)。

| 部位 | 軸 (yaw / pitch) | 起伏 | 回転の中心 | `angle_scale` |
| --- | --- | --- | --- | --- |
| 頭 | `AngleX` / `AngleY` | 顔の幅 × 0.3 | 顔のメッシュの外接矩形の中心 | 1 |
| 体 | `BodyAngleX` / `BodyAngleY` | 体の幅 × 0.2 | 体のメッシュの外接矩形の下端の中央 | 1 |

1. **正規化:** 部位のメッシュ (頭なら `face`) が覆う点で深度マップを標本化し、2% と 98% の分位点を 0 と 1 に写す (範囲の外はクランプ)。0 が輪郭、1 が最も手前。
2. **ラベルによる補正:** ラベルは `ugoe_ai::classify::Label` で、メッシュごとに受け取る (レイヤーの分類の結果 `LayerLabel` は `mesh_labels` で、`layer_path` の最後の名前と同じ名前のメッシュに対応付ける。同じ名前が複数あるものは付けない)。制御点を覆うメッシュのうち最も手前のもの (描画順、同じなら配列の後ろ) のラベルで補正する。何も覆っていない制御点は 0 (輪郭と同じ深さ)。

   | ラベル | 補正 |
   | --- | --- |
   | `hair_front` | 0.8 以上 (前髪は額より手前) |
   | `eyewear` | 1 以上 |
   | `nose` | +0.1 |
   | `hair_side` | 0.3 以下 |
   | `hair_back` | −0.3 に固定 (輪郭より奥。振り向くと逆へ回り込む) |
   | `ear` / `earwear` | −0.1 に固定 |
   | `background` | 覆うメッシュとして数えない |
   | その他・ラベルなし | 推定のまま |

3. **ピクセル単位:** 補正した値に起伏を掛けて、制御点の深度 (キャンバスのピクセル単位) にする。回転の中心の深度 (`pivot_depth`) は 0。
4. **コマンド:** 軸のパラメータがなければ標準の値域で足す (`AddParameter`)。`SetDepth` の後、中立のキーを足し、推定の設定を深度からの回転にしてから端のキーを足す (加算合成の格子のまま端のキーを足すと、斜めのセルがその場で加算合成で作られるため)。
   足したキーのセルは補間した値の `authored` になるので `MarkKeyformDerived` で推定に戻し、最後に `InferKeyforms { method: depth_rotation }` を置く。
   もとからあったキーのセル (手で作ったセル) は書き換えない。
5. 提案は複製したドキュメントに適用し、`ugoe_model::validate` を通ることを確かめてから返す ([ADR 0006](adr/0006-ai-proposals.md))。

### 法線マップの生成

判断の経緯は [ADR 0015](adr/0015-lighting.md)、法線マップの向き (G が画像の上向き) は [spec/rendering.md#ライティング](spec/rendering.md#ライティング)。

結果はどちらの方法も提案 (`Proposal`) で、テクスチャごとの `SetNormalMap { texture, normal_map: "<texture>_normal", png }` と、モデルのライティングが無効なら `SetLighting { enabled: true }` の列です
([editor.md#法線マップ](editor.md#法線マップ))。提案は複製したドキュメントに適用し、`validate` を通ることを確かめてから返します。

- **対象のテクスチャ:** 指定がなければ、メッシュ (マスク専用と、ラベルが `background` のものを除く) が使うすべてのテクスチャ。すでに法線マップを持つテクスチャは、`replace_existing` のときだけ作り直します。
- **名前:** `<texture>_normal` (テクスチャの名前やほかの法線マップと大文字小文字を区別せずに重なれば `_normal2`、`_normal3`、…)。作り直すとき、いまの法線マップをそのテクスチャだけが指していれば同じ名前の画像を差し替え、共有していれば新しい名前にします (共有しているほかのテクスチャを変えないため)。
- **パーツごとの設定:** `NormalMapOptions { default, per_texture, textures, replace_existing }` の `per_texture` で、テクスチャの名前ごとに設定を置き換えます。
- 法線マップの大きさはテクスチャと同じです。形 (アルファがしきい値以上の画素) の外は平ら (`(0, 0, 1)`) にします。

#### 決定的な方法 (`ugoe_rigging::bulge_normal_map`、提案は `ugoe_ai::normal::propose_bulge_normal_maps`)

アルファの形から「ふくらみ」の高さ `h` を作り、その勾配を法線にします (AI は使わない)。

- **形全体をふくらませる (`radius: None`、既定):** 形の中で `∇²u = −4`、形の外 (画像の外を含む) で `u = 0` を解き、`h = strength × √u` とします。
  円のアルファからは半球 (`u = R² − r²`)、帯からは断面が半円の筒になり、ほかの形でもなめらかにふくらみます。
  輪郭からの距離 (距離変換) をそのまま高さにすると形の芯 (中心線) に折れ目ができ、光を当てると筋になるので、この方法にしました。
  解き方はマルチグリッド法 (V サイクル 8 回。粗い格子の画素は、もとの 4 画素のうち 2 つ以上が形の中なら形の中) で、決定的です。
- **縁だけを丸める (`radius: Some(r)`):** 輪郭からの距離 `d` (ユークリッド距離変換) で `t = min(d / r, 1)`、`h = strength × r × √(1 − (1 − t)²)` とします。中は平らです。
- `strength` は高さと幅の比で、1 で断面が円、0 で平ら。
- 高さを形の中だけで正規化したガウスぼかし (`smoothing`、既定 σ = 1 画素) でならし、中心差分の勾配 (隣が形の外なら片側差分) から `n = normalize(−∂h/∂x, +∂h/∂y, 1)` を作ります (画像の y は下向き、法線マップの y は上向きなので y の符号が変わる)。

#### 深度推定からの方法 (`ugoe_ai::normal::propose_depth_normal_maps` / `plan_depth_normal_maps`)

深度推定器 ([深度推定器](#深度推定器)、振り向きの提案と共通) で原画から深度マップを作り、法線にします。

1. 対象のテクスチャの画素ごとに、それを覆うメッシュの三角形 (UV 空間) の重心座標から基準の姿 (パラメータが既定値) のキャンバス座標を求め、深度マップを双線形で引きます。
   三角形が覆わない形の中の画素には、隣 (上下左右) の値の平均を広げます (64 回まで。残りは最小値)。UV が反転したメッシュでも、テクスチャの向き (接空間) で正しい法線になります。
2. 対象のすべての画素の深度の 2% と 98% の分位点を 0 と 1 に写します (範囲の外はクランプ)。テクスチャごとではなく全体で正規化するので、小さなパーツの起伏が大きくなりません。
3. 起伏 `対象が覆うキャンバスの範囲の幅 × 0.3 × strength` (キャンバスのピクセル単位) を掛け、テクスチャの 1 画素がキャンバスの何ピクセルか (三角形の面積の比の平方根) で割って、テクスチャの画素単位の高さにします。
4. 深度推定はパーツの輪郭で落ちないので、`rim` (既定 0.3) の強さで決定的な方法のふくらみ (`radius: None`) を足します。`smoothing` (既定 1.5) でならし、決定的な方法と同じく勾配から法線を作ります。

ラベルによる深度の補正 (前髪は手前、など) は使いません。補正は部位の前後関係を変えるもので、面の向きは変えないためです (前髪を一律に手前へ動かしても、法線は変わらない)。
## Claude API のプロバイダ

既定のプロバイダは `ugoe_ai::claude::ClaudeProvider` で、Anthropic の Messages API (`POST /v1/messages`) を呼びます。

- **入力と出力:** `ProviderRequest` の画像 (PNG) を base64 の画像ブロックとして、プロンプトをテキストのブロックとして 1 つのユーザーメッセージに入れて送ります。出力は構造化出力 (`output_config.format` に `{"type": "json_schema", "schema": ...}`) で、呼び出し側の JSON Schema に沿った JSON を受け取ります。スキーマのオブジェクトには `additionalProperties: false` を書きます (構造化出力の制約。`minimum` などの数値の制約や再帰するスキーマは使えません)。
- **設定 (`ClaudeConfig`):** モデル (既定は `claude-opus-5-5`)、`max_tokens` (既定は 16000。考える分を含む)、考える量 `effort` (既定は API の既定)、API の URL、1 回のリクエストの時間の上限 (既定は 600 秒)、再試行の方針。
- **応答の扱い:** テキストのブロックだけをつないで JSON として読みます (`thinking` のブロックは読み飛ばします)。`stop_reason` が `refusal` なら `AiError::Refused`、`max_tokens` なら `AiError::Truncated` を返します。
- **再試行:** HTTP 408・409・429・5xx (529 overloaded を含む)・接続の失敗・タイムアウトは、指数バックオフ (0.5 秒から倍々、上限 8 秒) で 2 回まで再試行します。`retry-after` ヘッダがあればその秒数を待ち、長すぎる (既定で 60 秒を超える) ときは待たずに `AiError::RateLimited` を返します。400・401・403・404 などは再試行しません。再試行の規則 (`transport::RetryPolicy`) は OpenAI 互換の API と共通です。
- **HTTP クライアント:** 同期の `ureq` (TLS は rustls) を使い、`Transport` トレイトの裏に隠します。

## OpenAI 互換の API

`ugoe_ai::openai::OpenAiProvider` (T0512)。Chat Completions (`POST <base_url>/chat/completions`) を呼びます。1 つの実装で、OpenAI・OpenRouter・Gemini の OpenAI 互換の窓口 (`https://generativelanguage.googleapis.com/v1beta/openai`)・手元の Ollama (`http://localhost:11434/v1`)・LM Studio (`http://localhost:1234/v1`)・vLLM などを扱います。

- **入力:** システムプロンプトを `system` のメッセージに、画像 (PNG) を data URL (`data:image/png;base64,...`) の `image_url` として、プロンプトをテキストとして 1 つのユーザーメッセージに入れます。API キーがあれば `Authorization: Bearer` で送ります。キーは省略できます (手元のサーバ)。
- **出力の形 (`StructuredOutput`)。サーバの対応の差はここで吸収します:**

  | 設定 | 送るもの | 検証と問い直し (既定) | 向く送り先 |
  | --- | --- | --- | --- |
  | `JsonSchema` (既定) | `response_format: {type: "json_schema", json_schema: {name, schema, strict: true}}` | しない (0 回) | OpenAI・Gemini・Ollama・LM Studio・vLLM・OpenRouter (対応するモデル) |
  | `JsonObject` | `response_format: {type: "json_object"}` と、プロンプトの末尾にスキーマ | する (2 回) | JSON モードだけに対応するサーバ |
  | `Prompt` | プロンプトの末尾にスキーマだけ | する (2 回) | どのサーバでも |

  - 構造化出力の `strict` はスキーマのすべてのプロパティを `required` にし、`additionalProperties: false` を書く必要があります。ugoe-ai のスキーマ (分類・リギング) はこれを満たします。
  - 受け取った JSON は、どの形でもスキーマ (`type`・`enum`・`properties`・`required`・`additionalProperties`・`items`・`anyOf`) で確かめます。通らなければ、前の答え (`assistant`) と理由 (`user`) を足して送り直します (`OpenAiConfig::json_retries` 回まで。画像も送り直します)。構造化出力を使わないモデルは、考えた内容 (`<think>…</think>`)・コードフェンス・前置きを付けることがあるので、それを除いて最初の `{` から最後の `}` までを読みます。
  - OpenRouter はモデルによって構造化出力を強制しないことがあります (スキーマを参考として扱う)。その場合も検証で気づけますが、問い直すなら `JsonObject` か `Prompt` にします。
- **応答の扱い:** 最初の `choices[0].message.content` を読みます (文字列のほか、テキストの部分の列も読みます)。`message.refusal` があれば `AiError::Refused`、`finish_reason` が `length` なら `AiError::Truncated`、`content_filter` なら `AiError::Refused { category: "content_filter" }` を返します。
- **トークン数の上限:** 既定では送りません (サーバの既定)。`OpenAiConfig::max_tokens` を決めると `max_completion_tokens` (OpenAI の今の名前) で送ります。`max_tokens` しか受け付けないサーバ (Ollama・Gemini の互換の窓口など) では `MaxTokensField::MaxTokens` にします。
- **再試行:** Claude と同じ規則 (408・409・429・5xx・接続の失敗・タイムアウト、`retry-after`)。ただし 429 でも、残高・利用上限の不足 (`insufficient_quota`・`credit_balance_exhausted`・`*_spend_limit_exceeded`・`*_usage_limit_exceeded`) は再試行しても直らないので、すぐに `AiError::Api` を返します。エラーの本文は OpenAI の `{"error": {"message", "type", "code"}}` のほか、`{"error": "..."}` と `{"message": "..."}` も読みます。
- **送り先の提示:** 送り先がループバック (`localhost`・`127.0.0.0/8`・`::1`。`transport::is_loopback_url`) なら `Destination::Local` を返します。それ以外は送り先の URL から決めたサービスの名前 (`openai::service_name`。`OpenAI API`・`OpenRouter`・`Google Gemini API`・`OpenAI-compatible API (<host>)`) と URL・モデルを返します。LAN の中の計算機 (`192.168.x.x` など) は外部として扱います。
- **API キー:** `ApiKey` (Claude と共通)。CLI や開発では `OPENAI_API_KEY` から読めます (`ApiKey::from_env_var`)。
- 録画: `tests/cassettes/openai_*.json` (Chat Completions の応答の形に合わせて手で作ったもの。`openai_ollama_*` は Ollama の形)。分類とリギングの提案のテストは、Claude の録画と同じ答えを OpenAI の形にしたもので、同じ結果になることを確かめます (`tests/classify.rs`・`tests/auto_rig.rs`)。実際の API から録るには `tests/openai_replay.rs` の `record_live`。
- 法線マップの提案は `Provider` を使いません (決定的な方法と深度推定器 `DepthEstimator`) ので、プロバイダの選択に関わりません。
- 出典: [Chat Completions の API リファレンス](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create)・[Structured Outputs](https://developers.openai.com/api/docs/guides/structured-outputs)・[エラーコード](https://developers.openai.com/api/docs/guides/error-codes)・[レート制限](https://developers.openai.com/api/docs/guides/rate-limits)・[Ollama の OpenAI 互換](https://github.com/ollama/ollama/blob/main/docs/api/openai-compatibility.mdx)・[LM Studio の構造化出力](https://lmstudio.ai/docs/developer/openai-compat/structured-output)・[OpenRouter の構造化出力](https://openrouter.ai/docs/features/structured-outputs)・[Gemini の OpenAI 互換](https://ai.google.dev/gemini-api/docs/openai)。

### 録画と再生

テストはネットワークを使わず、録画 (`Cassette`、JSON ファイル) を `ReplayTransport` で再生します。録画は `crates/ugoe-ai/tests/cassettes/` にあります。

- 録画は往復の列で、リクエストは method・URL・本文 (JSON)、応答は状態コード・一部のヘッダ (`content-type`・`request-id`・`x-request-id`・`retry-after`)・本文を持ちます。応答の代わりに通信の失敗 (`timeout`・`connect`) も書けます。
- 録画のリクエストに本文が書いてあれば、再生のときに送った本文と一致するかを確かめます。
- 録画には画像のデータ (base64 のブロックと data URL) を残さず、長さだけにします。
- `RecordingTransport` で実際の通信を包むと録画を作れます (`tests/claude_replay.rs`・`tests/openai_replay.rs` の `record_live`。既定では動かしません)。

## MCP サーバ

`crates/ugoe-mcp` (T0508)。使い方とツールの一覧は [crates/ugoe-mcp/README.md](../crates/ugoe-mcp/README.md)。

### `.ugoeproj` を直接扱う

MCP サーバは実行中のエディタにはつながず、`.ugoeproj` を自分で開いて `ugoe_editor_core::Document` を持ちます。

- エディタと同じ `Document`・`Command`・検証・Undo を使うので、編集の規則は同じです (原則 4 と ADR 0005)。
- エディタを起動していなくても使えます (CI やターミナルだけの環境、エディタの無い自動化)。エディタに IPC の口を開けずに済みます。
- 代わりに、**同じプロジェクトをエディタとサーバで同時に開いて編集してはいけません。** 保存は後に書いたほうが勝ちます。サーバで保存したら、エディタで開き直します。
- 実行中のエディタにつなぐ形 (エディタの画面で提案の差分を見て採用する) は後続の候補です。そのときも同じツールの形 (提案と適用) を保ちます。

### 提案と適用 (ADR 0006 との関係)

ADR 0006 は「AI は提案だけを行い、適用は人間が承認する」と決めています。MCP サーバでは、これを **提案と適用のツールを分けること** で守ります。

- `propose_edit` は `Command` の列 (`ugoe_ai::Proposal` と同じ `title`・`rationale`・`commands`) を受け取り、ドキュメントの複製に `Batch` として適用して、検証の結果・`ApplyReport`・変わる要素の ID を返します。元のドキュメントは変えません (読むだけのツール)。成功すると提案の ID を返します。
- `apply_proposal` は提案の ID だけを受け取り、`Document::apply` で適用します (1 回の Undo で戻せます)。提案の後にドキュメントが変わっていたら拒否します (承認した差分と違う結果にならないように。Undo で提案を作った状態に戻れば、また使えます)。
- 提案の置き場はエディタと同じ `ugoe_editor_core::Proposals` です (T0637)。提案の結果にはコマンドごとの要約と依存 (`requires`) を付け、`apply_proposal` の `exclude` でコマンドの一部を外して適用できます (外した列も複製で確かめ、通らなければ適用しません)。
- **承認するのは MCP クライアントの確認です。** Claude Code は、許可していないツールを呼ぶ前にユーザーに確認を求めます。ユーザーは `propose_edit` の差分を見て、`apply_proposal` の呼び出しを許します。`propose_edit`・`get_*`・`render_png` は読むだけ (`readOnlyHint`) なので常に許してよく、`apply_proposal` と `save_project` は常に許す設定にしないことを勧めます。常に許すことは ADR 0006 の「すべての提案を自動で採用する」設定にあたります。
- 適用してもディスクには書きません。`save_project` を呼んで初めて書きます (`destructiveHint`)。
- `--read-only` で起動すると、適用・Undo・Redo・保存のツールを出しません。エージェントは読むこと・提案すること・描くことだけができます。

### エージェントに任せる自動リギング

ADR 0022 の「外部のエージェントに判断を任せる経路」(T0513)。[自動リギング](#自動リギング) のうち、画像を見て判断する 2 つの段階 (レイヤーの分類と、テンプレートで決められない値) を、MCP クライアント側のエージェント (Claude Code・OpenCode・Codex CLI など) が自分のモデルで行います。ugoe は API を呼ばず、API キーも持たず、外部に何も送りません (画像はエージェントに返すだけです)。

| 段階 | ツール | エディタの中で API を呼ぶ経路との対応 |
| --- | --- | --- |
| PSD から作る | `import_psd` (既存のプロジェクトなら `get_psd_layers` に `psd` を渡す) | 「New from PSD…」 |
| 画像を見る | `get_psd_layers`: レイヤーの一覧・全体の合成画像・サムネイル (MCP の image コンテンツ)・分類の指示と語彙 | `classify::classification_requests` が送る画像と指示 |
| 分類する | `set_layer_labels`: `classify::output_schema` の形の答え | `classify::classify_layers` の答え |
| 値を決める | `get_rig_guide` (値の説明・顔や目の位置・原画) → `propose_rig` (`rig::rig_output_schema` の形の値) | `rig::rig_request` と `rig::propose_rig_with_ai` の答え |
| 適用する | `apply_proposal` (利用者の確認) → `render_png` で確かめる → `save_project` | 提案のパネルの Adopt |

- **同じ部品を使う:** 画像は API に送るものと同じ作り方と上限 (`ClassifyOptions` の既定値。合成画像は長い辺 1024 画素、サムネイルは 256 画素まで) で、指示の文は API に送るシステムプロンプトと同じ (`classify::classification_instructions`・`rig_request` の `system`) です。答えの読み方と確かめ方も同じ関数 (`classify::apply_answer`・`rig::params_from_answer`・`propose_rig` の範囲と検証) を通します。ツールの入力の型は ugoe-ai の JSON Schema から作り、語彙や値の範囲を手で写しません。
- **既定の案:** 分類は名前の規則 (`classify_by_name`) の結果を既定の案として持ち、エージェントは画像を見て直すレイヤーだけを渡せばよいようにします (`set_by` が `rules` か `agent` か)。値を渡さなければ `RigParams` の既定値で組みます。
- **直して呼び直す:** 番号が範囲の外・語彙にないラベル・範囲の外の値・組んだリグが検証を通らないときは、ツールの実行エラー (`isError`) で理由を返します。エージェントはそれを読んで直し、呼び直します (API の経路の問い直しにあたります)。分類は誤りがあれば何も変えません。
- **1 回に返す画像:** MCP クライアントは画像を含む結果の大きさに上限を設けることがある (Claude Code は既定で 25,000 トークン) ので、`get_psd_layers` は既定で 8 枚ずつ返し、`count` (32 枚まで)・`thumbnail_size`・`indices` で調整できます。サムネイルごとに `layer N: 名前` の文を直前に置き、画像と番号の対応が分かるようにします。
- **覚えておくもの:** PSD のパスと分類は、サーバが開いているプロジェクトごとにメモリに持ちます (エディタと同じく、プロジェクトには保存しません)。開き直したら `get_psd_layers` に `psd` を渡します。
- **深度:** 深度推定器は使わず、顔に内接する楕円体 (`TurnDepth::Rules`) で振り向かせます。

### HTTP のトランスポート

ADR 0022 の「HTTP で MCP サーバにつなぐクライアント (ChatGPT など)」のために、stdio に加えて MCP の Streamable HTTP で
待ち受けられます (`ugoe-mcp --http <addr>`。T0514)。使い方と ChatGPT からつなぐ手順は
[crates/ugoe-mcp/README.md](../crates/ugoe-mcp/README.md#http-で動かす-chatgpt-など)。

- **同じ処理:** HTTP は JSON-RPC のメッセージを運ぶだけで、ツールと状態 (開いたプロジェクトと提案) は stdio と同じ
  `Server` が持ちます。状態はプロセスに 1 つで、つないだクライアントで共有します (セッションは発行しません)。
- **応答は JSON 1 つ:** ツールは途中の通知を送らないので SSE は使いません。GET と DELETE には 405 を返します。
- **守り:** ツールは利用者の権限で任意のパスを読み書きするので、HTTP で待ち受けるときは外から届かないようにします。
  既定はループバックだけに束ね、ループバックの外に束ねるにはトークンが要ります。トークンが無ければ `Host` が
  ループバックの名前で転送ヘッダの無い要求だけを受けます (ngrok などのトンネルで認証なしに公開されないように)。
  `Origin` はループバックか許したものだけを受けます (DNS リバインディング対策)。
- **ChatGPT:** ChatGPT のコネクタは OAuth か認証なしだけで、Bearer のヘッダを送れません。手元のサーバには OpenAI の
  Secure MCP Tunnel (外向きの接続だけ) でつなぐことを勧め、公開のトンネルを使うときはトークンを URL のパスに入れます。
  OAuth の認可サーバは持ちません (後続の候補)。
- **依存:** HTTP のサーバのクレート (多くは tokio を連れてくる) を入れず、std の `TcpListener` の上に小さく書きました。

## セキュリティとプライバシー

- API キーはプロバイダごとに OS の資格情報の保存先 (キーチェーンなど。`keyring-core`) に保存します (OpenAI 互換の API は送り先のホストごと)。プロジェクトファイルや普通のファイルには書き込みません。保存先が使えない環境では環境変数だけを使います ([エディタの AI の設定](editor.md#ai-の設定と送信の同意))。
  - `ugoe-ai` はキーチェーンを直接読みません。ホスト (エディタ) が読んだ値を `ApiKey::new` に渡します。CLI や開発では環境変数 `ANTHROPIC_API_KEY` から読みます (`ApiKey::from_env`)。OpenAI は `OPENAI_API_KEY` (`ApiKey::from_env_var`。エディタは送り先が OpenAI のときだけ使います)。
  - `ApiKey` の `Debug` は伏せ字で、エラーの文言にキーが紛れ込んだ場合も伏せます。録画にはリクエストのヘッダ (キーを含む) を残さず、送った画像のデータも長さだけに置き換えます。
- 外部の API に画像を送る前に、プロバイダに関わらず同じ流れでユーザーに確認します。手元のサーバ (ループバック) は外部に出ないので確かめません。プロジェクト単位で「送信しない」を設定できます (`project.json` の `editor.ai.send_externally`)。「今後は確かめない」はプロジェクトを開いている間だけ、同じ送り先に限って効きます。
  - 確認に示す内容は `Provider::disclosure` で得ます。送り先 (サービス名・URL・モデル) と、画像の枚数・バイト数、テキストのバイト数を返します。ローカルで完結するプロバイダ (と、ループバックへ送る OpenAI 互換のプロバイダ) は `Destination::Local` を返します。
  - 送ったデータの扱い (保持期間など) は送り先の規約に従います。確認の画面では、知っている送り先 (Anthropic・OpenAI・OpenRouter・Google) なら規約へのリンクを、それ以外なら規約を確かめるよう促す文を示します。
  - 平文の `http://` で外部の送り先に API キーを送りません (エディタが断ります。ループバックは除く)。
