---
id: T0195
title: "1.0 の前に、crates.io に公開する 8 つのクレートの公開 API の範囲を洗い出し、狭めるものをクレートごとに決める"
status: done
milestone: M9
crates: [docs]
depends_on: []
assignee: claude-worker-T0195
---

# T0195: 公開 API の範囲の洗い出し

## 目的

M9 のコードレビューは「公開の範囲の見直し (物理の `Chain` など) は 1.0 の範囲を決めるときに行う (見送り)」とした
(docs/workflow/code-review.md#横断の所見 の「ID の型・公開の範囲」の行。元は T0316 の所見: `ugoe-physics` の `Chain`・`Particle`・`Segment`・
`StepInput`・`WorldCollider`・`constrain_segment`・`signed_angle`・`rest_offset` は公開だがクレートの外から使われていない)。
1.0 の範囲は M1〜M11 と決まった (ADR 0035) ので、1.0 の前に行う。crates.io に出したクレートは取り消せず (ADR 0027)、
1.0 の後に公開の範囲を狭めるのは破壊的な変更になる。

中身が大きいので、このタスクは **洗い出しと方針** だけを行い、直すのはクレートごとの後続のタスクにする (起票の候補としてメモに書く)。

## 範囲

- **対象:** crates.io に公開する 8 つのクレート (ADR 0027。`ugoe-core`・`ugoe-model`・`ugoe-runtime`・`ugoe-format`・`ugoe-motion`・
  `ugoe-physics`・`ugoe-render`・`ugoe`)。公開の範囲がそのまま利用者の API になるもの。
- **対象の外:** 公開しないクレート (`ugoe-ffi`・`ugoe-wasm` と、ツール層・アプリ。ワークスペースの中でだけ使う)。ただし、対象のクレートの
  公開の項目が「ワークスペースのどこから使われているか」は、これらと `bindings/godot`・`fuzz/`・`tests/conformance`・`tools/spec-check`
  (ワークスペースの外のものを含む) を含めて調べる。`@ugoe/ui` の公開の範囲は T0920 で済んでいる。C ABI と wasm の JS の API は、
  それぞれの版 (`UGOE_ABI_VERSION`・npm の版) で扱うので含めない。

## 受け入れ条件

- [x] 8 つのクレートの公開の項目 (`pub` の型・関数・定数・モジュール・再公開) を一覧にし、項目ごとに「使っている所」(同じクレートの中だけ・
      ほかの公開のクレート・公開しないクレートやバインディング・テストやベンチだけ) を調べた。調べ方 (rustdoc の JSON・`cargo public-api` など)
      をメモに書いた (道具をリポジトリに足すなら利用者に確認する。手元で一度使うだけなら足さない)
- [x] 項目ごとに方針を決めた: 公開のまま (理由) / `pub(crate)` に狭める / `#[doc(hidden)]` で隠す (ほかのワークスペースのクレートが使うが利用者には見せない) /
      別のクレートやモジュールへ移す / 判断が要る。T0316 の物理の 8 項目を必ず含める
