---
id: T0197
title: "#[non_exhaustive] を、誤り・警告の enum、設定の struct、モデルのデータの enum に付ける"
status: done
milestone: M9
crates: [ugoe-core, ugoe-model, ugoe-runtime, ugoe-format, ugoe-physics, ugoe-motion, ugoe-render, ugoe, ugoe-ffi, ugoe-wasm, docs, spec]
depends_on: [T0196]
assignee: claude-worker-T0197
---

# T0197: #[non_exhaustive] を付ける

## 目的

`#[non_exhaustive]` が format と render の一部にしか付いておらず、揃っていない (T0195 の所見)。後から種類やフィールドを足しやすくするため、付ける。

## 決まっていること (利用者、2026-10-11)

- 誤り・警告の enum、設定の struct、モデルのデータの enum (デフォーマの種類・ブレンドなど) に付ける。
- 「安定に神経質にならない」方針 (AGENTS.md) は「保証したいくらいの意味でも付けてよい」という意味合い。厳密な semver の約束のための作業はしない。付けて害の無い所に付ける。

## 受け入れ条件

- [x] 公開の enum と struct を洗い出し、付けるもの・付けないもの (付けると使う側が不便になるだけのもの。例: 中身を全部作って渡す小さな値の struct) を決めて、理由をメモに書いた
- [x] 付けた。クレートの外 (ワークスペースの別のクレート・bindings/godot・fuzz・tests/conformance) の網羅の match や構造体の式が壊れたら、`_ =>` や作る関数で直した
- [x] モデルのデータの enum に付けたことを docs/spec/model.md に一言書いた (英語の spec/ は Rust の型の話を持たないなら不要。持つなら同じく)
- [x] docs/release.md「公開の範囲」の #[non_exhaustive] の記述を、付けた形に合わせた
- [x] `just ci`・`just wasm-check`・`just nostd-check`・`just conformance`・`just godot-check` が通る

## 参考

- T0195 のメモ (公開 API の洗い出し)、docs/release.md「公開の範囲」、T0196 (内部の整理。同じクレートを触るので後にする)

## メモ

- 利用者の決定 (2026-10-11) を受けてオーケストレーターが起票した。

### やったこと (claude-worker-T0197)

洗い出しは公開する 8 クレート (`ugoe-core`・`ugoe-model`・`ugoe-runtime`・`ugoe-format`・`ugoe-physics`・`ugoe-motion`・`ugoe-render`・`ugoe`)
の `pub enum`・`pub struct` を grep で集め、根から届くもの (T0196・T0336 の後の形) を見た。

**付けた (49 の型。既に付いていた `MsgPackEncodeError`・`MsgPackDecodeError`・`ReadLimits`・`ReadLimit`・`ImageLimits`・`ImageError`・`RenderError` はそのまま)**

- 誤り・警告の enum: `CompileError`・`ComposeError` (runtime)、`FormatError`・`ReadWarning` (format)、`ValidationError`・`MotionWarning`・
  `ProfileIssue`・`Severity` (model)、`LoadError`・`LoadWarning`・`ParameterValuesError`・`DressUpError` (sdk)。
- 誤りと警告の補足の値: `LoadErrorValue`・`FieldValue`・`ReadLimitExceeded` (sdk)。
- 設定の struct: `WriteOptions` (format。`with_compress` を足した)、`RendererOptions`・`MaskAtlasOptions` (render。フィールドを書き換えて使う)、
  `Toggles` (sdk。`Toggles::NONE` と `with_physics` などの `with_*` を足した)。
- モデルのデータの enum: `ugoe-model` の公開の enum すべて (23。`DeformerKind`・`BlendMode`・`MaskMode`・`Segment`・`ExpressionOp`・
  `PhysicsTarget`・`VertexOwner`・`ColliderShape`・許諾の 4 つ・`TrackingSource` など)。
