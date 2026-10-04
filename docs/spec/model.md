# モデル仕様

対応するコードは `crates/ugoe-model`。型を変えたら同じ PR でこの文書も更新してください。
英語の公開仕様 [`spec/model.md`](../../spec/model.md) も同じ PR で直します (対応は [public-spec.md](public-spec.md)。食い違ったら英語版が正)。

## JSON と MessagePack での表現

すべてのフィールドの型・省略時の値・必須かどうかの一覧は、英語の公開仕様 ([Data conventions](../../spec/model.md#data-conventions) と各節の表) にあります。要点:

- `#[serde(default)]` のないフィールドは必須です (`Model` の `parameters` `parts` `meshes` `textures`、各要素の `id` `name`、キーフォームの数値など)。
- `Option` のフィールドは、省略と `null` のどちらも「値なし」として読みます。書き出しでは、`skip_serializing_if` のないもの (`Part::parent`・`Mesh::parent`・`Parameter::mirror` など) を `null` として書きます。
- `Option` でない型のフィールドに `null` を置くとエラーです。省略できる (既定値のある) フィールドでも、`null` は既定値の意味になりません。
- 1 つのオブジェクトに同じフィールドを 2 回書きません。知っているフィールドが重なっていれば、読み込みはエラーになります。
- 省略時の値: `MeshKeyform` の `multiply_color` は白、`screen_color` は黒、`mask_strength` は 1。`RotationKeyform` の `reflect_x` / `reflect_y` は `false`。
- `Vec2` は `[x, y]` の配列、`Rgb` は `{ "r", "g", "b" }` のオブジェクトです (3 つとも必須)。メッシュの `indices` は `u16` なので、1 つのメッシュの頂点は 65,536 個までです。
- MessagePack では、`f32` のフィールドは整数や float 64 も読めます (serde の変換)。整数のフィールドに float を置くとエラーです (値が整数の float 32 / float 64 も float です)。
- JSON の整数は小数部や指数を付けずに書きます (`1.0` や `1e0` ではなく `1`)。参照実装 (serde_json) は `1.0` をエラーにします。

## トップレベル

`Model` の構成:

| フィールド | 説明 |
| --- | --- |
| `schema_version` | データのスキーマバージョン ([file-formats.md](file-formats.md#バージョニング)) |
| `meta` | 名前、キャンバスの大きさ、表示時の原点 |
| `parameters` | パラメータの定義 |
| `parameter_groups` | エディタでの表示用グループ。ランタイムの挙動には影響しない |
| `parts` | パーツの階層 |
| `deformers` | デフォーマの階層 |
| `meshes` | メッシュ |
| `glues` | グルー |
| `textures` | テクスチャの参照 (画像データはコンテナ側が持つ)。`name` `width` `height` と、法線マップの画像の名前 `normal_map` ([ライティング](#ライティング)) |
| `hit_areas` | 当たり判定 |
| `pose` | ポーズ (排他的なパーツ切り替え)。省略するとグループなし ([ポーズ](#ポーズ)) |
| `effects` | 目パチ・呼吸・リップシンクの設定。省略すると対象なし ([エフェクト設定](#エフェクト設定)) |
| `avatar` | アバタープロファイル。省略するとなし ([アバタープロファイル](#アバタープロファイル)) |
| `slots` | 着せ替えのスロット。省略するとなし ([スロット](#スロット)) |
| `lighting` | ホストが渡す光源でライティングするか。省略すると `false` ([ライティング](#ライティング)) |

物理 (`physics::PhysicsSettings`)、モーション (`motion::Motion`)、表情 (`expression::Expression`) は `Model` とは別のデータです。バンドルでは同じファイルに入れて配布できます ([file-formats.md](file-formats.md))。

物理のデータの意味は [physics.md](physics.md) にあります。物理の出力 (`PhysicsOutput`) の `blend` (`PhysicsBlend`: `overwrite` / `add`) は、
パラメータへの出力を現在の値と合成する方法です ([physics.md#合成の方法](physics.md#合成の方法))。省略すると `overwrite` で、`overwrite` は書き出しでも省略します。

## ID

- すべての要素は **文字列 ID** (`Id<T>`) で参照します。ID はモデル内で種類ごとに一意です。
- Rust では、ID を持つ要素の `id` とそれを指す参照を、どれも要素の種類ごとの `Id<T>` の別名で表します
  (`ParamId`・`ParameterGroupId`・`PartId`・`DeformerId`・`MeshId`・`GlueId`・`HitAreaId`・`SlotId`・
  `physics::PhysicsRigId`・`physics::ColliderId`)。種類の違う ID を取り違えるとコンパイルエラーになります。
  JSON と TypeScript ではどれも文字列で、形は変わりません。
  - 例外は、指す先の種類が文脈で決まる参照です。キーフォームのミラー元 (`MirrorSource::id`) は格子の持ち主と同じ種類
    (メッシュかデフォーマ) を指すので、`String` のままにします。
- ID は `[A-Za-z0-9_]+` で書くことを推奨します。エディタは新しい要素にこの形式の ID を振ります。
- ランタイムはロード時に ID を連番インデックスに変換します (`CompiledModel`)。評価中に文字列の比較は行いません。
- 配列中の順序には意味があります。
  - `meshes` の順序は、描画順が同じときの前後関係を決めます (後ろにあるほど手前)。
  - 他の配列の順序は、エディタでの表示順です。

## パラメータ

| フィールド | 説明 |
| --- | --- |
| `min` / `max` / `default` | `min <= default <= max`。値は常にこの範囲にクランプされる |
| `kind` | `normal` はキーフォームの補間に使う。`blend_shape` は加算変形 (後述) に使う |
| `mirror` | 左右で対になるパラメータ。オーサリング用で、キーフォームのミラーがどのキーをどのキーに写すかに使う ([キーフォーム推定のデータ#ミラー](#ミラー)) |
| `group` | `parameter_groups` の ID |
| `internal` | 中継用 (内部) のパラメータか。省略時と `false` のときは書き出さない (下記) |

`internal = true` のパラメータは、物理のリグの間で値を受け渡すための中継パラメータ ([physics.md#リグの順序と中継](physics.md#リグの順序と中継)) など、
利用者やアプリが直接動かすことを想定しないパラメータです。値の評価はほかのパラメータと同じで、フラグは次のことにだけ効きます。

- トラッキング (`ugoe_motion::TrackingMap`) は書き込みません。標準の対応表のパラメータでも、`avatar.tracking` の書き込み先でも同じです。
  アバタープロファイルの準拠の検査は、トラッキング・視線の対象が `internal` なら警告します ([avatar-profile.md#準拠の検査](avatar-profile.md#準拠の検査))。
- ビューアのスライダーの一覧では既定で隠します (表示を切り替えられる)。エディタでは一覧に残し、中継用であることが分かるように表示します。
- 構造の検証 (`validate`) では制限しません。キーフォームの軸・物理の入力と出力・モーションなど、どこから参照してもかまいません。
- これを含まない既存のデータもそのまま読めます (`SCHEMA_VERSION` は変えていません)。知らないランタイムは読み飛ばし、通常のパラメータとして扱います。

### 標準パラメータ

> アバタープロファイル ([avatar-profile.md](avatar-profile.md)) は、この標準パラメータに拡張パラメータとトラッキングとの対応表を加えたものです。

`ugoe_model::standard` に定義があります。`ALL` が標準パラメータ、`EXTENDED` がアバタープロファイルの拡張パラメータ (`EyeLSmile` / `EyeRSmile` は両方にある)、`BODY_EXTENDED` が体の拡張パラメータ (`ShoulderLY` / `ShoulderRY`)、`VOWELS` が母音 (`MouthVowelA` 〜 `MouthVowelO`) で、`standard::range(id)` で既定の値域を引けます。モーション・表情・物理プリセット・AI の自動リギングは標準 ID を前提に作ります。

慣例:

- 角度系 (`AngleX/Y/Z`) は -30〜30 度。`AngleX` は正の値でキャラクターから見て左 (画面では右) を向く。`AngleY` は正で上を向く。`AngleZ` は正で時計回りに傾く。
- 開閉系 (`EyeLOpen` など) は 0 が閉じた状態で 1 が開いた状態。既定値は 1。
- `-1〜1` の系統は 0 が中立。

> 未決: 目を見開いた状態 (1 を超える値) を許すか。Cubism では開閉パラメータの最大値を 1 より大きくして「見開き」を表すことがある。

## パーツ

- パーツは木構造です (`parent`)。
- パーツの不透明度はキーフォームで決まり、**祖先パーツの不透明度と掛け合わされます**。
- メッシュとデフォーマは必ず 1 つのパーツに属します (`part`)。メッシュの最終的な不透明度 = メッシュ自身の不透明度 × 所属パーツ (と祖先) の不透明度 × 親デフォーマの不透明度の積。
- デフォーマの `part` は評価に影響しません。デフォーマの不透明度にそのパーツの不透明度は掛からず、デフォーマの下のメッシュも、自分がそのパーツに属さない限り影響を受けません。ツールでパーツとまとめて扱うためのものです。

## デフォーマ

- デフォーマは木構造です (`parent`)。デフォーマの階層はパーツの階層とは独立しています。
- 子 (デフォーマまたはメッシュ) の座標は、**親デフォーマの局所座標系** で表します。変換の式は [evaluation.md](evaluation.md) にあります。
- 種類:
  - **ワープ** (`warp`): `(rows+1)×(cols+1)` 個の制御点を持つ格子。子の局所座標は [0,1]² です。
  - **回転** (`rotation`): 原点・角度・拡大率・反転を持ちます。子の局所座標は、原点を (0,0) とした回転前のピクセル座標です。
- デフォーマのキーフォームは不透明度も持ち、子孫すべてに掛け合わされます。
- オーサリング用のフィールド ([キーフォーム推定のデータ](#キーフォーム推定のデータ)):
  - `mirror` / `mirror_axis`: ミラーで対にするデフォーマと対称軸。
  - ワープの `depth`: 制御点ごとの深度。並びは制御点と同じで、長さは `(rows+1)*(cols+1)`。

## メッシュ

| フィールド | 説明 |
| --- | --- |
| `uvs` | テクスチャ座標 (0〜1、左上が原点)。頂点数を決める |
| `indices` | 三角形の頂点インデックス。長さは 3 の倍数 |
| `keyforms` | `MeshKeyform` の格子 (頂点位置・不透明度・描画順・乗算色・スクリーン色・`mask_strength`) |
| `blend` | 合成モード ([rendering.md](rendering.md#合成モード)) |
| `culling` | 裏面カリング。頂点がキャンバス座標で反時計回りに並ぶ三角形を表とする |
| `masks` | マスクの参照 `MaskRef { mesh, invert }` の列。`invert = true` は反転マスク |
| `mask_mode` | 通常マスクの結合方法。`union` (既定、和集合) または `intersection` (共通部分) |
| `mask_only` | `true` なら色としては描画せず、マスクとしてだけ働く (既定 `false`) |
| `mirror` / `mirror_axis` | ミラーで対にするメッシュと対称軸 (オーサリング用、[キーフォーム推定のデータ](#キーフォーム推定のデータ)) |
| `depth` | 頂点ごとの深度 (オーサリング用)。長さは `uvs` と同じ |
| `lit` | ライティングを受けるか (既定 `true`)。`Model::lighting` が `true` のときだけ効く ([ライティング](#ライティング)) |

- マスクの意味論 (カバレッジ、入れ子の継承、`mask_strength`) は [rendering.md#マスク](rendering.md#マスク) にあります。
  判断の経緯は [ADR 0009](../adr/0009-clipping-mask-semantics.md)。
- `MaskRef::invert` は省略できます (既定 `false`)。
- `MeshKeyform::mask_strength` (0〜1、既定 1) はマスクの効き具合です。マスクを持たないメッシュでは意味を持ちません。
  範囲外の値も有限値なら許し、ランタイムが補間後に 0〜1 にクランプします。
- 頂点数とトポロジー (`uvs` と `indices`) はキーフォーム間で共通です。キーフォームが持つのは位置だけです。
- 描画順は 0〜1000 を慣例とします (既定値は 500)。

### ブレンドシェイプ

`kind = blend_shape` のパラメータに結び付いたキーフォームは、通常の補間結果に **差分を加算** します。
ARKit のブレンドシェイプ (52 種) のように、互いに独立した表情の差分を重ねるための仕組みです。
補間の式は [evaluation.md](evaluation.md#6-ブレンドシェイプ) にあります。

差分は **要素の格子 (`KeyformGrid`) の `blend_shapes` に、同じキーフォームの型の格子の列として持ちます** (ブレンドシェイプ格子)。

```json
"keyforms": {
  "bindings": [{ "parameter": "AngleX", "keys": [-30, 0, 30] }],
  "keyforms": [ ... ],
  "blend_shapes": [
    { "bindings": [{ "parameter": "mouthSmileLeft", "keys": [0, 1] }], "keyforms": [ 基準の形, 笑った形 ] },
    { "bindings": [{ "parameter": "jawOpen", "keys": [0, 1] }], "keyforms": [ 基準の形, 口を開いた形 ] }
  ]
}
```

- セルは差分ではなく **形そのもの** (通常のセルと同じ `MeshKeyform` などで、同じ親座標系) を持ちます。ランタイムは既定値での形との差を取ります。
  そのため、既定値のキーのセルを基準の形 (通常の格子の既定値での形) にしておけば、別のキーのセルに目的の形をそのまま置けます。
- 対応する要素はメッシュ・ワープデフォーマ・回転デフォーマです。パーツとグルーの格子は `blend_shapes` を持てません (検証エラー)。
- 差分を持つのは数値のフィールドだけです (メッシュの頂点位置・不透明度・描画順・乗算色・スクリーン色・`mask_strength`、ワープの制御点・不透明度、回転の原点・角度・拡大率・不透明度)。回転の `reflect_x` / `reflect_y` は通常の格子の値を使います。
- ブレンドシェイプ格子自身は `blend_shapes` を持ちません (入れ子にしない)。`origins` などのオーサリング用のフィールドは持てますが、推定 ([keyform-inference.md](keyform-inference.md)) はブレンドシェイプ格子を扱いません。
- 格子の種類は置き場所で決まり、パラメータの `kind` は検証とエディタでの振り分けに使います。通常の格子の軸は `blend_shape` 以外、ブレンドシェイプ格子の軸は `blend_shape` のパラメータにします。未知の種類 (`Unknown`) のパラメータはどちらにも置けます。
- 1 つの要素の中で、同じパラメータを 2 つのブレンドシェイプ格子の軸にはできません。エディタはパラメータで格子を選ぶためです。軸が 2 本以上の格子 (2 つのブレンドシェイプの組み合わせの補正など) は作れます。
- `blend_shapes` は省略でき、空のときは書き出しません。これを含まない既存のデータもそのまま読めます (`SCHEMA_VERSION` は変えていません)。知らないランタイムは読み飛ばし、ブレンドシェイプのパラメータが既定値のときの形で表示します (機能 `deform.blend_shape`、[file-formats.md](file-formats.md#機能とフォールバックの一覧))。

判断の経緯 (T0307): 差分専用の型を作る案は、型ごとに差分の型が増え、エディタの格子の操作 (キーの追加・セルの置き換え・頂点数の変更) を作り直す必要がありました。
既存の格子の型に格子の列を持たせる形なら、格子の操作と検証をそのまま使え、形そのものを持つので作り方も通常のセルと同じです。
差分を「既定値での形との差」にすることで、既定値ではモデルの形が変わらないことがデータによらず保証されます。

## グルー (M2)

2 つのメッシュの頂点ペアを、重み `weight` の位置に引き寄せます。引き寄せの強さはキーフォーム (`intensity`, 0〜1) で決まります。式は [evaluation.md](evaluation.md#5-グルー) にあります。

## ポーズ

`pose::Pose`。互いに排他的なパーツ (腕の差分など) をグループにまとめ、各グループで表示中のパーツを 1 つに保ちます。再生の挙動は [motion.md](motion.md#エフェクト) にあります。

| フィールド | 説明 |
| --- | --- |
| `fade_duration` | 表示パーツを切り替えるときのクロスフェード時間 (秒)。既定値は 0.5 |
| `groups[].entries[].part` | グループの要素となるパーツ |
| `groups[].entries[].links` | `part` と同じ不透明度に追従するパーツ (袖など)。省略可 |

- 同じパーツは、ポーズ全体 (すべてのグループの `part` と `links`) で 1 回しか使えません。
- `groups` が空ならポーズは何もしません。

## エフェクト設定

`effects::Effects`。目パチ (`eye_blink`)・呼吸 (`breath`)・リップシンク (`lip_sync`) の対象パラメータと時間を持ちます。挙動と既定値は [motion.md](motion.md#エフェクト) にあります。

- 省略したフィールドは既定値になります。既定値は **対象パラメータなし** (何もしない) で、時間などの値だけが仕様の既定値を持ちます。標準パラメータを対象にした推奨設定は `Effects::standard()` で作ります。
- `pose` と `effects` は `#[serde(default)]` 付きで追加したフィールドなので、これらを含まない既存のデータもそのまま読めます (`SCHEMA_VERSION` は変えていません)。バンドルでは `MODL` チャンクの `Model` に含まれます。

## アバタープロファイル

`avatar::AvatarProfile` (`Model::avatar`)。内容と意味は [avatar-profile.md](avatar-profile.md) にあります。

- `Option` で、省略するとアバタープロファイルなしです。`None` のときは書き出しません。`#[serde(default)]` 付きで追加したフィールドなので、これを含まない既存のデータもそのまま読めます (`SCHEMA_VERSION` は変えていません)。バンドルでは `MODL` チャンクの `Model` に含まれます。
- `AvatarProfile` と `AvatarMeta` のフィールドは省略でき、省略すると既定値になります。許諾の既定値は最も制限の強い値です。
- ランタイムは `avatar` を読み飛ばせます (機能 `avatar.profile`、[file-formats.md](file-formats.md#機能とフォールバックの一覧))。そのため、構造の検証 (`validate`) は `avatar` を検査しません。準拠の検査は `avatar::check::check_profile` (`ugoe validate --profile avatar`) で行います ([avatar-profile.md#準拠の検査](avatar-profile.md#準拠の検査))。

## スロット

`module::Slot` (`Model::slots`)。着せ替えのモジュールの取り付け位置で、内容と合成の規則は [modules.md](modules.md) にあります。
モジュール (`module::Module`) は `Model` とは別のデータです。

- `Vec` で、省略すると空です。空のときは書き出しません。これを含まない既存のデータもそのまま読めます (`SCHEMA_VERSION` は変えていません)。バンドルでは `MODL` チャンクの `Model` に含まれます。
- 知らないランタイムはスロットを無視し、本体だけを表示します (機能 `modules`、[file-formats.md](file-formats.md#機能とフォールバックの一覧))。
- `Slot` の `part`・`colliders`・`pose_parts` (T0121) は省略でき、省略したときは書き出しません。`Module` の `pose` も同じです。どれも既存のデータをそのまま読めます (`SCHEMA_VERSION` は変えていません)。
- アバタープロファイルの `slots` は、ここにあるスロットの ID を参照します。

## ライティング

法線マップとライティングの意味は [rendering.md#ライティング](rendering.md#ライティング)、判断の経緯は [ADR 0015](../adr/0015-lighting.md) にあります。

- `TextureRef::normal_map` (`Option<String>`) はそのテクスチャの法線マップの画像の名前です。省略すると法線マップなしで、法線を `(0, 0, 1)` として扱います。
  法線マップはテクスチャと同じ UV 空間の接空間法線です。画像はコンテナ側が持ち、バンドルでは `NRML` チャンクに、プロジェクトでは `textures/<normal_map>.png` に入ります ([file-formats.md](file-formats.md#バンドル-ugoe)、[名前と読み込み](file-formats.md#名前と読み込み))。
- `Model::lighting` (既定 `false`) と `Mesh::lit` (既定 `true`) の両方が `true` のメッシュだけがライティングを受けます。モデル全体で有効にしてから、受けないメッシュ (目のハイライトなど) を外す使い方を想定しています。
- どれも既定値のときは書き出しません。これを含まない既存のデータもそのまま読めます (`SCHEMA_VERSION` は変えていません)。
- 光源 (`Lighting { ambient, lights }`) はモデルのデータではなく、ホストがフレームごとにレンダラへ渡します。
- 知らないランタイムはライティングなしで描きます (機能 `lighting.normal_map`、[file-formats.md](file-formats.md#機能とフォールバックの一覧))。

## キーフォームの補間

`KeyformGrid::interpolation` (`KeyformInterpolation`) は格子のキーの間の補間の種類です ([evaluation.md#曲線補間](evaluation.md#曲線補間))。

| 値 | 意味 |
| --- | --- |
| `linear` | 既定。各軸で隣り合う 2 つのキーの間を直線で補間する (多重線形) |
| `smooth` | キーが 3 つ以上の軸を、キーを通る滑らかな曲線で補間する (Cubism の「拡張補間」に相当)。形 (頂点・制御点・回転の原点・角度・拡大率) だけに効き、不透明度・描画順・色などは線形のまま |

```json
"keyforms": { "bindings": [ ... ], "keyforms": [ ... ], "interpolation": "smooth" }
```

- 格子ごとに持ちます。ブレンドシェイプ格子もそれぞれ持てます。パーツとグルーの格子は形を持たないので、`smooth` でも線形で補間します (エディタは設定を拒否する)。
- 省略でき、`linear` のときは書き出しません。これを含まない既存のデータは `linear` として読めます (`SCHEMA_VERSION` は変えていません)。未知の値は `linear` として扱います (`effective()`)。

## キーフォーム推定のデータ

キーフォームの推定 ([keyform-inference.md](keyform-inference.md)) のためのデータです。
公開仕様のフィールドの表で「Authoring data」と書いたものがこれに当たります (パラメータの `mirror` も含む)。
**すべてオーサリング用で、ランタイムは読み飛ばします** (評価・再生・物理・描画の結果に影響しない)。ツールは理解できなければ落としてもかまいません。
どのフィールドも省略できるか、`null` を取れます。省略できるものは既定値のときは書き出しません。`null` を取れるもの (`Option`) は、
値がなければ `null` と書くか省略します (ほかの `Option` のフィールドと同じ。[JSON と MessagePack での表現](#json-と-messagepack-での表現))。これらを含まない既存のデータもそのまま読めます (`SCHEMA_VERSION` は変えていません)。

### キーフォーム格子

`KeyformGrid` (メッシュ・デフォーマ・パーツ・グルーのすべての格子) に次のフィールドがあります。

| フィールド | 説明 |
| --- | --- |
| `origins` | セルごとの由来の列 (下の表)。並びは `keyforms` と同じ。空 (既定) ならすべて `authored` で、空でなければ長さは `keyforms` と同じ |
| `inference` | 方法を記録していない `derived` のセルを作り直すときの推定の設定。省略すると加算合成 |
| `mirror_source` | `mirror` のセルのミラー元: `id` (同じ種類の要素の ID。自分自身も可)、`axis` (明示した対称軸)・`tolerance` (明示した許容距離) は省略できる |

由来 (`KeyformOrigin`) は `authored` と、推定で作ったセルの由来 (推定の方法) です ([keyform-inference.md#セルの由来](keyform-inference.md#セルの由来))。

| 由来 | 意味 |
| --- | --- |
| `authored` | 人が作ったセル |
| `derived` | 推定で作ったセル (方法の記録なし)。格子の `inference` の方法で作ったものとして扱う。T0614 より前のファイルと `mark_keyform_derived` がこの値を書く |
| `additive` / `depth_rotation` / `mirror` | 加算合成・深度からの回転・ミラーで作ったセル。ミラー元は `mirror_source` |

`inference` は `method` で方法を選ぶタグ付きの値です。

| `method` | 設定 |
| --- | --- |
| `additive` | 加算合成。設定なし |
| `depth_rotation` | 深度からの回転。`angle_scale` (既定 1)、`pivot` (回転の中心、対象と同じ親座標系)、`pivot_depth` (中心の深度、既定 0)、`yaw` / `pitch` (縦軸・横軸まわりの回転に使うパラメータ。省略するとその向きには回さない) |

```json
"keyforms": {
  "bindings": [ ... ],
  "keyforms": [ ... ],
  "origins": ["depth_rotation", "authored", "mirror"],
  "inference": { "method": "depth_rotation", "pivot": [128, 128], "yaw": "AngleX" },
  "mirror_source": { "id": "Face" }
}
```

### ミラー

- `Mesh::mirror` / `Deformer::mirror` は、左右で対にする相手の ID です。省略 (既定) と自分自身は「自分と対応する」(左右対称な 1 つの要素) を表します。
- 対は **双方向に張ります**。`A.mirror = B` なら `B.mirror = A` でなければなりません。
- `Parameter::mirror` は、キーフォームをミラーするときに、どのパラメータのどのキーがどのキーに対応するかを表します
  ([keyform-inference.md](keyform-inference.md) の対応の表)。`A.mirror = B` (`B ≠ A`) なら `A` のキーは `B` の同じ値のキーに対応します
  (`EyeLOpen` ↔ `EyeROpen`)。`A.mirror = A` なら自分自身が相手で、符号を反転したキーに対応します (`AngleX` の 30 は −30)。
  省略は「ミラーで変わらないパラメータ」で、キーは同じパラメータの同じ値のキーに対応します (`MouthOpenY` の 1 は 1)。
  メッシュ・デフォーマでは省略と自分自身が同じ意味ですが、パラメータでは違います (省略は同じ値、自分自身は符号を反転した値)。
  対は双方向に張るべきですが、メッシュ・デフォーマと違って検証はこれを検査しません (参照先が存在することだけを検査する)。
- `mirror_axis` は対称軸 (縦の直線) の x 座標で、その要素の頂点・制御点・原点と同じ **親座標系** で表します。省略すると親で決まる既定値になります ([keyform-inference.md#対称軸](keyform-inference.md#対称軸))。左右の対の親が異なる場合は、この値をキャンバス座標へ写して使い、省略するとキャンバスの原点になります ([keyform-inference.md#親が異なる対](keyform-inference.md#親が異なる対))。

### 深度

- `Mesh::depth` は頂点ごと、`WarpDeformer::depth` は制御点ごとの深度 (キャンバスのピクセル単位、手前が正) です。
- 深度は基準のセルの形に対して与えます。省略 (既定) は深度なしです。

## 未知の値

拡張されうる enum (`BlendMode` `MaskMode` `DeformerKind` `ParameterKind` `KeyformOrigin` `KeyformInference` `KeyformInterpolation`、
アバタープロファイルの `AvatarPermission` `CommercialUsage` `CreditNotation` `Modification` `TrackingSource`、
モーション・表情・物理の `CurveTarget` `Segment` `ExpressionOp` `PhysicsSource` `PhysicsTarget` `PhysicsBlend` `ColliderShape`) は `Unknown` のバリアントを持ち、
新しい版のファイルにある知らないタグを `Unknown` として読みます。元のタグと中身は捨てます。

- `Unknown` にはその種類のフォールバックを適用します。一覧は [file-formats.md#未知の値の読み込み](file-formats.md#未知の値の読み込み)。
  値で置き換えられる種類は `effective()` (`BlendMode` `MaskMode` `ParameterKind` `KeyformInterpolation` `PhysicsBlend` と許諾の 4 つ) や `KeyformGrid::origin` がフォールバック後の値を返します。
- `Unknown` は **書き出せません** (シリアライズがエラーになる)。別の値として黙って保存しないためです。
- 検証 (`validate`) は `Unknown` をエラーにしません。場所は `ugoe_model::unknown` (`in_model` など) で調べられます。

## 予定されているデータ

次のデータは仕様が決まっていて、まだ実装していません。詳細はそれぞれの仕様書にあります。

| データ | 内容 | 仕様 | タスク |
| --- | --- | --- | --- |
| (まだない) | | | |

## 検証

`ugoe_model::validate::validate` は構造の整合性を検査し、見つけたエラーをすべて返します。ランタイムのコンパイル、ファイルの読み込み、AI の提案の受け入れ時に必ず呼びます。

検査項目:

- ID が種類 (パラメータ・パラメータグループ・パーツ・デフォーマ・メッシュ・グルー・当たり判定) ごとに一意であること
- すべてのキーフォーム格子 (メッシュ・パーツ・デフォーマ・グルー) について:
  - キーフォーム数 = 各軸のキー数の積
  - 各軸にキーが 1 つ以上あること
  - 各軸のキーが昇順で、重複がないこと (NaN を含まないこと)
  - 軸のパラメータが存在し、同じ格子で同じパラメータを 2 回使っていないこと
  - `origins` が空か、長さがキーフォーム数と同じであること
  - 深度からの回転の設定: `angle_scale`・`pivot`・`pivot_depth` が有限値で、`yaw` / `pitch` のパラメータが存在し、その格子の軸であること
  - `mirror_source` の `id` が、格子を持つ要素と同じ種類の要素に存在すること (`UnknownReference`、`field` は `keyforms.mirror_source`)。`axis` と `tolerance` は指定されていれば有限値であること
- メッシュのインデックスが 3 の倍数で、範囲内にあること
- キーフォームの頂点数 = `uvs` の長さ
- ワープのキーフォームの制御点数 = `(rows+1)*(cols+1)`
- ブレンドシェイプ格子 ([ブレンドシェイプ](#ブレンドシェイプ)):
  - 上の格子の検査と、頂点数・制御点数・`mask_strength` の検査をブレンドシェイプ格子にも行う (`mask_strength` の場所の添字は、通常の格子のセルの後にブレンドシェイプ格子のセルを宣言順に続けて数える)
  - 通常の格子の軸が `blend_shape` のパラメータでなく、ブレンドシェイプ格子の軸が通常 (`normal`) のパラメータでないこと (`ParameterKindMismatch`)
  - ブレンドシェイプ格子に軸が 1 本以上あり、入れ子の `blend_shapes` を持たないこと、パーツとグルーの格子が `blend_shapes` を持たないこと (`InvalidBlendShape`)
  - 同じパラメータが、1 つの要素の 2 つのブレンドシェイプ格子の軸になっていないこと (`DuplicateBinding`)
- ワープの `rows` と `cols` がどちらも 1 以上であること
- 参照先が存在すること: メッシュの `part` / `parent` (デフォーマ) / `masks`、デフォーマの `part` / `parent`、パーツの `parent`、パラメータの `mirror` / `group`、グルーの `mesh_a` / `mesh_b`、当たり判定の `mesh`、ポーズの `part` / `links`、エフェクト (目パチ・呼吸・リップシンク) の対象パラメータ
- メッシュの `texture` が `textures` の範囲内にあること
- グルーの頂点ペアのインデックスが、それぞれのメッシュの頂点数の範囲内にあること
- パーツとデフォーマの親子関係に循環がないこと (自分自身を親にするのも循環)
- マスクの参照が循環しないこと (自分自身をマスクにする場合を含む)
- メッシュのキーフォームの `mask_strength` が有限値であること
- メッシュとデフォーマの `mirror`: 参照先が同じ種類の中に存在し、対が双方向であること (`A.mirror = B` なら `B.mirror = A`。自分自身との対は常に正しい)。`mirror_axis` は有限値
- `depth` の長さがメッシュの頂点数・ワープの制御点数と同じで、すべて有限値であること
- パラメータで `min <= default <= max` が成り立つこと
- 同じパーツがポーズ全体で 2 回以上現れないこと (`part` と `links` を合わせて数える)
- スロット: ID が一意で、`anchor` (デフォーマ)・`replaces` (パーツ)・`parameters` (パラメータ)・`mask_sources` (メッシュ)・`part` (パーツ)・`pose_parts` (ポーズの要素のパーツ。`target_kind` は `pose entry`) の参照先が存在し (`UnknownReference`、`kind` は `slot`)、`draw_order` の `min` と `max` が有限値で `min <= max` であること
- 時間と周期の値: ポーズの `fade_duration` と目パチの `closing` / `closed` / `opening` は 0 以上、目パチの `interval` と呼吸の `cycle` は 0 より大きいこと (いずれも有限値)

### 物理の検証

物理 (`PhysicsSettings`) は `Model` と別のデータなので、`ugoe_model::validate::validate_physics(&physics, &model)` で参照先のモデルと合わせて検査します。モデル自体の検査は含みません。

- リグと衝突体の ID が、それぞれの中で一意であること
- 参照先が存在すること: 入力のパラメータ、出力先 (パラメータ・デフォーマ・メッシュ)、リグの `colliders` (衝突体)・`frame` (デフォーマ)、衝突体の `attach` (デフォーマ)、モデルのスロットの `colliders` (衝突体。`kind` は `slot`。本体に物理の設定が無いまま合成したモデルではスロットの `colliders` は空になる。[modules.md#合成](modules.md#合成))
- 出力先のデフォーマの種類: `deformer_angle` / `deformer_offset` は回転デフォーマ、`vertices` の `warp` はワープデフォーマであること (`DeformerKindMismatch`。種類が未知のデフォーマは検査しない)
- `vertices` の `weights` の長さが、メッシュの頂点数・ワープの制御点数と同じで (`WeightCount`)、すべて 0〜1 であること
- 粒子の `collision_radius` と衝突体の `radius` が 0 以上の有限値で、衝突体の位置が有限値であること
- 未知の出力先と形状 (`Unknown`) はエラーにしない