- [x] 結果を docs/workflow/code-review.md#横断の所見 の「ID の型・公開の範囲」の行から辿れるようにした (行を直し、一覧の置き場所
      (このタスクのメモか docs の 1 節) を書いた)。公開の範囲の約束 (どういう項目を公開するか) を、今後の追加で守れる形で docs の 1 か所に書いた
      (例: docs/release.md#cratesio か各クレートの README)
- [x] 狭めるものを、クレート (か依存の順で並べられるまとまり) ごとの後続のタスクの「起票の候補」としてメモに書いた (各候補に、変える項目・
      影響するクレートとバインディング・`docs/spec` や公開仕様 `spec/` との関係)。公開仕様の型の名前と対応する項目を狭めるときは、そう書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md#横断の所見 (「ID の型・公開の範囲」の行)・T0316 (物理のレビュー) の所見の表とメモ・T0136・T0912・T0920
- ADR 0027 (crates.io に公開する範囲)・ADR 0035 (1.0 の範囲)、docs/release.md#cratesio、`scripts/crates.mjs` (`PUBLISHED`)

## メモ

- T0089 で起票した (元の候補: T0085 (code-review.md の「公開の範囲の見直しは 1.0 の範囲を決めるときに行う (見送り)」。1.0 の範囲が決まったので 1.0 の前に行う))。
- M9 の後続 (docs/roadmap.md の M9 の「後続のタスク」)。M9 の見送りの所見から出たので M9 に置く。ADR 0035 で M9 の後続も 1.0 に含める。
- 番号: モデル・ランタイム・形式の帯 (T01xx) の空き。対象の 8 つのうち 6 つがこの帯。
- 公開の範囲を変えるのは公開 API の変更。このタスクではコードを変えない。リリース前なので、狭めるときに互換の別名や移行の説明は要らない (利用者の方針)。

### 調べ方

- 公開の項目: 各クレートを `cargo +nightly rustdoc -p <クレート> -- -Z unstable-options --output-format json` で JSON にし
  (形式の版 61。`ugoe-format` は feature `png` を足した。ほかは既定の feature)、根のモジュールから `pub` のモジュール・型・関数・定数・型の別名・
  再公開 (`pub use`) をたどった (510 の道。再公開による同じ項目の別の道を含む)。feature `schema`・`ts` の下の項目 (`ugoe_core::codegen`・
  `ugoe_model::spec_files`) は手で足した。公開の型の固有の impl の公開のメソッドも別に列挙した。
- 使っている所: リポジトリのすべての `.rs` (ワークスペースの外の `bindings/godot`・`fuzz/`・`tests/conformance` を含む。`tools/spec-check` は
  TypeScript で、Rust の API を使わない) で、項目の名前と定義したクレートへの参照 (`ugoe_x`・`ugoe::x`) の両方が出てくるファイルを数え、
  「ほかの公開のクレート」「公開しないクレートとバインディング」「テスト・例・ベンチだけ」に分けた。名前が衝突する項目 (`Segment` は
  `ugoe_model::motion::Segment` と衝突する) と、外で使われていないと出た項目は grep で確かめ直した。
- 型に出てくるか: 公開の項目の型 (関数の引数と戻り値・公開のフィールド・enum の値・公開のメソッド) から参照される項目を JSON から集め、
  外で使われていない項目が公開の項目の型に出てくるか (出てくるなら狭められない) を調べた。
- 道具はリポジトリに足していない (手元で一度使った Python のスクリプト)。`cargo public-api` は入れていない。

### 項目ごとの方針

`ugoe` は下位のクレートを `ugoe::core` などとして丸ごと再公開するので、下位のクレートを狭めることが `ugoe` を狭めることになる。
「公開のまま」の理由の略: **他** = ほかの公開のクレートが使う、**型** = 公開の項目の型に出てくる、**口** = 組み込み先・ツールが使う口として意味がある、
**仕様** = `docs/spec`・公開仕様 `spec/` が API として名前を挙げる。

**ugoe-core** (31)

| 項目 | 方針 |
| --- | --- |
| `Rgb`・`Id`・`Vec2`・`Affine2`・`Mat2` (根) | 公開のまま (他・型。glam の型を出すので glam の版も API の一部) |
| モジュール `color`・`id` | 非公開に (中身はすべて根で再公開している。モジュールの道を使う所は無い) |
| `lighting::{Light, Lighting, MAX_LIGHTS}` | 公開のまま (他: render・sdk。C ABI・Godot・スタジオが使う) |
| `math::*` (`sin` から `rem_euclid_f64` まで 17) | 公開のまま (他: ランタイム層のすべて。ツールも決定的な計算に使う)。`exp`・`ln`・`trunc` はどこも使わないが、`f32` のメソッドと揃えた組なので残す |
| `lerp` | 公開のまま (他) |
| `codegen` (feature `schema`・`ts`) | 判断が要る (下の「判断が要る点」の 2) |

**ugoe-model** (253)

| 項目 | 方針 |
| --- | --- |
| データの型 (`Model`・`Mesh`・`Deformer`・`Physics*`・`Motion`・`Expression`・`Module`・`AvatarProfile` など 80 の struct・enum と 10 の ID の型の別名。根の 33 の再公開) | 公開のまま (仕様: `spec/model.md`・`spec/schema/` のスキーマそのもの。他: ほかのすべての公開のクレート)。外で使われていない `HitAreaId` もほかの ID の別名と揃えて残す |
| `standard::*` (標準のパラメータ ID の定数 46 と `range`) | 公開のまま (仕様: 標準のパラメータ。口: トラッキング・ツール)。母音の 5 つ (`MOUTH_VOWEL_*`) は外で使われていないが、表の一部なので残す |
| `avatar::*` の定数と `required_parameters`・`avatar::check` | 公開のまま (仕様: `spec/avatar-profile.md` の水準と必須のパラメータ。口: エディタ・AI・CLI の検査)。`PROFILE_VERSION`・`LEVEL_*`・`HEAD_INPUTS`・`required_parameters` は外で使われていないが、プロファイルの表を引く口として残す |
| `effects`・`pose` の定数 (`EYE_BLINK_GROUP`・`LIP_SYNC_GROUP`・`DEFAULT_FADE_DURATION`) | 公開のまま (仕様の既定値・グループ名) |
| `validate::*` (`validate`・`validate_*`・`motion_warnings`・`describe_errors`・`DESCRIBED_ERRORS_MAX`・`ValidationError`・`MotionWarning`) | 公開のまま (他: format・runtime・physics・sdk。型: `ValidationError` は sdk の誤りに出る) |
| `unknown::in_*` | 公開のまま (他: format。口: エディタが書き出す前に調べる) |
| `SCHEMA_VERSION` | 公開のまま (仕様) |
| `fixtures::*` (関数 22・定数 3・再公開 6) | 判断が要る (下の 1)。公開する 8 つのクレートの本体のコードは使わず、テスト・例のほか、CLI (`ugoe fixture`)・適合テスト・fuzz の種・エディタ中核のコード生成が使う |
| `spec_files` (feature `schema`) | 判断が要る (下の 2) |

**ugoe-runtime** (45)

| 項目 | 方針 |
| --- | --- |
| `CompiledModel`・`ModelInstance`・`ParameterValues`・`ParamIndex`・`ParameterLayout`・`DrawableState`・`DrawableChanges`・`WorldTransform`・`CompileError`・`ComposeError`・`compose`・`compose_physics`・`compose_physics_without_base` | 公開のまま (他・口) |
| `DynamicOffsets` | 公開のまま (型: `ModelInstance::dynamic_offsets(_mut)`。他: physics・sdk。`docs/portability.md#ホストから来る数` がホストの書く値として挙げる) |
| `ResolvedMaskRef` | 公開のまま (型: `CompiledModel::mask_refs`。render・C ABI が使う) |
| モジュール `compile`・`compose`・`dynamic`・`instance`・`parameters` | 非公開に (中身は `REPLACED_PART_PREFIX` を除いてすべて根で再公開している。モジュールの道を使う所は無い) |
| `compose::REPLACED_PART_PREFIX` | `pub(crate)` に (どこも使わない。`#replaced/` の名前は `docs/spec/modules.md` が仕様として書いている) |
| `keyform::{weights, Weights, ResolvedBinding, blend_scalar, blend_vec2, blend_bool}` | 公開のまま (他: physics。口: エディタ中核がキーフォームの補間に使う。`blend_*` は組) |
| `keyform::{linear_weights, GridWeights}` | `pub(crate)` に (`weights` の中の部品。どこも使わず、型にも出ない) |

**ugoe-format** (86)

| 項目 | 方針 |
| --- | --- |
| 読み書きの関数 (`read_*`・`write_*`・`*_from_json`・`*_to_json`・`check_motion`・`check_expression`) と `Bundle`・`ModuleFile`・`TextureData`・`TextureFormat`・`ChunkTag`・`WriteOptions`・`ReadLimits`・`ReadLimit`・`ReadReport`・`ReadWarning`・`FormatError` | 公開のまま (他: sdk。口: C ABI・wasm・Godot・ツール。仕様: `docs/spec/file-formats.md#api-ugoe-format`) |
| `MsgPackDecodeError`・`MsgPackEncodeError` | 公開のまま (型: `FormatError` の値) |
| `features::*` (`Feature`・`FEATURES`・`FeatureDecl`・`find`・`detect`・`detect_module`) | 公開のまま (仕様: `FEATURES` は公開仕様の機能の表と同じと `docs/spec/public-spec.md` が書く。型: `FeatureDecl` は `ReadReport` に出る) |
| `image::*` | 公開のまま (口: 組み込み先が PNG・KTX2 を読む。スタジオ・Godot・C ABI・wasm・エディタ中核が使う) |
| `MAGIC`・`bundle::CONTAINER_VERSION`・`bundle::MAX_DECOMPRESSED_CHUNK_LEN`・`bundle::DEFAULT_MAX_TOTAL_DECOMPRESSED` | 公開のまま (形式を見分ける口と、`ReadLimits` の既定値) |
| `bundle::{HEADER_LEN, CHUNK_ENTRY_LEN, CHUNK_ALIGN, CHUNK_FLAG_ZSTD}` | `pub(crate)` に (コンテナの中の配置。どこも使わない。値は公開仕様 `spec/bundle.md` に書いてある) |
| モジュール `bundle`・`features`・`image`・`module_file` | 公開のまま (根で再公開しない項目を持つ) |

**ugoe-motion** (31)

| 項目 | 方針 |
| --- | --- |
| `MotionManager`・`ExpressionManager`・`PosePlayer`・`EventSource`・`PartOpacityBuffer`・`Priority`・`EyeBlinkPlayer`・`BreathPlayer`・`LipSyncPlayer`・`tracking::*` | 公開のまま (他: sdk。型: `Puppet::motions` などに出る。口: `Puppet` を使わない組み込み先) |
| `MotionPlayer`・`ExpressionPlayer`・`BlinkPhase` | 公開のまま (型: `MotionManager::players`・`ExpressionManager::players`・`EyeBlinkPlayer::phase`。wasm が `players` を使う) |
| `curve::*` | 公開のまま (口: エディタ中核・wasm・適合テスト) |
| `tracking::standard_value` | 公開のまま (口: 標準の対応表の参照の実装。sdk のテストが期待値に使う) |
| `Rng` | `pub(crate)` に (どこも使わず、型にも出ない。スタジオの `Rng` は別のもの) |
| `easing` | `pub(crate)` に (どこも使わない。仕様の `easing(x)` は式の名前で、Rust の API ではない) |

**ugoe-physics** (23。T0316 の 8 項目を含む)

| 項目 | 方針 |
| --- | --- |
| `PhysicsSystem`・`RestPoseCollision`・`DEFAULT_FPS` | 公開のまま (他: sdk。型: `Puppet::physics`。口: エディタ中核の検査) |
| `Chain`・`Particle` (T0316) | 公開のまま、ただし読むだけに: `PhysicsSystem::chain` → `Chain::particles` → `Particle` を wasm (エディタの物理の表示) と sdk のテストが使う。`Chain::new`・`settle`・`step` は `pub(crate)` に |
| `Segment`・`StepInput` (T0316) | `pub(crate)` に (`Chain::step`・`settle` を狭めると型にも出ない) |
| `WorldCollider`・`constrain_segment` (T0316)・`SweepBuffer`・`SWEEP_MAX_STEP_DEGREES` | `pub(crate)` に (衝突の部品。どこも使わない) |
| `signed_angle`・`rest_offset` (T0316)・`ROOT_ROTATION_FOLLOW` | `pub(crate)` に (公開仕様 `spec/physics.md` に同じ名前の式・係数があるが、式の名前で Rust の API ではない) |
| `BASE_FPS`・`MAX_STEPS_PER_UPDATE` | `pub(crate)` に (どこも使わない。値は `docs/spec/physics.md` に書いてある) |
| モジュール `chain`・`collider` | 非公開に (`Chain`・`Particle` を根で再公開する) |

**ugoe-render** (11)

| 項目 | 方針 |
| --- | --- |
| すべて (`Renderer`・`RendererOptions`・`MaskAtlasOptions`・`View`・`FrameStats`・`RenderError`・`MAX_TEXTURES`・`DRAWABLE_WGSL`・`Light`・`Lighting`・`MAX_LIGHTS`) | 公開のまま (口: ホストが描く。仕様: `docs/spec/rendering.md` が `RendererOptions`・`MaskAtlasOptions`・`MAX_TEXTURES` を挙げる。`DRAWABLE_WGSL` は参照シェーダを別のエンジンへ写す口。wgpu の型を出すので wgpu の版も API の一部) |

**ugoe** (31)

| 項目 | 方針 |
| --- | --- |
| すべて (`Puppet`・`Toggles`・`VERSION`・誤りと警告の型・`limits`・`lighting`・下位のクレートの再公開) | 公開のまま (このクレートが利用者の入口。誤りと警告の値は wasm と C ABI が同じ値を返すためにここで決める) |

**メソッド:** 公開の型の公開のメソッドのうち、定義したクレートの外の `.rs` に名前が出てこないのは 28 (`ChunkTag::EXPR` などの定数、
`CompiledModel::deformer_order`・`mask_order`・`hit_area_mesh`、`MotionManager::current_priority`・`motion_numbers`、`MotionPlayer::curve_values`・
`fade_weight`、`EyeBlinkPlayer::openness`・`phase`、`PhysicsSystem::step_duration`、`Renderer::target_format`・`with_options` など)。どれも型の状態を
読む口か組の一部なので公開のまま。`WorldCollider::blocks`・`push_out` と `GridWeights::compute` は型と一緒に狭める。

### 判断が要る点 (利用者)

1. **`ugoe_model::fixtures` の扱い。** 公開する 8 つのクレートの本体は使わないが、すべてのクレートのテスト・例と、CLI・適合テスト・fuzz・
   エディタ中核のコード生成 (`spec_files::example_models` も) が使う。公開のままだと、フィクスチャの中身を変えることも semver に関わる。
   - 案 A: feature `fixtures` (既定で無効) の裏に置く。`ugoe-model` 自身は `#[cfg(any(test, feature = "fixtures"))]`。使うクレートは開発時の依存
     (CLI・適合テスト・fuzz・エディタ中核は通常の依存) で feature を有効にする。crates.io の利用者には既定で見えない。
   - 案 B: 公開のまま `#[doc(hidden)]` にし、doc コメントに「semver の約束に含めない」と書く。変更は小さいが、利用者は使えてしまう。
   - 案 C: 公開のまま、公開の API として約束する。
   - 推奨: **案 A** (feature の名前は 2 と揃える)。別のクレート (`ugoe-fixtures`) へ移す案は、`ugoe-model` 自身の単体テストが使えなくなる
     (開発時の依存の循環で型が別物になる) ので採らない。
2. **コード生成の feature (`schema`・`ts`) と `ugoe_core::codegen`・`ugoe_model::spec_files`。** feature を有効にすると `schemars`・`ts-rs` の
   トレイトの実装が公開 API に出るので、両者の版を上げることが破壊的な変更になる。使うのはエディタ中核と公開仕様の生成だけ。
   - 案 A: feature を `unstable-schema`・`unstable-ts` に改め、Cargo.toml と README に「semver の約束に含めない」と書く。`codegen`・`spec_files` は
     `#[doc(hidden)]`。
   - 案 B: 名前は今のまま、README と doc コメントに「semver の約束に含めない」と書くだけ。
   - 推奨: **案 A** (使う側の Cargo.toml を見ただけで約束の外と分かる。1 の feature も `unstable-fixtures` にすると揃う)。
3. (関連・範囲の外) **`#[non_exhaustive]` の揃え方。** 1.0 の後に値を足しうる誤り・警告の enum (`FormatError`・`ReadWarning`・`CompileError`・
   `ComposeError`・`ValidationError`・`MotionWarning`・`LoadError`・`LoadWarning`・`DressUpError` など) と設定の struct は、値を足すと
   利用者の網羅的な `match` が壊れる。`ugoe-format` と `ugoe-render` は一部に付けているが揃っていない。推奨: 誤り・警告の enum と設定の struct に
   付ける (モデルのデータの enum は `Unknown` の値と機能の宣言で前方の互換を扱うので、別に決める)。公開の範囲ではないので、起票の候補 6 にした。

### 起票の候補

依存の順 (core → model → runtime → format → motion → physics) に並べた。どれも `ugoe` の API (`ugoe::<クレート>::…`) も狭める。
公開仕様 `spec/` の型の名前と対応する項目は無い (`spec/physics.md` の `rest_offset`・`signed_angle`・`Particle`、`spec/motion.md` の `easing`、
`spec/bundle.md` の `FeatureDecl` は式・スキーマの名前で、Rust の項目を狭めても公開仕様は変わらない)。

1. **ugoe-core: モジュール `color`・`id` を非公開にする。** 影響: 無し (道を使う所が無い)。doc のリンク (`[`color::…`]`) を直す。
2. **ugoe-runtime: モジュール `compile`・`compose`・`dynamic`・`instance`・`parameters` を非公開にし、`REPLACED_PART_PREFIX`・
   `keyform::linear_weights`・`keyform::GridWeights` を `pub(crate)` にする。** 影響: 無し (外で使う所が無い)。`docs/spec/evaluation.md` は
   `DynamicOffsets` を根の名前で挙げているので変わらない。
3. **ugoe-format: `bundle::{HEADER_LEN, CHUNK_ENTRY_LEN, CHUNK_ALIGN, CHUNK_FLAG_ZSTD}` を `pub(crate)` にする。** 影響: 無し。
   `docs/spec/file-formats.md` は値を書いているが名前を API として挙げていない。
4. **ugoe-motion: `Rng`・`easing` を `pub(crate)` にする。** 影響: 無し。`docs/spec/motion.md` の `easing` は式の名前。
5. **ugoe-physics: T0316 の所見。** モジュール `chain`・`collider` を非公開にし (`Chain`・`Particle` を根で再公開)、`Chain::new`・`settle`・`step`・
   `Segment`・`StepInput`・`WorldCollider`・`SweepBuffer`・`constrain_segment`・`SWEEP_MAX_STEP_DEGREES`・`signed_angle`・`rest_offset`・
   `ROOT_ROTATION_FOLLOW`・`BASE_FPS`・`MAX_STEPS_PER_UPDATE` を `pub(crate)` にする。`Chain` は `particles`・`is_finite` だけの読む型にする。
   影響: `ugoe-wasm` の物理の表示 (`PhysicsSystem::chain` → `particles`) と `crates/ugoe/tests/m3_puppet.rs` は今のまま使える。
   `crates/ugoe-physics/README.md` の `chain::Chain`・`collider::…` の説明を直す (`docs/testing.md` の `WorldCollider::blocks` は内部の時間の内訳の説明なのでそのまま)。
   `docs/spec/physics.md` の `ROOT_ROTATION_FOLLOW`・`MAX_STEPS_PER_UPDATE` は値の説明として残る。
6. **公開するクレートの誤り・警告の enum と設定の struct の `#[non_exhaustive]` を揃える** (判断が要る点の 3。決めてから)。
   影響: 組み込み先 (`ugoe-ffi`・`ugoe-wasm`・Godot) の網羅的な `match` に `_` の腕が要る。
7. **`ugoe_model::fixtures` とコード生成の feature の扱い** (判断が要る点の 1・2。決めてから)。影響: `fixtures` を使うすべてのクレートの
   Cargo.toml (開発時の依存の feature)、`ugoe-cli`・`tests/conformance`・`fuzz`・`ugoe-editor-core`・`bindings/godot` のテスト、
   `just nostd-check`・`just wasm-check` の feature の組み合わせ、AGENTS.md の「`ugoe_model::fixtures` を入力にしてテストする」の記述。
   feature の名前を変えるなら `ugoe-editor-core` の依存と `docs/editor.md#型の生成` も直す。

### やらなかったこと

- コードは変えていない (このタスクは洗い出しと方針だけ)。
- `ugoe-ffi`・`ugoe-wasm`・ツール層・アプリの公開の範囲は調べていない (範囲の外)。使っている所としてだけ数えた。
