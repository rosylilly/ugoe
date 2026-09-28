# モデル仕様

対応するコードは `crates/ugoe-model`。型を変えたら同じ PR でこの文書も更新してください。

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
| `textures` | テクスチャの参照 (画像データはコンテナ側が持つ) |
| `hit_areas` | 当たり判定 |
| `pose` | ポーズ (排他的なパーツ切り替え)。省略するとグループなし ([ポーズ](#ポーズ)) |
| `effects` | 目パチ・呼吸・リップシンクの設定。省略すると対象なし ([エフェクト設定](#エフェクト設定)) |
| `avatar` | アバタープロファイル。省略するとなし ([アバタープロファイル](#アバタープロファイル)) |

物理 (`physics::PhysicsSettings`)、モーション (`motion::Motion`)、表情 (`expression::Expression`) は `Model` とは別のデータです。バンドルでは同じファイルに入れて配布できます ([file-formats.md](file-formats.md))。

## ID

- すべての要素は **文字列 ID** (`Id<T>`) で参照します。ID はモデル内で種類ごとに一意です。
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
| `mirror` | 左右で対になるパラメータ。エディタのミラー編集と AI の自動設定に使う |
| `group` | `parameter_groups` の ID |

### 標準パラメータ

> アバタープロファイル ([avatar-profile.md](avatar-profile.md)) は、この標準パラメータに拡張パラメータとトラッキングとの対応表を加えたものです。

`ugoe_model::standard` に定義があります。`ALL` が標準パラメータ、`EXTENDED` がアバタープロファイルの拡張パラメータ (`EyeLSmile` / `EyeRSmile` は両方にある)、`VOWELS` が母音 (`MouthVowelA` 〜 `MouthVowelO`) で、`standard::range(id)` で既定の値域を引けます。モーション・表情・物理プリセット・AI の自動リギングは標準 ID を前提に作ります。

慣例:

- 角度系 (`AngleX/Y/Z`) は -30〜30 度。`AngleX` は正の値でキャラクターから見て左 (画面では右) を向く。`AngleY` は正で上を向く。`AngleZ` は正で時計回りに傾く。
- 開閉系 (`EyeLOpen` など) は 0 が閉じた状態で 1 が開いた状態。既定値は 1。
- `-1〜1` の系統は 0 が中立。

> 未決: 目を見開いた状態 (1 を超える値) を許すか。Cubism では開閉パラメータの最大値を 1 より大きくして「見開き」を表すことがある。

## パーツ

- パーツは木構造です (`parent`)。
- パーツの不透明度はキーフォームで決まり、**祖先パーツの不透明度と掛け合わされます**。
- メッシュとデフォーマは必ず 1 つのパーツに属します (`part`)。メッシュの最終的な不透明度 = メッシュ自身の不透明度 × 所属パーツ (と祖先) の不透明度 × 親デフォーマの不透明度の積。

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

- マスクの意味論 (カバレッジ、入れ子の継承、`mask_strength`) は [rendering.md#マスク](rendering.md#マスク) にあります。
  判断の経緯は [ADR 0009](../adr/0009-clipping-mask-semantics.md)。
- `MaskRef::invert` は省略できます (既定 `false`)。
- `MeshKeyform::mask_strength` (0〜1、既定 1) はマスクの効き具合です。マスクを持たないメッシュでは意味を持ちません。
  範囲外の値も有限値なら許し、ランタイムが補間後に 0〜1 にクランプします。
- 頂点数とトポロジー (`uvs` と `indices`) はキーフォーム間で共通です。キーフォームが持つのは位置だけです。
- 描画順は 0〜1000 を慣例とします (既定値は 500)。

### ブレンドシェイプ (M3)

`kind = blend_shape` のパラメータに結び付いたキーフォームは、通常の補間結果に **差分を加算** します。
補間の式は [evaluation.md](evaluation.md#ブレンドシェイプ) にあります。

> 未決: ブレンドシェイプの差分をどう持つか (`MeshKeyform` を流用して既定形状との差を取るか、差分専用の型を用意するか)。T0303 の着手前に決める。

## グルー (M2)

2 つのメッシュの頂点ペアを、重み `weight` の位置に引き寄せます。引き寄せの強さはキーフォーム (`intensity`, 0〜1) で決まります。式は [evaluation.md](evaluation.md#グルー) にあります。

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

## キーフォーム推定のデータ

キーフォームの推定 ([keyform-inference.md](keyform-inference.md)) のためのデータです。
**すべてオーサリング用で、ランタイムは読み飛ばします** (評価の結果に影響しない)。
どのフィールドも省略でき、既定値のときは書き出しません。これらを含まない既存のデータもそのまま読めます (`SCHEMA_VERSION` は変えていません)。

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
- `mirror_axis` は対称軸 (縦の直線) の x 座標で、その要素の頂点・制御点・原点と同じ **親座標系** で表します。省略すると親で決まる既定値になります ([keyform-inference.md#対称軸](keyform-inference.md#対称軸))。左右の対の親が異なる場合は、この値をキャンバス座標へ写して使い、省略するとキャンバスの原点になります ([keyform-inference.md#親が異なる対](keyform-inference.md#親が異なる対))。

### 深度

- `Mesh::depth` は頂点ごと、`WarpDeformer::depth` は制御点ごとの深度 (キャンバスのピクセル単位、手前が正) です。
- 深度は基準のセルの形に対して与えます。省略 (既定) は深度なしです。

## 未知の値

拡張されうる enum (`BlendMode` `MaskMode` `DeformerKind` `ParameterKind` `KeyformOrigin` `KeyformInference`、
アバタープロファイルの `AvatarPermission` `CommercialUsage` `CreditNotation` `Modification` `TrackingSource`、
モーション・表情・物理の `CurveTarget` `Segment` `ExpressionOp` `PhysicsSource`) は `Unknown` のバリアントを持ち、
新しい版のファイルにある知らないタグを `Unknown` として読みます。元のタグと中身は捨てます。

- `Unknown` にはその種類のフォールバックを適用します。一覧は [file-formats.md#未知の値の読み込み](file-formats.md#未知の値の読み込み)。
  値で置き換えられる種類は `effective()` (`BlendMode` `MaskMode` `ParameterKind` と許諾の 4 つ) や `KeyformGrid::origin` がフォールバック後の値を返します。
- `Unknown` は **書き出せません** (シリアライズがエラーになる)。別の値として黙って保存しないためです。
- 検証 (`validate`) は `Unknown` をエラーにしません。場所は `ugoe_model::unknown` (`in_model` など) で調べられます。

## 予定されているデータ

次のデータは仕様が決まっていて、まだ実装していません。詳細はそれぞれの仕様書にあります。

| データ | 内容 | 仕様 | タスク |
| --- | --- | --- | --- |
| 物理の出力先・衝突体・重力の基準 | 動的物理 | [physics.md#動的物理](physics.md#動的物理-揺れ用パラメータのいらない物理) | T0304 |
| `Model::slots` | 着せ替えのスロット | [modules.md](modules.md) | T0111 |
| `TextureRef::normal_map`、`Model::lighting`、`Mesh::lit` | 法線マップとライティング | [rendering.md#ライティング](rendering.md#ライティング) | T0112 |

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
- 時間と周期の値: ポーズの `fade_duration` と目パチの `closing` / `closed` / `opening` は 0 以上、目パチの `interval` と呼吸の `cycle` は 0 より大きいこと (いずれも有限値)
