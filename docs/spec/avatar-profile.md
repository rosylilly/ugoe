# ugoe アバタープロファイル仕様 (バージョン 0)

判断の経緯は [ADR 0012](../adr/0012-avatar-profile.md)。

> 実装: データ定義は `ugoe_model::avatar`、検証は `ugoe validate --profile avatar`、トラッキングの対応付けは `ugoe_motion::tracking` です。
> この仕様は公開を前提に、ugoe の実装から独立して読めるように書きます。1.0 までは内容が変わりえます。
> この仕様書のライセンスは MIT です ([ADR 0018](../adr/0018-licensing-and-1.0-scope.md))。

アバタープロファイルは、2D アバターをどのアプリでも同じように動かすための約束です。
モデルは `Model.avatar` にプロファイルの情報を持ち、アプリはそれを読んでトラッキングや表示を接続します。

## 準拠レベル

| レベル | 名前 | 必要なもの |
| --- | --- | --- |
| 1 | 基本 | メタデータ、頭の向き (`AngleX` `AngleY` `AngleZ`)、目の開閉 (`EyeLOpen` `EyeROpen`)、口の開き (`MouthOpenY`) |
| 2 | 標準 | レベル 1 に加えて、視線 (`EyeBallX` `EyeBallY`)、眉 (`BrowLY` `BrowRY`)、口の形 (`MouthForm`)、体の向き (`BodyAngleX` `BodyAngleY` `BodyAngleZ`)、呼吸 (`Breath`) |
| 3 | 表情 | レベル 2 に加えて、下の「拡張パラメータ」のうち少なくとも 8 つ |