- ほかに種類が増えうる enum: `Light` (core)、`TextureFormat` (format)、`TextureSource` (sdk)、`BlinkPhase` (motion)。
- ライブラリが作って返すだけの struct: `ReadReport`・`Feature`・`FeatureDecl` (format。`FeatureDecl` は `optional`・`required` で作れる)、
  `FrameStats` (render)、`DrawableState` (runtime)。

**付けなかった (理由)**

- 使う側が中身を全部作って渡す小さな値の struct: `Rgb`・`Id`・`View`・`Lighting`・トラッキングの入力 (`TrackingInput`・`BodyInput`・
  `Rotation`)・`LipSyncInput`・`PartOpacityInput`・`DressUpChange`・`Bundle`・`ModuleFile`・`TextureData`・`Rgba8`・`ChunkTag`・`ParamIndex`。
- 中身の決まった小さな結果の struct: `EventSource`・`RestPoseCollision`・`ResolvedMaskRef`・`Particle`。
- `ugoe-model` のデータの struct (エディタ中核・AI・CLI・テストが構造体の式で作る。タスクの決定も enum だけ)。
- `Priority` (motion): `Idle < Normal < Force` の順序の決まった 3 つで、スタジオが自分の型と `From` で行き来する。付けると両方向の変換が
  書けなくなるだけなので外した。
- 非公開のフィールドを持つ型 (`Puppet`・`Renderer`・`CompiledModel`・`ModelInstance`・`MotionManager` など) は付けても変わらない。
- 公開しない `ugoe-ffi`・`ugoe-wasm` の型 (C ABI の `#[repr(C)]` の struct・wasm_bindgen の型) は外の利用者がいない。

**使う所の直し**

- `Unknown` を持つ種類の `match` は、最後の `X::Unknown` (や `A | X::Unknown`) の腕を `_` にした (同じフォールバックが新しい種類にも効く)。
  順が最後でない腕は並べ替えた (`ugoe-physics` の出力の値、`ugoe-motion` の `evaluate`、`ugoe-render` の合成モード、エディタ中核のセグメントの検査)。
- 誤りのコードの表 (エディタ中核の `message_codes`): 知らない種類は `detail` (英語の文) を渡す (既存の `PsdError`・`MeshError` と同じ)。
  コードは `CompileError` が `compile.unsupported`、`ProfileIssue` が `editor.internal`。
- C ABI: 知らない画像の形式は画像なし (`UGOE_TEXTURE_NONE`)・std 無しのデコードは `unsupported_texture`、知らない読み込みの失敗は
  `UGOE_ERROR_FORMAT`。Godot・スタジオ・適合テスト: 知らない光源は使わない、知らない画像の置き場所は画像なし。
- テストの構造体の式は `stats_with` (render の gpu テスト)・`with_compress`・`Toggles` の `with_*` に置き換えた。
- `ugoe-wasm` の `codes.rs` の `guard` は、SDK の誤りと警告を網羅で確かめられなくなった (`#[non_exhaustive]` なので `_` が要る)。
  wasm の種類だけを網羅し、SDK に種類を足したら見本も足すと書いた。
- 文書: `docs/spec/model.md#未知の値` に一言、`docs/release.md`「公開の範囲」の `#[non_exhaustive]` の項を付けた形に書き直した。
  `docs/spec/file-formats.md` の `WriteOptions`、`docs/spec/motion.md` の `Toggles` の作り方も直した。英語の `spec/` は Rust の型を書かないので直していない。

**検査:** `just ci`・`just wasm-check`・`just nostd-check`・`just conformance`・`just godot-check`・`just fuzz-check` が通った。

### 起票の候補

- SDK の誤りと警告の `code` の網羅を SDK の中で確かめる (`ugoe-wasm` の `codes.rs` の `guard` が `#[non_exhaustive]` で SDK の種類を網羅できなく
  なったため。例: `ugoe` に種類ごとの見本を返す `#[doc(hidden)]` の関数を置き、その中で網羅の `match` をする)。

