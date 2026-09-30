# 公開仕様 (`spec/`) との対応表

英語の公開仕様 [`spec/`](../../spec/README.md) (CC BY 4.0) と、この日本語の開発用の仕様書 (`docs/spec/`) の節の対応です。
方針は [ADR 0021](../adr/0021-public-specification.md)。**両者が食い違ったら英語版が正** です。

- `ugoe-model` の型やファイル形式を変えたら、`docs/spec/` と同じ PR で、この表の対応する `spec/` の節も直します。
- `spec/` は実装の型名 (Rust)・タスク ID・日本語の文書へのリンクに頼らず、それだけで読めるように書きます。フィールド名は JSON / MessagePack の名前で書きます。
- `docs/spec/` にだけあるもの (実装の型名・API・判断の経緯・性能の計測・エディタのプロジェクト形式) は、`spec/` に書きません。
- `spec/` の相対リンクは `node scripts/check-md-links.mjs spec` で検査できます。
- 対応を CI で検査する仕組みは T0119 で作ります。

## モデル (`spec/model.md`)

| `spec/model.md` | `docs/spec/` | 備考 |
| --- | --- | --- |
| [Data conventions](../../spec/model.md#data-conventions) | — | 型の表記、JSON と MessagePack の対応、省略と既定値、`null`、enum の 4 つの表現。日本語版にまとまった節はない |
| [Coordinate system](../../spec/model.md#coordinate-system) | [evaluation.md](evaluation.md#3-デフォーマのワールド変換)、`ugoe_model::Canvas` の doc | |
| [Identifiers](../../spec/model.md#identifiers) | [model.md#ID](model.md#id) | 合成で使う `/` と `#` を避ける推奨は英語版だけ |
| [Model](../../spec/model.md#model) | [model.md#トップレベル](model.md#トップレベル) | |
| [Parameters](../../spec/model.md#parameters) | [model.md#パラメータ](model.md#パラメータ) | |
| [Well-known parameter IDs](../../spec/model.md#well-known-parameter-ids) | [model.md#標準パラメータ](model.md#標準パラメータ)、[avatar-profile.md#拡張パラメータ-レベル-3](avatar-profile.md#拡張パラメータ-レベル-3) | 値域の表は `ugoe_model::standard` と同じ |
| [Parts](../../spec/model.md#parts) | [model.md#パーツ](model.md#パーツ) | |
| [Keyform grids](../../spec/model.md#keyform-grids) | [model.md#キーフォームの補間](model.md#キーフォームの補間)、[evaluation.md#1-キーフォームの重み](evaluation.md#1-キーフォームの重み) | 並び (軸 0 が最も速い) は英語版では model.md に書く |
| [Blend shapes](../../spec/model.md#blend-shapes) | [model.md#ブレンドシェイプ](model.md#ブレンドシェイプ) | |
| [Deformers](../../spec/model.md#deformers) | [model.md#デフォーマ](model.md#デフォーマ) | 変換の式は T0117 (`spec/evaluation.md`) |
| [Meshes](../../spec/model.md#meshes)・[Masks](../../spec/model.md#masks) | [model.md#メッシュ](model.md#メッシュ) | マスクの式は T0117 (`spec/rendering.md`) |
| [Glues](../../spec/model.md#glues) | [model.md#グルー-m2](model.md#グルー-m2) | |
| [Textures](../../spec/model.md#textures)・[Normal maps and lighting](../../spec/model.md#normal-maps-and-lighting) | [model.md#ライティング](model.md#ライティング) | 法線の符号化と陰影は T0117 |
| [Hit areas](../../spec/model.md#hit-areas) | [model.md#トップレベル](model.md#トップレベル) | |
| [Pose](../../spec/model.md#pose) | [model.md#ポーズ](model.md#ポーズ) | |
| [Effects](../../spec/model.md#effects) | [model.md#エフェクト設定](model.md#エフェクト設定)、[motion.md#既定値](motion.md#既定値) | 挙動は T0117 (`spec/motion.md`) |
| [Avatar profile](../../spec/model.md#avatar-profile) | [model.md#アバタープロファイル](model.md#アバタープロファイル)、[avatar-profile.md#メタデータ](avatar-profile.md#メタデータ)、[avatar-profile.md#アプリ向けの情報](avatar-profile.md#アプリ向けの情報) | データだけ。意味・準拠レベル・トラッキングの式・アプリの規則は [spec/avatar-profile.md](#アバタープロファイル-specavatar-profilemd) |
| [Slots](../../spec/model.md#slots) | [model.md#スロット](model.md#スロット)、[modules.md#スロット](modules.md#スロット) | |
| [Authoring data](../../spec/model.md#authoring-data) | [model.md#キーフォーム推定のデータ](model.md#キーフォーム推定のデータ) | 推定のアルゴリズム ([keyform-inference.md](keyform-inference.md)) は公開仕様に含めない (ツールの内部) |
| [Physics settings](../../spec/model.md#physics-settings) | [physics.md](physics.md) の各節 | データだけ。振り子の式は T0117 (`spec/physics.md`) |
| [Motions](../../spec/model.md#motions) | [motion.md#カーブ](motion.md#カーブ)、[motion.md#対象](motion.md#対象) | データだけ。再生は T0117 |
| [Expressions](../../spec/model.md#expressions) | [motion.md#表情](motion.md#表情) | データだけ |
| [Modules](../../spec/model.md#modules) | [modules.md#モジュール](modules.md#モジュール)、[modules.md#ポーズ](modules.md#ポーズ) | |
| [Composition](../../spec/model.md#composition) | [modules.md#合成](modules.md#合成) | `Puppet` の API ([modules.md#puppet-での着せ替え](modules.md#puppet-での着せ替え)) は含めない |
| [Unknown values](../../spec/model.md#unknown-values) | [model.md#未知の値](model.md#未知の値)、[file-formats.md#未知の値の読み込み](file-formats.md#未知の値の読み込み) | |
| [Model validation](../../spec/model.md#model-validation) | [model.md#検証](model.md#検証) | |
| [Physics validation](../../spec/model.md#physics-validation) | [model.md#物理の検証](model.md#物理の検証) | |
| [Module validation](../../spec/model.md#module-validation) | [modules.md#検証](modules.md#検証) | |

## バンドル (`spec/bundle.md`)

| `spec/bundle.md` | `docs/spec/` | 備考 |
| --- | --- | --- |
| [File kinds](../../spec/bundle.md#file-kinds) | [file-formats.md#一覧](file-formats.md#一覧) | プロジェクト (`.ugoeproj`) は公開仕様に含めない (ADR 0021) |
| [Container layout](../../spec/bundle.md#container-layout)・[Chunk types](../../spec/bundle.md#chunk-types) | [file-formats.md#バンドル-ugoe](file-formats.md#バンドル-ugoe) | |
| [Payload encoding](../../spec/bundle.md#payload-encoding) | [file-formats.md#ペイロードのエンコード](file-formats.md#ペイロードのエンコード) | 展開後の上限と MessagePack の入れ子の上限は英語版に要件として書く |
| [Textures and normal maps](../../spec/bundle.md#textures-and-normal-maps) | [file-formats.md#バンドル-ugoe](file-formats.md#バンドル-ugoe)、[file-formats.md#モジュールと法線マップのファイル](file-formats.md#モジュールと法線マップのファイル) | |
| [Metadata](../../spec/bundle.md#metadata) | [file-formats.md#バンドル-ugoe](file-formats.md#バンドル-ugoe) | |
| [Bundle files](../../spec/bundle.md#bundle-files) | [file-formats.md#バンドル-ugoe](file-formats.md#バンドル-ugoe) | |
| [Module files](../../spec/bundle.md#module-files) | [file-formats.md#モジュール-ugoemod](file-formats.md#モジュール-ugoemod) | |
| [Reading](../../spec/bundle.md#reading) | [file-formats.md#読み込み時の検査](file-formats.md#読み込み時の検査) | |
| [Writing](../../spec/bundle.md#writing) | [file-formats.md#api-ugoe-format](file-formats.md#api-ugoe-format) | |
| [Versioning](../../spec/bundle.md#versioning) | [file-formats.md#バージョニング](file-formats.md#バージョニング) | |
| [Forward compatibility](../../spec/bundle.md#forward-compatibility)・[Feature declarations](../../spec/bundle.md#feature-declarations) | [file-formats.md#機能の宣言](file-formats.md#機能の宣言) | |
| [Baseline features](../../spec/bundle.md#baseline-features)・[Features after 1.0](../../spec/bundle.md#features-after-10) | [file-formats.md#機能とフォールバックの一覧](file-formats.md#機能とフォールバックの一覧) | 機能 ID の表は `ugoe_format::features::FEATURES` と同じ |
| [Re-saving](../../spec/bundle.md#re-saving) | [file-formats.md#再保存](file-formats.md#再保存) | |
| [Standalone JSON files](../../spec/bundle.md#standalone-json-files) | [file-formats.md#api-ugoe-format](file-formats.md#api-ugoe-format) | |

## アバタープロファイル (`spec/avatar-profile.md`)

| `spec/avatar-profile.md` | `docs/spec/` | 備考 |
| --- | --- | --- |
| [Overview](../../spec/avatar-profile.md#overview) | [avatar-profile.md](avatar-profile.md) の冒頭 | アバター・準拠するモデル・アプリ・検証ツールの用語は英語版だけ |
| [Orientation and sides](../../spec/avatar-profile.md#orientation-and-sides) | [avatar-profile.md#向きと左右の約束](avatar-profile.md#向きと左右の約束)、[model.md#標準パラメータ](model.md#標準パラメータ) | |
| [Standard parameters](../../spec/avatar-profile.md#standard-parameters)・[Extended parameters](../../spec/avatar-profile.md#extended-parameters)・[Vowel parameters](../../spec/avatar-profile.md#vowel-parameters) | [avatar-profile.md#拡張パラメータ-レベル-3](avatar-profile.md#拡張パラメータ-レベル-3)、[model.md#標準パラメータ](model.md#標準パラメータ) | 値域は `ugoe_model::standard`。標準パラメータの意味の表は英語版だけ |
| [Conformance levels](../../spec/avatar-profile.md#conformance-levels) | [avatar-profile.md#準拠レベル](avatar-profile.md#準拠レベル) | 必要なパラメータは `ugoe_model::avatar::LEVEL_1_PARAMETERS` などと同じ |
| [Conformance checks](../../spec/avatar-profile.md#conformance-checks) | [avatar-profile.md#準拠の検査](avatar-profile.md#準拠の検査) | 項目と重さは `ugoe_model::avatar::check` と同じ |
| [Tracking input](../../spec/avatar-profile.md#tracking-input)・[Standard mapping](../../spec/avatar-profile.md#standard-mapping) | [avatar-profile.md#頭の向き](avatar-profile.md#頭の向き)、[avatar-profile.md#arkit-のブレンドシェイプ](avatar-profile.md#arkit-のブレンドシェイプ) | 式は `ugoe_motion::tracking` と同じ。`MouthX`・`JawX` の符号は両方で未決 |
| [Overrides](../../spec/avatar-profile.md#overrides)・[Computing the parameter values](../../spec/avatar-profile.md#computing-the-parameter-values) | [avatar-profile.md#トラッキングの対応の上書き](avatar-profile.md#トラッキングの対応の上書き) | |
| [Audio](../../spec/avatar-profile.md#audio) | [avatar-profile.md#音声-リップシンク](avatar-profile.md#音声-リップシンク) | |
| [Gaze](../../spec/avatar-profile.md#gaze)・[Hit areas](../../spec/avatar-profile.md#hit-areas)・[Slots](../../spec/avatar-profile.md#slots) | [avatar-profile.md#アプリ向けの情報](avatar-profile.md#アプリ向けの情報) | |
| [Metadata and license terms](../../spec/avatar-profile.md#metadata-and-license-terms) | [avatar-profile.md#メタデータ](avatar-profile.md#メタデータ) | |
| [Rules for applications](../../spec/avatar-profile.md#rules-for-applications) | [avatar-profile.md#アプリの規則](avatar-profile.md#アプリの規則)、[avatar-profile.md#実装の範囲](avatar-profile.md#実装の範囲) | MediaPipe の観察は英語版では参考 (informative) |
| [Versions](../../spec/avatar-profile.md#versions) | [avatar-profile.md#バージョン](avatar-profile.md#バージョン) | |

## まだ公開仕様にない文書

| `docs/spec/` | 公開仕様 | タスク |
| --- | --- | --- |
| [evaluation.md](evaluation.md) | `spec/evaluation.md` | T0117 |
| [rendering.md](rendering.md) | `spec/rendering.md` | T0117 |
| [physics.md](physics.md) | `spec/physics.md` | T0117 |
| [motion.md](motion.md) | `spec/motion.md` | T0117 |
| [keyform-inference.md](keyform-inference.md) | 含めない (ツールの内部。結果のデータは `spec/model.md` の Authoring data) | — |
