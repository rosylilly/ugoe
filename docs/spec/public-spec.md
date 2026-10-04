# 公開仕様 (`spec/`) との対応表

英語の公開仕様 [`spec/`](../../spec/README.md) (CC BY 4.0) と、この日本語の開発用の仕様書 (`docs/spec/`) の節の対応です。
方針は [ADR 0021](../adr/0021-public-specification.md)。**両者が食い違ったら英語版が正** です。

- `ugoe-model` の型やファイル形式を変えたら、`docs/spec/` と同じ PR で、この表の対応する `spec/` の節も直します。
- `spec/` は実装の型名 (Rust)・タスク ID・日本語の文書へのリンクに頼らず、それだけで読めるように書きます。フィールド名は JSON / MessagePack の名前で書きます。
- `docs/spec/` にだけあるもの (実装の型名・API・判断の経緯・性能の計測・エディタのプロジェクト形式) は、`spec/` に書きません。
- 食い違いと対応の漏れは `just spec` (`just ci` と CI に含まれる) で検査します ([同期の検査](#同期の検査))。

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
| [Deformers](../../spec/model.md#deformers) | [model.md#デフォーマ](model.md#デフォーマ) | 変換の式は [spec/evaluation.md](../../spec/evaluation.md#deformers) |
| [Meshes](../../spec/model.md#meshes)・[Masks](../../spec/model.md#masks) | [model.md#メッシュ](model.md#メッシュ) | マスクの式は [spec/rendering.md](../../spec/rendering.md#masks) |
| [Glues](../../spec/model.md#glues) | [model.md#グルー-m2](model.md#グルー-m2) | |
| [Textures](../../spec/model.md#textures)・[Normal maps and lighting](../../spec/model.md#normal-maps-and-lighting) | [model.md#ライティング](model.md#ライティング) | 法線の符号化と陰影は [spec/rendering.md](../../spec/rendering.md#lighting) |
| [Hit areas](../../spec/model.md#hit-areas) | [model.md#トップレベル](model.md#トップレベル) | |
| [Pose](../../spec/model.md#pose) | [model.md#ポーズ](model.md#ポーズ) | |
| [Effects](../../spec/model.md#effects) | [model.md#エフェクト設定](model.md#エフェクト設定)、[motion.md#既定値](motion.md#既定値) | 挙動は [spec/motion.md](../../spec/motion.md#effects) |
| [Avatar profile](../../spec/model.md#avatar-profile) | [model.md#アバタープロファイル](model.md#アバタープロファイル)、[avatar-profile.md#メタデータ](avatar-profile.md#メタデータ)、[avatar-profile.md#アプリ向けの情報](avatar-profile.md#アプリ向けの情報) | データだけ。意味・準拠レベル・トラッキングの式・アプリの規則は [spec/avatar-profile.md](#アバタープロファイル-specavatar-profilemd) |
| [Slots](../../spec/model.md#slots) | [model.md#スロット](model.md#スロット)、[modules.md#スロット](modules.md#スロット) | |
| [Authoring data](../../spec/model.md#authoring-data) | [model.md#キーフォーム推定のデータ](model.md#キーフォーム推定のデータ) | 推定のアルゴリズム ([keyform-inference.md](keyform-inference.md)) は公開仕様に含めない (ツールの内部) |
| [Physics settings](../../spec/model.md#physics-settings) | [physics.md](physics.md) の各節 | データだけ。振り子の式は [spec/physics.md](../../spec/physics.md) |
| [Motions](../../spec/model.md#motions) | [motion.md#カーブ](motion.md#カーブ)、[motion.md#対象](motion.md#対象) | データだけ。再生は [spec/motion.md](../../spec/motion.md#motion-playback) |
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
| [Payload encoding](../../spec/bundle.md#payload-encoding) | [file-formats.md#ペイロードのエンコード](file-formats.md#ペイロードのエンコード) | MessagePack の入れ子の上限は英語版に要件として書く |
| [Resource limits](../../spec/bundle.md#resource-limits) | [file-formats.md#読み込みの上限](file-formats.md#読み込みの上限) | 上限は形式でなく読み込み器の設定 (T0150)。参照実装の既定値は Informative |
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
| [Standard parameters](../../spec/avatar-profile.md#standard-parameters)・[Extended parameters](../../spec/avatar-profile.md#extended-parameters)・[Body extended parameters](../../spec/avatar-profile.md#body-extended-parameters)・[Vowel parameters](../../spec/avatar-profile.md#vowel-parameters) | [avatar-profile.md#拡張パラメータ-レベル-3](avatar-profile.md#拡張パラメータ-レベル-3)、[avatar-profile.md#体の拡張パラメータ](avatar-profile.md#体の拡張パラメータ)、[model.md#標準パラメータ](model.md#標準パラメータ) | 値域は `ugoe_model::standard`。標準パラメータの意味の表は英語版だけ |
| [Conformance levels](../../spec/avatar-profile.md#conformance-levels) | [avatar-profile.md#準拠レベル](avatar-profile.md#準拠レベル) | 必要なパラメータは `ugoe_model::avatar::LEVEL_1_PARAMETERS` などと同じ |
| [Conformance checks](../../spec/avatar-profile.md#conformance-checks) | [avatar-profile.md#準拠の検査](avatar-profile.md#準拠の検査) | 項目と重さは `ugoe_model::avatar::check` と同じ |
| [Tracking input](../../spec/avatar-profile.md#tracking-input)・[Standard mapping](../../spec/avatar-profile.md#standard-mapping) | [avatar-profile.md#頭と体](avatar-profile.md#頭と体)、[avatar-profile.md#arkit-のブレンドシェイプ](avatar-profile.md#arkit-のブレンドシェイプ) | 式は `ugoe_motion::tracking` と同じ。`MouthX`・`JawX` の符号は両方で未決 |
| [Overrides](../../spec/avatar-profile.md#overrides)・[Computing the parameter values](../../spec/avatar-profile.md#computing-the-parameter-values) | [avatar-profile.md#トラッキングの対応の上書き](avatar-profile.md#トラッキングの対応の上書き) | |
| [Audio](../../spec/avatar-profile.md#audio) | [avatar-profile.md#音声-リップシンク](avatar-profile.md#音声-リップシンク) | |
| [Gaze](../../spec/avatar-profile.md#gaze)・[Hit areas](../../spec/avatar-profile.md#hit-areas)・[Slots](../../spec/avatar-profile.md#slots) | [avatar-profile.md#アプリ向けの情報](avatar-profile.md#アプリ向けの情報) | |
| [Metadata and license terms](../../spec/avatar-profile.md#metadata-and-license-terms) | [avatar-profile.md#メタデータ](avatar-profile.md#メタデータ) | |
| [Rules for applications](../../spec/avatar-profile.md#rules-for-applications) | [avatar-profile.md#アプリの規則](avatar-profile.md#アプリの規則)、[avatar-profile.md#実装の範囲](avatar-profile.md#実装の範囲)、[avatar-profile.md#体のトラッキング-mediapipe-pose-landmarker](avatar-profile.md#体のトラッキング-mediapipe-pose-landmarker) | MediaPipe の観察は英語版では参考 (informative)。Pose Landmarker の点からの式の詳細と確かめたことは日本語版だけ |
| [Versions](../../spec/avatar-profile.md#versions) | [avatar-profile.md#バージョン](avatar-profile.md#バージョン) | |

## 評価 (`spec/evaluation.md`)

| `spec/evaluation.md` | `docs/spec/` | 備考 |
| --- | --- | --- |
| [Overview](../../spec/evaluation.md#overview) | [evaluation.md#8-出力](evaluation.md#8-出力) | 変更フラグ・ワールド変換を読む API は含めない (実装の API) |
| [Numbers and determinism](../../spec/evaluation.md#numbers-and-determinism) | [evaluation.md#用語と前提](evaluation.md#用語と前提) | 英語版は演算の順序を推奨 (RECOMMENDED) にし、許容誤差で互換を決める |
| [Conformance](../../spec/evaluation.md#conformance) | [evaluation.md#互換の基準](evaluation.md#互換の基準) | 許容誤差は `spec/examples` の期待値 (T0119) と同じ |
| [Parameter values](../../spec/evaluation.md#parameter-values) | `ugoe_runtime::ParameterValues` の doc | |
| [Keyform interpolation](../../spec/evaluation.md#keyform-interpolation) | [evaluation.md#1-キーフォームの重み](evaluation.md#1-キーフォームの重み) | 方式の比較 (T0314) は含めない |
| [Blend shapes](../../spec/evaluation.md#blend-shapes) | [evaluation.md#6-ブレンドシェイプ](evaluation.md#6-ブレンドシェイプ) | |
| [Order of evaluation](../../spec/evaluation.md#order-of-evaluation) | [evaluation.md#2-評価の順序](evaluation.md#2-評価の順序) | |
| [Parts](../../spec/evaluation.md#parts) | [evaluation.md#2-評価の順序](evaluation.md#2-評価の順序) | |
| [Deformers](../../spec/evaluation.md#deformers) | [evaluation.md#3-デフォーマのワールド変換](evaluation.md#3-デフォーマのワールド変換) | 外挿の方式の比較 (T0104) は informative の 1 文だけ |
| [Meshes](../../spec/evaluation.md#meshes) | [evaluation.md#4-メッシュ](evaluation.md#4-メッシュ) | |
| [Glues](../../spec/evaluation.md#glues) | [evaluation.md#5-グルー](evaluation.md#5-グルー) | |
| [Render order](../../spec/evaluation.md#render-order) | [evaluation.md#7-描画順](evaluation.md#7-描画順) | |
| [Dynamic physics offsets](../../spec/evaluation.md#dynamic-physics-offsets) | [evaluation.md#動的物理を使う場合-2-段階の評価](evaluation.md#動的物理を使う場合-2-段階の評価) | 2 段目で一部だけ評価し直すのは informative (結果は全体の評価と同じ) |
| [Hit testing](../../spec/evaluation.md#hit-testing) | [evaluation.md#9-当たり判定](evaluation.md#9-当たり判定) | API の表は含めない |

## 描画 (`spec/rendering.md`)

| `spec/rendering.md` | `docs/spec/` | 備考 |
| --- | --- | --- |
| [Inputs](../../spec/rendering.md#inputs)・[Drawing a frame](../../spec/rendering.md#drawing-a-frame) | [rendering.md#gpu-リソースの方針](rendering.md#gpu-リソースの方針)、[rendering.md#ビュー変換](rendering.md#ビュー変換) | GPU の資源の扱いは含めない |
| [Color values](../../spec/rendering.md#color-values)・[Textures](../../spec/rendering.md#textures) | [rendering.md#前提](rendering.md#前提) | 画素の中心での被覆 (英語版は Drawing a frame) も日本語版は前提に書く |
| [Mesh color](../../spec/rendering.md#mesh-color) | [rendering.md#描画可能要素ごとのシェーディング](rendering.md#描画可能要素ごとのシェーディング) | |
| [Blend modes](../../spec/rendering.md#blend-modes) | [rendering.md#合成モード](rendering.md#合成モード) | |
| [Masks](../../spec/rendering.md#masks) | [rendering.md#マスク](rendering.md#マスク) | 参照レンダラの実装方式 (アトラス) は含めない (Implementation notes に要約だけ) |
| [Back-face culling](../../spec/rendering.md#back-face-culling) | [rendering.md#裏面カリング](rendering.md#裏面カリング) | |
| [Lighting](../../spec/rendering.md#lighting) | [rendering.md#ライティング](rendering.md#ライティング) | 参照レンダラ・ホストでの扱いは含めない |
| [Unknown values](../../spec/rendering.md#unknown-values) | [rendering.md#合成モード](rendering.md#合成モード)、[rendering.md#マスクの参照](rendering.md#マスクの参照) | |
| [Conformance](../../spec/rendering.md#conformance) | [rendering.md#互換の基準](rendering.md#互換の基準) | |
| [Implementation notes](../../spec/rendering.md#implementation-notes) | [rendering.md#描画の最適化](rendering.md#描画の最適化)、[rendering.md#転送の省略](rendering.md#転送の省略) | informative。見た目を変えない実装の工夫 |

## 物理 (`spec/physics.md`)

| `spec/physics.md` | `docs/spec/` | 備考 |
| --- | --- | --- |
| [Overview](../../spec/physics.md#overview) | [physics.md#概要](physics.md#概要) | |
| [Coordinates and units](../../spec/physics.md#coordinates-and-units) | [physics.md#座標と単位](physics.md#座標と単位) | |
| [State](../../spec/physics.md#state)・[Advancing time](../../spec/physics.md#advancing-time) | [physics.md#時間の進め方](physics.md#時間の進め方) | 英語版は 1 回の更新の手順を擬似コードで書く |
| [Inputs](../../spec/physics.md#inputs) | [physics.md#入力](physics.md#入力) | |
| [Pendulum step](../../spec/physics.md#pendulum-step)・[Rest pose](../../spec/physics.md#rest-pose) | [physics.md#振り子チェーン](physics.md#振り子チェーン) | 係数の決め方の根拠 (T0301) は含めない |
| [Outputs](../../spec/physics.md#outputs) | [physics.md#出力](physics.md#出力)、[physics.md#合成の方法](physics.md#合成の方法) | |
| [Rig order and relays](../../spec/physics.md#rig-order-and-relays) | [physics.md#リグの順序と中継](physics.md#リグの順序と中継) | |
| [Dynamic physics](../../spec/physics.md#dynamic-physics) | [physics.md#動的物理-揺れ用パラメータのいらない物理](physics.md#動的物理-揺れ用パラメータのいらない物理) | 衝突体の掃引も擬似コードで書く。`rest_pose_collisions` の API は含めない |
| [Evaluation with dynamic physics](../../spec/physics.md#evaluation-with-dynamic-physics) | [physics.md#評価との関係](physics.md#評価との関係) | |
| [Unknown values and invalid references](../../spec/physics.md#unknown-values-and-invalid-references) | [physics.md#旧ランタイムでの扱い](physics.md#旧ランタイムでの扱い) | |
| [Non-finite state](../../spec/physics.md#non-finite-state) | [physics.md#有限でない状態](physics.md#有限でない状態) | |
| [Determinism](../../spec/physics.md#determinism)・[Conformance](../../spec/physics.md#conformance) | [physics.md#決定性](physics.md#決定性)、[physics.md#互換の基準](physics.md#互換の基準) | |

## モーション (`spec/motion.md`)

| `spec/motion.md` | `docs/spec/` | 備考 |
| --- | --- | --- |
| [Overview](../../spec/motion.md#overview) | [motion.md#モーション](motion.md#モーション) | |
| [Curves](../../spec/motion.md#curves) | [motion.md#カーブ](motion.md#カーブ) | |
| [Motion playback](../../spec/motion.md#motion-playback) | [motion.md#再生とフェード](motion.md#再生とフェード) | |
| [Playing several motions](../../spec/motion.md#playing-several-motions) | [motion.md#モーションマネージャ](motion.md#モーションマネージャ)、[motion.md#エフェクトグループ](motion.md#エフェクトグループ) | 優先度は SHOULD (ランタイムの API のため) |
| [Expressions](../../spec/motion.md#expressions) | [motion.md#表情](motion.md#表情) | |
| [Effects](../../spec/motion.md#effects) | [motion.md#エフェクト](motion.md#エフェクト) | `Effects::standard()` の推奨値は含めない |
| [Pose](../../spec/motion.md#pose)・[Part opacity](../../spec/motion.md#part-opacity) | [motion.md#ポーズ](motion.md#ポーズ)、[motion.md#パーツの不透明度](motion.md#パーツの不透明度) | |
| [Order of operations in a frame](../../spec/motion.md#order-of-operations-in-a-frame) | [motion.md#フレーム内の適用順](motion.md#フレーム内の適用順)、[motion.md#トラッキング中のモーションの優先](motion.md#トラッキング中のモーションの優先) | 決めたことの経緯 (T0303・T0722 など) は含めない。モーションの優先は MAY |
| [Unknown values](../../spec/motion.md#unknown-values) | [motion.md#カーブ](motion.md#カーブ)、[motion.md#表情](motion.md#表情) | |
| [Determinism](../../spec/motion.md#determinism) | [motion.md#目パチ](motion.md#目パチ) | 乱数は目パチの種付きの生成器だけ |
| [Conformance](../../spec/motion.md#conformance) | [motion.md#互換の基準](motion.md#互換の基準) | |

## JSON Schema と例 (`spec/schema/`・`spec/examples/`)

| 公開仕様 | 開発用 | 備考 |
| --- | --- | --- |
| [spec/schema/](../../spec/schema/README.md) | [editor.md#型の生成](../editor.md#型の生成) | `Model`・`Module`・`PhysicsSettings`・`Motion`・`Expression` の JSON Schema。`ugoe_model::spec_files` が Rust の型から生成する (`just gen`) |
| [spec/examples/](../../spec/examples/README.md) | [testing.md#公開仕様の例](../testing.md#公開仕様の例) | 例のモデル (`model.json` はフィクスチャから生成) と、手で導いた期待値 (`expected.json`)。ランタイムが満たすことを `ugoe-format` の `tests/spec_examples.rs` で確かめる。ファイルの例 (`bundles/`) は `tests/spec_bundles.rs` で参照実装の読み込みと照合する |

## まだ公開仕様にない文書

| `docs/spec/` | 公開仕様 | タスク |
| --- | --- | --- |
| [keyform-inference.md](keyform-inference.md) | 含めない (ツールの内部。結果のデータは `spec/model.md` の Authoring data) | — |

## 表の行にない節

`docs/spec/` の節のうち、上の表の行で対応を示さないもの (公開仕様に含めないか、別の節にまとめて書いたもの) です。

- [model.md#JSON と MessagePack での表現](model.md#json-と-messagepack-での表現): 英語版の [Data conventions](../../spec/model.md#data-conventions) と各節の表の要点
- [model.md#予定されているデータ](model.md#予定されているデータ): 実装の予定 (公開仕様は実装したものだけを書く)
- [file-formats.md#プロジェクト (`.ugoeproj/`)](file-formats.md#プロジェクト-ugoeproj): エディタのプロジェクト形式は含めない (ADR 0021)
- [modules.md#旧ランタイムでの扱い](modules.md#旧ランタイムでの扱い): 英語版では [bundle.md#Baseline features](../../spec/bundle.md#baseline-features) の `modules` の行

## 同期の検査

`just spec` が次を検査します (`scripts/check-md-links.mjs docs spec` と `scripts/check-spec.mjs`)。`just ci` と CI の `web` ジョブに含まれます。

- **リンク:** `docs/` と `spec/` の Markdown の相対リンクとアンカーの先があること。
- **例とスキーマ:** `spec/examples/` の JSON (`model.json`・`module.json`・`*.physics.json`・`*.motion.json`・`*.expression.json`) が `spec/schema/` の JSON Schema を満たすこと (ajv、draft 2020-12)。
- **型とコード:** `spec/model.md` の型の説明が、生成した JSON Schema (= Rust の型) と一致すること。
  - 構造体: 「`` `TypeName`: ``」(または「`` `TypeName` (…): ``」など) の直後の `| Field |` の表の 1 列目と、スキーマの `properties`。
    表のない型は、その段落に書いたフィールド名 (`` `min` `` など) と比べます。
  - enum: 値の表 (`| Value |`)・タグの表 (`` | `type` | ``)・段落の中の値の一覧と、スキーマの値。タグ付きの enum は、
    `Further fields` の列にコードのフィールドがすべて書かれていること。
  - スキーマの型は、どれも `spec/model.md` に説明があること (`vec2`・`rgb` と、タグの表の中で説明する型を除く)。
- **機能と未知の値:** 機能 ID の表 (`spec/bundle.md`・[file-formats.md#機能とフォールバックの一覧](file-formats.md#機能とフォールバックの一覧)) が
  `ugoe_format::features::FEATURES` と一致し、未知の値のフォールバックの表の enum が `spec/model.md#unknown-values` と
  [file-formats.md#未知の値の読み込み](file-formats.md#未知の値の読み込み) で一致し、拡張されうる enum をすべて含むこと。
- **対応表 (この文書):**
  - `spec/*.md` はどれもこの文書の左の列に出てきて、その `##` 見出し (`Contents` を除く) は、どれもそれか配下の見出しへのリンクとして左の列にあること。
  - `docs/spec/*.md` はどれもこの文書に出てきて、「まだ公開仕様にない文書」の表にないものは、`##` 見出しがどれも右の列か「表の行にない節」にあること。
- **文書の一覧:** `spec/README.md` の Documents の表が `spec/*.md` と一致し、`spec/schema/README.md` と `spec/examples/README.md` があること。

検査が失敗したら、実装 (コード) を正として両方の文書を直します。新しい節を足したらこの文書に行を足します。
