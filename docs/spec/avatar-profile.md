# ugoe アバタープロファイル仕様 (バージョン 0)

判断の経緯は [ADR 0012](../adr/0012-avatar-profile.md)。

> 実装: データ定義は `ugoe_model::avatar`、検証は `ugoe validate --profile avatar`、トラッキングの対応付けは `ugoe_motion::tracking` です。
> 公開仕様は英語の [spec/avatar-profile.md](../../spec/avatar-profile.md) (CC BY 4.0) で、食い違ったら英語版が正です ([ADR 0021](../adr/0021-public-specification.md)、節の対応は [public-spec.md](public-spec.md#アバタープロファイル-specavatar-profilemd))。
> この文書は開発用で、実装の型名や経緯を含みます。1.0 までは内容が変わりえます。

アバタープロファイルは、2D アバターをどのアプリでも同じように動かすための約束です。
モデルは `Model.avatar` にプロファイルの情報を持ち、アプリはそれを読んでトラッキングや表示を接続します。

## 準拠レベル

| レベル | 名前 | 必要なもの |
| --- | --- | --- |
| 1 | 基本 | メタデータ、頭の向き (`AngleX` `AngleY` `AngleZ`)、目の開閉 (`EyeLOpen` `EyeROpen`)、口の開き (`MouthOpenY`) |
| 2 | 標準 | レベル 1 に加えて、視線 (`EyeBallX` `EyeBallY`)、眉 (`BrowLY` `BrowRY`)、口の形 (`MouthForm`)、体の向き (`BodyAngleX` `BodyAngleY` `BodyAngleZ`)、呼吸 (`Breath`) |
| 3 | 表情 | レベル 2 に加えて、下の「拡張パラメータ」のうち少なくとも 8 つ |

メタデータの条件は、`meta.name` が空白以外の文字を含み、`meta.authors` に空白以外の文字を含む要素が 1 つ以上あることです。

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

### 体の拡張パラメータ

体のトラッキング (MediaPipe Pose Landmarker など) で動かすパラメータです (T0645)。どのレベルにも必要なく、レベル 3 の「拡張パラメータのうち 8 つ」にも数えません。`ugoe_model::standard::BODY_EXTENDED` にあります。

| ID | 値域 (既定) | 意味 |
| --- | --- | --- |
| `ShoulderLY` / `ShoulderRY` | -1〜1 (0) | キャラクター自身の左肩・右肩の上下。0 が力を抜いた中立、1 で肩をすくめた (上げきった) 状態、負で肩を下げた状態 |

肩の上下は体 (胴) に対する肩の動きで、体ごと傾く動きは `BodyAngleZ` が表します。

## フェイストラッキングとの対応

アプリは、トラッキングの結果をこの表に従ってパラメータに変換します。モデルは `Model.avatar.tracking` で対応を上書きできます (係数・オフセット・対象のパラメータの変更)。

### 頭と体

入力の頭の回転はカメラに対する頭の向きで、体の回転 (上半身の、カメラに対する向き) と肩の上下は省略できます (体のトラッキングが無い・体が映っていない)。体の回転はヨー・ピッチ・ロールを軸ごとに省略できます (腰が映らないとピッチだけ無い、など)。

| パラメータ | 入力 |
| --- | --- |
| `AngleX` / `AngleY` / `AngleZ` | 頭の回転 (ヨー・ピッチ・ロール、度) をそのまま (1 度 = 1)。上の「向きの約束」に符号を合わせ、パラメータの値域にクランプする。体の回転は差し引かない |
| `BodyAngleX` / `BodyAngleY` / `BodyAngleZ` | 体の回転のその軸があればその値 (度、頭と同じ符号)。なければ頭の回転のその軸 × 0.3 |
| `ShoulderLY` / `ShoulderRY` | 左肩・右肩の上下 (0 が中立、1 ですくめた状態) があればその値。なければ値が無い (書き込まない) |

**頭の角度から体の回転を引かない理由 (T0645 で決めた):** 体ごと 20° 回ると、`AngleX` = 20 と `BodyAngleX` = 10 (クランプ) の両方が付き、頭は体に対して回った分より大きく見えます。それでも引かないのは次の理由です。

- 体のトラッキングが無いときの規則 (`AngleX` は頭の回転そのまま、体はその 0.3 倍を足す) が、もともと「`AngleX` はカメラに対する頭の向き」という約束で、モデルもそれに合わせて作られている。体のトラッキングの有無で `AngleX` の意味が変わらない。
- 単眼のカメラの Pose Landmarker のヨーは、試した画像で 10〜15° ずれることがある (下の「実装の範囲」)。引くと、その誤差が最も目立つ `AngleX` に乗る。
- `BodyAngle*` の値域 (±10) は頭 (±30) より狭いので、重なる量は小さい。

トレードオフ: 多くのリグで頭は体の子なので、体ごと回ったときは、画面の頭の回転が体の分 (`BodyAngleX` の値域の 10 まで) だけ実際より大きくなります (二重に近くなる)。差し引かないことで、単眼の体のヨーの誤差 (10〜15°) が `AngleX` に乗るのを避けています。アプリは利用者の選択で、対応付けの前に頭の回転から体の回転を差し引いてもよい (MAY) が、既定では差し引かない (SHOULD NOT)。参照実装 (ビューア・エディタ) はこの切り替えをまだ持ちません。

体に対する頭の角度で動かしたいモデルは、上書き (`head` の入力に `standard` の `BodyAngleX` を負の係数で足す、など) で作れます。ARKit の顔の変換も (MediaPipe と同じく) カメラに対する頭の向きなので、この約束のまま使えます。

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

1. 各上書きについて (上書きの並びの順は結果に影響しない)、入力の値 `v` から `v × scale + offset` を計算し、`parameter` に書き込む。同じパラメータへの書き込みが複数あれば足し合わせる。`standard` の入力の値は、上の表の式の結果 (クランプ前) です。
2. 上の表の標準の対応は、そのパラメータを `input` とする `standard` の上書きがあるとき、またはそのパラメータに書き込む上書きがあるときは行わない (上書きが置き換える)。それ以外のパラメータには標準の対応で書き込む。
3. 最後に各パラメータの値域にクランプする。

`head` の値の符号は、上の「向きの約束」と同じです (`yaw` は `AngleX`、`pitch` は `AngleY`、`roll` は `AngleZ` の向き)。`head` は常に頭の回転で、体の回転は読みません (体は `standard` の `BodyAngleX` などで読む)。
`standard` の入力が値を持たない (肩の入力が無い) とき、その上書きは値を持ちません。書き込み先に値を持たない上書きが 1 つでもあれば、そのパラメータには書き込みません (下の「有限値でない」と同じ扱い。実装は NaN)。
知らない `source` の上書きは無視します (新しい版のファイル)。
同じく、`input` が `source` の種類の入力として知られていない上書き、書き込むパラメータが決まらないかモデルにない上書き、`scale` か `offset` が有限値でない上書きも無視します。無視した上書きは、手順 2 の置き換えも起こしません (検証ツールはこれらをエラーにします)。
標準の対応も、モデルにないパラメータには書き込みません。中継用 (`internal`) のパラメータ ([model.md#パラメータ](model.md#パラメータ)) には、標準の対応でも上書きでも書き込みません。書き込み先が中継用の上書きも、手順 2 の置き換えは起こします (`standard` の `input` の標準の対応は書き込まれない)。計算した値が有限値でない (入力が NaN など) ときは、そのパラメータに書き込みません。

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

`ugoe_motion::tracking` (`TrackingInput`・`TrackingMap`) がこの節の対応付けを実装します。モデルごとに `TrackingMap` を 1 度作り (`TrackingMap::for_model`。中継用のパラメータを除く)、毎フレーム `apply` でパラメータに書き込みます。
対応付けは入力の値をそのまま使います。次のことはアプリ (トラッキングの入力を作る側) が行います。

- 平滑化 (揺れを抑えるフィルタ) と、トラッキングが途切れたときの扱い (値を保つ・中立に戻す)
- キャリブレーション (利用者の中立の顔の向きや表情を差し引く)
- トラッカーの出力 (回転行列など) から、上の「向きの約束」に合わせた頭の回転への変換
- 鏡像にした映像を入力にした場合の `Left` / `Right` の入れ替え (体も同じく、左右の肩を入れ替え、ヨーとロールの符号を反転する)
- 体のトラッカーの出力 (点の座標) から、体の回転と肩の上下への変換

MediaPipe Face Landmarker のブレンドシェイプのカテゴリ名は ARKit の名前と同じなので、そのまま使えます (`_neutral` は対応しないので無視する)。
ただし MediaPipe は `tongueOut` を出力しない (`_neutral` と 51 個) ので、MediaPipe だけでは `TongueOut` は動きません。
MediaPipe の `Left` / `Right` は、入力画像に写った顔自身の左右で、入力を左右反転すると入れ替わると報告されています ([google-ai-edge/mediapipe#6368](https://github.com/google-ai-edge/mediapipe/issues/6368))。したがって反転していないカメラ映像を入力にすれば、この仕様の約束 (トラッキングされた人自身の左右) と一致します。

ビューア (T0703) で、実際の人の顔の動画と画像を MediaPipe Face Landmarker に入れて次を確かめました。

- 画像の左に写った目 (反転していない映像では、写った人自身の右目) を閉じると `eyeBlinkRight` が上がる。MediaPipe の `Left` / `Right` は上の報告のとおり、写った人自身の左右です。
- 虹彩が画像の左に寄る (反転していない映像では、写った人が自分の右を見る) と `eyeLookInLeft` と `eyeLookOutRight` が上がり、`EyeBallX` は負 (キャラクターの右を見る) になる。
  `EyeBallX` の符号は `AngleX` (正で自分の左を向く) と揃っていて、反転していない映像を入れると、キャラクターは写った人と同じ向きを見る。
  画面の上で鏡に映った自分のように動かしたいアプリは、表示を反転するか、入力の左右を入れ替える (ヨーとロールの符号も反転する)。
- 変換行列から求めた頭の向きは、顔が画像の右を向く (反転していない映像では、写った人が自分の左を向く) と正のヨー、画像を時計回りに回すと正のロールになった。

### 体のトラッキング (MediaPipe Pose Landmarker)

ビューアとエディタのプレビュー (`packages/face-tracking`、T0645) は、顔と同じフレームで MediaPipe Pose Landmarker (lite、`pose_landmarker_lite.task` 版 1) を動かし、体の入力を作ります。変換は `packages/face-tracking/src/pose.ts`。使う点は 7・8 (左右の耳)、11・12 (左右の肩)、23・24 (左右の腰) です。

- 点の `left` / `right` は写った人自身の左右です (反転していない画像では左肩が画像の右に写る。画像を左右反転しても名前は入れ替わらない = 顔と同じく、反転した映像なら入れ替えて直す)。左肩が画像の右に無い (背中を向けている) ときは使いません。
- **ロール:** 画像 (ピクセル) での肩の線の傾き `atan2(左肩.y − 右肩.y, 左肩.x − 右肩.x)`。画面で時計回りが正です。`worldLandmarks` の肩の線の傾きは、正面を向いた画像で 14° 傾くなど安定しなかったので使いません。
- **ヨー:** `worldLandmarks` (メートル、腰の中点が原点、y は下向き、z はカメラから遠いほど大きい) の肩の線の奥行きの傾き `atan2(左肩.z − 右肩.z, 左肩.x − 右肩.x)`。自分の左を向くと (左肩が奥へ行き) 正です。
- **ピッチ:** `worldLandmarks` の、腰の中点から肩の中点へのベクトル `t` で `atan2(t.z, −t.y)`。後ろへ反ると (肩が奥へ行き) 正です。腰が見えていなければ無し。
- **肩の上下:** 画像での、耳の中点から肩の中点までの肩の線に垂直な距離 `d` を、肩幅 `w` (画像の肩幅 ÷ max(0.5, cos(ヨー)) でヨーの縮みを補正) で割り、`(0.62 − d / w) / 0.2` とします。0.62 は正面の画像での `d / w` の目安 (0.60〜0.65)、0.2 はすくめると肩が肩幅の約 2 割上がるという見積もりです。左右の肩を別々には求めず、両方に同じ値を入れます (片方の肩だけを上げる動きは、肩の点だけでは体ごとの傾き = ロールと区別できないため、ロールに任せる)。耳が見えていなければ無し。
- **見えているか:** 点の組 (肩・耳・腰) ごとに、両方の点の `visibility` の小さい方が 0.6 以上で見え始め、0.4 未満で見えなくなったとします (ちらつきを抑える)。肩が見えなければ体の値はすべて無しで、体の回転は頭 × 0.3 に戻ります。顔が見つからないフレームは体も使いません。
- **キャリブレーション:** 体の値も中立を差し引きます。ピッチと肩の上下は、カメラの高さ・角度や体格で基準が大きく変わる (正面の画像でピッチが −15〜−20° になった) ので、**キャリブレーションした後だけ**渡します (中立の姿勢でその値が無ければ渡さない)。
- 平滑化は値ごとの One Euro フィルタで、途切れた値のフィルタは忘れます。記録 (JSON) は版 2 で `body` を持ちます (版 1 も読める)。

実際の画像で確かめたこと (ヘッドレスの Chrome、SwiftShader。素材はリポジトリの外: MediaPipe の試験用の `portrait.jpg`・`pose.jpg`、Wikimedia Commons の写真):

- 正面の上半身 (`portrait.jpg`) でロール −2°、時計回りに 15° 回した画像で +14〜+16°。左右反転でヨーの符号が反転する (lite では −3° と +10° のように、画像によって 10〜15° ずれる。full は −8° と +11°)。
- 体を自分の右へ大きくひねった座像でヨー −65〜−68° (`BodyAngleX` は −10 にクランプ)。
- 肩をすくめた写真と中立の写真の `d / w` は、頭の傾きなどが混じって、はっきりした差が出なかった。肩の上下の係数 (0.62・0.2) は実際のカメラでの調整が要る。

> 未決: 肩の上下の係数と、体のピッチの符号は実際のカメラで確かめていない (T0645 のメモの「利用者に確かめてほしいこと」)。

> 未決: `MouthX` / `JawX` の符号は、口やあごを横に動かす映像が手に入らず確かめられていない (T0703・T0704 でも素材がなく、利用者の実際のカメラでの確認もまだ)。`mouthLeft` / `jawLeft` も目と同じく写った人自身の左と考えられるが、実際のカメラで確かめる。
> パラメータの意味 (正でキャラクター自身の左) は決まっていて、未決なのは式の符号だけです。公開仕様にも未決として書き、逆だった場合は上書き (`{ "source": "standard", "input": "MouthX", "scale": -1 }`) で直せることを示しています。

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

値の意味:

- `avatar_permission`: `only_author` は作者だけ、`only_licensed` は作者が許諾した人だけ、`everyone` は誰でも。
- `commercial_usage`: `personal_non_profit` は個人の非営利の利用だけ、`personal_profit` は個人の営利の利用まで、`corporation` は法人などの利用まで。
- `modification`: `prohibited` は改変の禁止、`allow` は改変してよいが改変したものの再配布は不可、`allow_redistribution` は改変したものの再配布まで可。
- `allow_redistribution` は改変していないモデル (ファイル) の再配布。

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

角度からパラメータへは線形に対応させることを推奨します (0 度で既定値、`+range` で最大値、`-range` で最小値、範囲の外はクランプ)。
ランタイム (`ugoe` のクレート) は `look_at` を使う処理を持たず、視線の追従はアプリが行います。

`hit_areas` の要素は `{ "name": "Head", "hit_area": "<HitArea の ID>" }` です。
標準の名前はこのバージョンでは `Head` と `Body` です。検証ツールはほかの名前をエラーにしません。アプリは知らない名前を無視します。
`slots` に書いていないスロットは作者用で、アプリは利用者に見せないことを推奨します。

## アプリの規則

アバターとしてモデルを読むアプリ (トラッキングのアプリ・配信ツール・ゲーム・ビューア) の規則です。英語版の [Rules for applications](../../spec/avatar-profile.md#rules-for-applications) が正です。

- **読むもの:** `Model.avatar` の各フィールド。使わない部分は無視してよいが、利用許諾を表示するため `meta` は読む。主張したレベルに頼らず、モデルにあるトラッキングの対象を動かす。準拠の検査に失敗したモデルや、新しい `profile_version` のモデルも拒否しない。
- **動かし方:** [フェイストラッキングとの対応](#フェイストラッキングとの対応) の式と上書きの規則に従う。鏡像にした映像を入力にしたら、`Left` / `Right` を入れ替え、ヨーとロールの符号を反転してから対応付けに渡す (体の入力も、左右の肩を入れ替え、体のヨーとロールの符号を反転する)。中継用のパラメータとモデルにないパラメータには書き込まない。トラッカーの出力の変換・平滑化・キャリブレーション・途切れたときの扱いはアプリが決める。目の開閉をトラッキングしている間にモデルの目パチを使うなら、利用者が目パチを止められるようにする (どちらも `EyeLOpen` / `EyeROpen` に掛かる)。
- **利用許諾の表示:** 初めて使う前にメタデータ (少なくとも `name`・`authors`・`copyright`・`license_url`・`third_party_licenses` と許諾の項目) を見せ、その後も見られるようにする。許諾を、既定値と未知の値のフォールバックを適用した値より緩く見せてはならない。
- **警告:** アプリが提供する使い方が許諾に反するとき、利用者がその使い方を選んだ時点で警告する。

  | 項目 | 警告する時点 |
  | --- | --- |
  | `avatar_permission` が `everyone` でない | アバターとして使い始めるとき |
  | `commercial_usage` | アプリや機能が、値の許す範囲を超える商用の目的 (収益化した配信・商品など) に使われるとき |
  | `allow_violent_usage`・`allow_sexual_usage`・`allow_political_or_religious_usage`・`allow_antisocial_or_hate_usage` が `false` | アプリや選んだ内容がその種類のとき |
  | `credit_notation` が `required` | 配信・録画・ゲームなどでアバターを見せるとき。クレジット (`name`・`authors`・`copyright`) を表示しやすくする |
  | `allow_redistribution` が `false` | モデルのファイルを書き出す・アップロードする・共有するとき |
  | `modification` が `prohibited` | 改変したモデルを保存するとき |
  | `modification` が `allow` | 改変したモデルを再配布するとき |

  アプリは利用者が作者か・許諾を受けたかを知りえないので、警告は項目を示して先へ進めるようにする (使い方を止めることは求めない)。
- **プロファイルが無いモデル:** 許諾が宣言されていない。アプリは許諾が分からないことを伝え、どの使い方も許されているかのように見せない。

> 実装の状況: ビューア (apps/viewer) はまだメタデータと利用許諾を表示しない (T0631 のメモの後続の候補)。エディタはインスペクタで編集できる。

## 準拠の検査

`ugoe validate --profile avatar` (`ugoe_model::avatar::check::check_profile`) は、モデルの構造の検証に加えて次を検査し、主張したレベルと満たしているレベルを表示します。
エラーが 1 つでもあれば準拠していません。警告は準拠を妨げません。

| 検査 | 重さ |
| --- | --- |
| `Model.avatar` があること | エラー |
| `level` が 1〜3 であること | エラー |
| メタデータの `name` と `authors` があること | エラー |
| 主張したレベルまでに必要なパラメータがあること (レベル 3 は拡張パラメータが 8 つ以上。`level` が 3 より大きいときはレベル 3 まで) | エラー |
| `tracking` の `input` が `source` の種類の入力として知られていること、`parameter` (または省略した `standard` の `input`) がモデルにあること (`source` が知られているときだけ)。`scale` と `offset` が有限値であること (`source` が未知の上書きも) | エラー |
| `look_at` を書いたなら、`x` と `y` のパラメータがモデルにあり、可動範囲が有限で 0 より大きいこと | エラー |
| `hit_areas` の `name` が重複せず、`hit_area` がモデルの当たり判定にあること | エラー |
| `slots` の ID がモデルのスロット (`Model.slots`) にあること | エラー |
| `meta.thumbnail` がテクスチャの範囲内にあること | エラー |
| 標準・拡張・体の拡張・母音のパラメータの値域 (`min` / `default` / `max`) がこの仕様の値と同じであること | 警告 |
| `profile_version` が検証ツールの知っているバージョン以下であること | 警告 |
| 知らない値 (許諾・`tracking` の `source`) がないこと | 警告 |
| トラッキングの対象 (標準の対応表のパラメータ・`tracking` の書き込み先・`look_at` のパラメータ) が中継用 (`internal`) でないこと ([model.md#パラメータ](model.md#パラメータ)) | 警告 |

数値はモデルの検証と同じ方針で見ます ([model.md#数値の検証の方針](model.md#数値の検証の方針))。プロファイルの `f32` (`tracking` の `scale`・`offset`、`look_at` の `range_x`・`range_y`) は、NaN・無限大なら場所 (`avatar.tracking[0].scale` など) つきのエラーです。
`source` が未知の上書きは無視されますが、`f32` が有限値である規則は新しい版でも変わらないので、その数値も検査します (表情の `op` が未知の項目と同じ)。
範囲は、範囲の外で計算が壊れる `range_x`・`range_y` (0 で割る) だけを 0 より大きいことに限ります。`scale` と `offset` には範囲を定めません (0 や負の係数で反転・固定する上書きにも意味がある)。プロファイルには下限と上限の組の値はありません。

満たしているレベルは、主張したレベルとは無関係に、メタデータと必要なパラメータだけから決めます (メタデータが無ければ 0)。

## バージョン

このプロファイルの仕様にはバージョンを付けます (現在 0)。モデルは準拠するバージョンを `Model.avatar.profile_version` に書きます。

体のトラッキングの入力と体の拡張パラメータ (`ShoulderLY` / `ShoulderRY`、T0645) は、バージョン 0 のまま足しました。どのレベルにも必要なく、知らないアプリはそれを動かさないだけで、既存のモデル・アプリの振る舞いは変わらないためです (1.0 までの下書きの規則。英語版の [Versions](../../spec/avatar-profile.md#versions))。
