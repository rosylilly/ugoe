# ugoe アバタープロファイル仕様 (バージョン 0)

判断の経緯は [ADR 0012](../adr/0012-avatar-profile.md)。

> 実装待ち: データ定義と検証は T0110、トラッキングの対応付けの実装は T0306 で行います。
> この仕様は公開を前提に、ugoe の実装から独立して読めるように書きます。1.0 までは内容が変わりえます。

アバタープロファイルは、2D アバターをどのアプリでも同じように動かすための約束です。
モデルは `Model.avatar` にプロファイルの情報を持ち、アプリはそれを読んでトラッキングや表示を接続します。

## 準拠レベル

| レベル | 名前 | 必要なもの |
| --- | --- | --- |
| 1 | 基本 | メタデータ、頭の向き (`AngleX` `AngleY` `AngleZ`)、目の開閉 (`EyeLOpen` `EyeROpen`)、口の開き (`MouthOpenY`) |
| 2 | 標準 | レベル 1 に加えて、視線 (`EyeBallX` `EyeBallY`)、眉 (`BrowLY` `BrowRY`)、口の形 (`MouthForm`)、体の向き (`BodyAngleX` `BodyAngleY` `BodyAngleZ`)、呼吸 (`Breath`) |
| 3 | 表情 | レベル 2 に加えて、下の「拡張パラメータ」のうち少なくとも 8 つ |

モデルは準拠を主張するレベルを `Model.avatar.level` に書きます。検証ツール (`ugoe validate --profile avatar`) は、主張したレベルに必要なパラメータが揃っているかを検査します。

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

この表で使わない ARKit のブレンドシェイプ: `jawForward`、`mouthRollLower`、`mouthRollUpper`、`mouthDimpleLeft`、`mouthDimpleRight`、`mouthStretchLeft`、`mouthStretchRight`、`mouthLowerDownLeft`、`mouthLowerDownRight`、`mouthUpperUpLeft`、`mouthUpperUpRight`、`cheekSquintLeft`、`cheekSquintRight`。
モデルは `Model.avatar.tracking` で、これらを独自のパラメータに対応させられます。

> 未決: `MouthX` / `JawX` と、`EyeBallX` の符号が「キャラクターの左 = 正」で直感と合うか、実際のトラッキングで確かめる (T0306)。

### 音声 (リップシンク)

音声からの口の開きは `MouthOpenY` に書き込みます。母音の推定 (あいうえお) に対応する場合は、拡張として `MouthVowelA` 〜 `MouthVowelO` (0〜1) を使います。

## メタデータ

`Model.avatar.meta` に持ちます。すべてのアプリはメタデータを表示でき、利用許諾に反する使い方を利用者に警告すべきです。

| フィールド | 内容 |
| --- | --- |
| `name` | アバターの名前 (必須) |
| `version` | アバターのバージョン |
| `authors` | 作者の一覧 (必須) |
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

## アプリ向けの情報

| フィールド | 内容 |
| --- | --- |
| `Model.avatar.look_at` | 視線の追従に使うパラメータと可動範囲 |
| `Model.avatar.hit_areas` | 標準の当たり判定の名前 (`Head`、`Body`) と `HitArea` の対応 |
| `Model.avatar.slots` | 着せ替えのスロット ([modules.md](modules.md)) のうち、アプリに公開するもの |

## バージョン

このプロファイルの仕様にはバージョンを付けます (現在 0)。モデルは準拠するバージョンを `Model.avatar.profile_version` に書きます。