モデルは準拠を主張するレベルを `Model.avatar.level` (整数、既定 1) に書きます。検証ツール (`ugoe validate --profile avatar`) は、主張したレベルに必要なパラメータが揃っているかを検査します ([準拠の検査](#準拠の検査))。
`Model.avatar` を持たないモデルは、どのレベルにも準拠しません。

## パラメータ

### 向きと左右の約束

- **左右 (`L` / `R`) はキャラクター自身から見た左右です。** 画面に正対したキャラクターの `EyeL` は、画面の右側にあります。
- `AngleX` は正の値でキャラクターから見て左 (画面では右) を向きます。`AngleY` は正で上、`AngleZ` は正で時計回りに傾きます (Y 下向きの画面で見て)。
- 開閉系は 0 が閉じた状態、1 が開いた状態です。`-1〜1` の系統は 0 が中立です。

### 標準パラメータ

`ugoe_model::standard` のパラメータ ([model.md#標準パラメータ](model.md#標準パラメータ)) を使います。

### 拡張パラメータ (レベル 3)

| ID | 値域 (既定) | 意味 |
| --- | --- | --- |
| `EyeLWide` / `EyeRWide` | 0〜1 (0) | 目の見開き |
| `EyeLSmile` / `EyeRSmile` | 0〜1 (0) | 目を細める (笑い目) |
| `BrowInnerUp` | 0〜1 (0) | 眉の内側を上げる (困り眉) |
| `BrowLAngle` / `BrowRAngle` | -1〜1 (0) | 眉の傾き (正で外側が上がる) |
| `MouthX` | -1〜1 (0) | 口の左右の位置 (正でキャラクターの左) |
| `JawX` | -1〜1 (0) | あごの左右の位置 (正でキャラクターの左) |
| `MouthPucker` | 0〜1 (0) | 口をすぼめる |
| `MouthFunnel` | 0〜1 (0) | 口を丸く突き出す |
| `MouthPress` | 0〜1 (0) | 唇を押し付ける |
| `MouthShrug` | 0〜1 (0) | 口元を上げる (への字の逆) |
| `CheekPuff` | 0〜1 (0) | 頬をふくらませる |
| `TongueOut` | 0〜1 (0) | 舌を出す |
| `NoseSneer` | 0〜1 (0) | 鼻にしわを寄せる |

拡張パラメータは、このプロファイルのバージョンの中で追加されることはありますが、既存の ID の意味と値域は変えません。

## フェイストラッキングとの対応

アプリは、トラッキングの結果をこの表に従ってパラメータに変換します。モデルは `Model.avatar.tracking` で対応を上書きできます (係数・オフセット・対象のパラメータの変更)。

### 頭の向き

| パラメータ | 入力 |
| --- | --- |
| `AngleX` / `AngleY` / `AngleZ` | 頭の回転 (ヨー・ピッチ・ロール、度)。上の「向きの約束」に符号を合わせ、パラメータの値域にクランプする |
| `BodyAngleX` / `BodyAngleY` / `BodyAngleZ` | 体のトラッキングがあればその回転。なければ頭の回転 × 0.3 |

### ARKit のブレンドシェイプ

ARKit の 52 ブレンドシェイプ (0〜1) からの変換です。ARKit の `Left` / `Right` はトラッキングされた人の左右で、キャラクター自身の左右と同じ向きとして扱います (画面を鏡像で表示するアプリは、表示側で反転する)。

| パラメータ | 式 |
| --- | --- |
| `EyeLOpen` | `1 − eyeBlinkLeft` |
| `EyeROpen` | `1 − eyeBlinkRight` |
| `EyeLWide` / `EyeRWide` | `eyeWideLeft` / `eyeWideRight` |
| `EyeLSmile` / `EyeRSmile` | `eyeSquintLeft` / `eyeSquintRight` |
| `EyeBallX` | `((eyeLookOutLeft − eyeLookInLeft) − (eyeLookOutRight − eyeLookInRight)) / 2` (正でキャラクターの左を見る) |
| `EyeBallY` | `(eyeLookUpLeft + eyeLookUpRight − eyeLookDownLeft − eyeLookDownRight) / 2` |
| `BrowLY` | `(browOuterUpLeft + browInnerUp) / 2 − browDownLeft` |
| `BrowRY` | `(browOuterUpRight + browInnerUp) / 2 − browDownRight` |
| `BrowInnerUp` | `browInnerUp` |
| `BrowLAngle` / `BrowRAngle` | `browOuterUpLeft − browInnerUp` / `browOuterUpRight − browInnerUp` |
| `MouthOpenY` | `max(0, jawOpen − mouthClose)` |
| `MouthForm` | `(mouthSmileLeft + mouthSmileRight − mouthFrownLeft − mouthFrownRight) / 2` |
| `MouthX` | `mouthLeft − mouthRight` |
| `JawX` | `jawLeft − jawRight` |
| `MouthPucker` | `mouthPucker` |
| `MouthFunnel` | `mouthFunnel` |
| `MouthPress` | `(mouthPressLeft + mouthPressRight) / 2` |
| `MouthShrug` | `(mouthShrugUpper + mouthShrugLower) / 2` |
| `CheekPuff` | `cheekPuff` |
| `TongueOut` | `tongueOut` |
| `NoseSneer` | `(noseSneerLeft + noseSneerRight) / 2` |

結果はパラメータの値域にクランプします。

### トラッキングの対応の上書き

`Model.avatar.tracking` は上書きの列です。各要素は次のフィールドを持ちます。

| フィールド | 内容 |
| --- | --- |
| `source` | 入力の種類: `standard` (上の表で計算したパラメータの値)・`blend_shape` (ARKit のブレンドシェイプの値、0〜1)・`head` (頭の回転、度) |
| `input` | 入力の名前。`standard` なら上の表のパラメータ ID、`blend_shape` なら ARKit のブレンドシェイプ名、`head` なら `yaw` / `pitch` / `roll` |
| `parameter` | 書き込むパラメータ。`standard` なら省略でき、省略すると `input` と同じ ID。それ以外では必須 |
| `scale` | 係数 (既定 1) |
| `offset` | オフセット (既定 0) |

アプリは次の順で値を決めます。

1. 各上書きについて、入力の値 `v` から `v × scale + offset` を計算し、`parameter` に書き込む。同じパラメータへの書き込みが複数あれば足し合わせる。`standard` の入力の値は、上の表の式の結果 (クランプ前) です。
2. 上の表の標準の対応は、そのパラメータを `input` とする `standard` の上書きがあるとき、またはそのパラメータに書き込む上書きがあるときは行わない (上書きが置き換える)。それ以外のパラメータには標準の対応で書き込む。
3. 最後に各パラメータの値域にクランプする。

`head` の値の符号は、上の「向きの約束」と同じです (`yaw` は `AngleX`、`pitch` は `AngleY`、`roll` は `AngleZ` の向き)。
知らない `source` の上書きは無視します (新しい版のファイル)。
同じく、`input` が `source` の種類の入力として知られていない上書き、書き込むパラメータが決まらないかモデルにない上書き、`scale` か `offset` が有限値でない上書きも無視します。無視した上書きは、手順 2 の置き換えも起こしません (検証ツールはこれらをエラーにします)。
標準の対応も、モデルにないパラメータには書き込みません。計算した値が有限値でない (入力が NaN など) ときは、そのパラメータに書き込みません。

```json
"tracking": [
  { "source": "standard", "input": "EyeBallX", "scale": -1 },
  { "source": "standard", "input": "MouthForm", "parameter": "MouthSmile" },
  { "source": "blend_shape", "input": "jawForward", "parameter": "JawForward", "scale": 0.5 }
]
```

1 つ目は `EyeBallX` の符号を反転し、2 つ目は `MouthForm` の代わりに独自の `MouthSmile` を動かし、3 つ目は表で使わないブレンドシェイプを独自のパラメータに対応させます。

この表で使わない ARKit のブレンドシェイプ: `jawForward`、`mouthRollLower`、`mouthRollUpper`、`mouthDimpleLeft`、`mouthDimpleRight`、`mouthStretchLeft`、`mouthStretchRight`、`mouthLowerDownLeft`、`mouthLowerDownRight`、`mouthUpperUpLeft`、`mouthUpperUpRight`、`cheekSquintLeft`、`cheekSquintRight`。
モデルは `Model.avatar.tracking` で、これらを独自のパラメータに対応させられます。

### 実装の範囲

`ugoe_motion::tracking` (`TrackingInput`・`TrackingMap`) がこの節の対応付けを実装します。モデルごとに `TrackingMap` を 1 度作り、毎フレーム `apply` でパラメータに書き込みます。
対応付けは入力の値をそのまま使います。次のことはアプリ (トラッキングの入力を作る側) が行います。

- 平滑化 (揺れを抑えるフィルタ) と、トラッキングが途切れたときの扱い (値を保つ・中立に戻す)
- キャリブレーション (利用者の中立の顔の向きや表情を差し引く)
- トラッカーの出力 (回転行列など) から、上の「向きの約束」に合わせた頭の回転への変換
- 鏡像にした映像を入力にした場合の `Left` / `Right` の入れ替え

MediaPipe Face Landmarker のブレンドシェイプのカテゴリ名は ARKit の名前と同じなので、そのまま使えます (`_neutral` は対応しないので無視する)。
ただし MediaPipe は `tongueOut` を出力しない (`_neutral` と 51 個) ので、MediaPipe だけでは `TongueOut` は動きません。
MediaPipe の `Left` / `Right` は、入力画像に写った顔自身の左右で、入力を左右反転すると入れ替わると報告されています ([google-ai-edge/mediapipe#6368](https://github.com/google-ai-edge/mediapipe/issues/6368))。したがって反転していないカメラ映像を入力にすれば、この仕様の約束 (トラッキングされた人自身の左右) と一致します。

ビューア (T0703) で、実際の人の顔の動画と画像を MediaPipe Face Landmarker に入れて次を確かめました。

- 画像の左に写った目 (反転していない映像では、写った人自身の右目) を閉じると `eyeBlinkRight` が上がる。MediaPipe の `Left` / `Right` は上の報告のとおり、写った人自身の左右です。
- 虹彩が画像の左に寄る (反転していない映像では、写った人が自分の右を見る) と `eyeLookInLeft` と `eyeLookOutRight` が上がり、`EyeBallX` は負 (キャラクターの右を見る) になる。
  `EyeBallX` の符号は `AngleX` (正で自分の左を向く) と揃っていて、反転していない映像を入れると、キャラクターは写った人と同じ向きを見る。
  画面の上で鏡に映った自分のように動かしたいアプリは、表示を反転するか、入力の左右を入れ替える (ヨーとロールの符号も反転する)。
- 変換行列から求めた頭の向きは、顔が画像の右を向く (反転していない映像では、写った人が自分の左を向く) と正のヨー、画像を時計回りに回すと正のロールになった。

> 未決: `MouthX` / `JawX` の符号は、口やあごを横に動かす映像が手に入らず確かめられていない。`mouthLeft` / `jawLeft` も目と同じく写った人自身の左と考えられるが、実際のカメラで確かめる。

### 音声 (リップシンク)

音声からの口の開きは `MouthOpenY` に書き込みます。母音の推定 (あいうえお) に対応する場合は、拡張として `MouthVowelA` 〜 `MouthVowelO` (0〜1) を使います。

## メタデータ

`Model.avatar.meta` に持ちます。すべてのアプリはメタデータを表示でき、利用許諾に反する使い方を利用者に警告すべきです。

| フィールド | 内容 |
| --- | --- |
| `name` | アバターの名前 (必須、空白だけは不可) |
| `version` | アバターのバージョン |
| `authors` | 作者の一覧 (必須、1 人以上) |
| `copyright` | 著作権の表示 |
| `contact` | 連絡先 |
| `references` | 原作・参考元 |
| `thumbnail` | サムネイル画像 (テクスチャのインデックス) |
| `license_url` | 利用規約の URL |
| `avatar_permission` | アバターとして使ってよい人: `only_author` / `only_licensed` / `everyone` |
| `commercial_usage` | 商用利用: `personal_non_profit` / `personal_profit` / `corporation` |
| `allow_violent_usage` | 過度に暴力的な表現での利用 (真偽値) |
| `allow_sexual_usage` | 過度に性的な表現での利用 (真偽値) |
| `allow_political_or_religious_usage` | 政治・宗教目的での利用 (真偽値) |
| `allow_antisocial_or_hate_usage` | 反社会的・差別的な目的での利用 (真偽値) |
| `credit_notation` | クレジット表記: `required` / `unnecessary` |
| `allow_redistribution` | 再配布 (真偽値) |
| `modification` | 改変: `prohibited` / `allow` / `allow_redistribution` |
| `third_party_licenses` | 素材などの第三者のライセンス表記 |

許諾の項目は、既定で最も制限の強い値 (`only_author`、`personal_non_profit`、`false`、`required`、`prohibited`) とします。
知らない値 (新しい版のファイル) も、最も制限の強い値として扱います。
`name` と `authors` 以外のフィールドは省略できます。

## アプリ向けの情報

| フィールド | 内容 |
| --- | --- |
| `Model.avatar.look_at` | 視線の追従に使うパラメータと可動範囲 (下の表)。省略すると `EyeBallX` / `EyeBallY` を、可動範囲 30 度で使う |
| `Model.avatar.hit_areas` | 標準の当たり判定の名前 (`Head`、`Body`) と `HitArea` の対応 |
| `Model.avatar.slots` | 着せ替えのスロット ([modules.md](modules.md)) のうち、アプリに公開するもの (スロットの ID の列) |

`look_at` のフィールド:

| フィールド | 内容 |
| --- | --- |
| `x` / `y` | 左右 (正でキャラクターの左)・上下 (正で上) のパラメータ。既定は `EyeBallX` / `EyeBallY` |
| `range_x` / `range_y` | 可動範囲 (度、0 より大きい)。見る先がこの角度だけずれたとき、パラメータが端の値になる。既定は 30 |

`hit_areas` の要素は `{ "name": "Head", "hit_area": "<HitArea の ID>" }` です。

## 準拠の検査

`ugoe validate --profile avatar` (`ugoe_model::avatar::check::check_profile`) は、モデルの構造の検証に加えて次を検査し、主張したレベルと満たしているレベルを表示します。
エラーが 1 つでもあれば準拠していません。警告は準拠を妨げません。

| 検査 | 重さ |
| --- | --- |
| `Model.avatar` があること | エラー |
| `level` が 1〜3 であること | エラー |
| メタデータの `name` と `authors` があること | エラー |
| 主張したレベルまでに必要なパラメータがあること (レベル 3 は拡張パラメータが 8 つ以上) | エラー |
| `tracking` の `input` が `source` の種類の入力として知られていること、`parameter` (または省略した `standard` の `input`) がモデルにあること、`scale` と `offset` が有限値であること | エラー |
| `look_at` を書いたなら、`x` と `y` のパラメータがモデルにあり、可動範囲が 0 より大きいこと | エラー |
| `hit_areas` の `name` が重複せず、`hit_area` がモデルの当たり判定にあること | エラー |
| `meta.thumbnail` がテクスチャの範囲内にあること | エラー |
| 標準・拡張パラメータの値域 (`min` / `default` / `max`) がこの仕様の値と同じであること | 警告 |
| `profile_version` が検証ツールの知っているバージョン以下であること | 警告 |
| 知らない値 (許諾・`tracking` の `source`) がないこと | 警告 |

満たしているレベルは、主張したレベルとは無関係に、メタデータと必要なパラメータだけから決めます (メタデータが無ければ 0)。

## バージョン

このプロファイルの仕様にはバージョンを付けます (現在 0)。モデルは準拠するバージョンを `Model.avatar.profile_version` に書きます。
