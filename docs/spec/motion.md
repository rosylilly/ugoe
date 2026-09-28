# モーション・表情・エフェクト仕様

対応するコードは、データが `ugoe_model::{motion, expression, effects, pose}`、再生が `crates/ugoe-motion`、フレーム内の順序の管理が `crates/ugoe` (`Puppet`)。

## モーション

### カーブ

カーブは対象 (`CurveTarget`) とセグメント列を持ちます。最初のセグメントは必ず `start` で、以降の各セグメントの始点は直前のセグメントの終点です。

| セグメント | 区間 `[p0.t, to.t]` での値 |
| --- | --- |
| `linear` | `lerp(p0.v, to.v, u)`、`u = (t - p0.t) / (to.t - p0.t)` |
| `bezier` | 3 次ベジェ `B(s) = (1-s)³p0 + 3(1-s)²s c1 + 3(1-s)s² c2 + s³ to`。時刻 `t` について `B_t(s) = t` を解いて `s` を求め、`B_v(s)` を値とする |
| `stepped` | 区間の終点より前は `p0.v`、終点では `to.v` |
| `inverse_stepped` | 区間の始点では `p0.v`、始点より後は `to.v` |

- ベジェの制御点の時刻は区間内 (`p0.t <= c.t <= to.t`) にあるものとします。これにより `B_t` は単調になり、`s` がただ 1 つに決まります。範囲外の値は読み込み時にクランプします。
- 最初の点より前の時刻では最初の点の値を、最後の点より後の時刻では最後の点の値を使います。

### 対象

| 対象 | 意味 |
| --- | --- |
| `parameter` | パラメータの値 |
| `part_opacity` | パーツの不透明度を上書きする |
| `effect_group` | エフェクトグループ (`EyeBlink`、`LipSync`。[エフェクトグループ](#エフェクトグループ)) の強さ。エフェクトの出力に掛ける |

### 再生とフェード

- モーションの重み `w = fade_in_weight * fade_out_weight`。フェードの曲線は `easing(x) = 0.5 - 0.5 cos(π x)` (0〜1) を使います。
- カーブ個別の `fade_in` / `fade_out` が指定されていれば、モーション全体の値の代わりにそれを使います。
- パラメータへの適用: `value = lerp(current, curve_value, w)` (`ParameterValues::blend`)。
- `looping = true` のモーションは `duration` で折り返します。ループの境目ではフェードをかけません。
- `events` は再生時刻がイベントの時刻を通過したフレームで発火します。ループをまたいだ場合も漏らさず発火します。

### モーションマネージャ (T0303)

- 優先度 (`idle` < `normal` < `force`) を持ちます。現在再生中のものより低い優先度の要求は無視します。
- 新しいモーションを開始すると、再生中のモーションは自身の `fade_out` の時間をかけてフェードアウトし、新しいモーションは `fade_in` の時間をかけてフェードインします。

## 表情

表情は、モーションを適用した後のパラメータ値に重ねます。

| op | 式 |
| --- | --- |
| `add` | `v + value * w` |
| `multiply` | `v * lerp(1, value, w)` |
| `overwrite` | `lerp(v, value, w)` |

`w` は表情のフェードの重みです。複数の表情を同時に使う場合は、開始した順に適用します。

## エフェクト

設定のデータは `Model.effects` (`ugoe_model::effects::Effects`) と `Model.pose` (`ugoe_model::pose::Pose`) です ([model.md](model.md#エフェクト設定))。再生は T0303 で実装します。

| エフェクト | 挙動 |
| --- | --- |
| 目パチ | 対象パラメータ (`eye_blink.parameters`) を、ランダムな間隔 (平均 `interval`) で 1 → 0 → 1 と動かす。閉じるのに `closing`、閉じたまま `closed`、開くのに `opening` 秒かける |
| 呼吸 | `breath.parameters` の各要素について、対象パラメータに `weight * (offset + peak * sin(2π t / cycle))` を加算する |
| リップシンク | 外部から与えられた音量 (0〜1) を対象パラメータ (`lip_sync.parameters`) に書き込む |
| ポーズ | 排他的なパーツのグループで、表示中のパーツを 1 つに保つ。切り替えは `fade_duration` 秒でクロスフェードする。`links` のパーツはグループの要素と同じ不透明度にする |

目パチの乱数は、シード (`eye_blink.seed`) 付きの決定的な擬似乱数にします (テストで再現できるようにするため)。間隔の分布は T0303 で決めます。

### 既定値

`Effects::default()` は対象パラメータを持たず (何もしない)、時間などは下の値を持ちます。`Effects::standard()` は標準パラメータを対象にした推奨設定です。

| 項目 | `default()` | `standard()` |
| --- | --- | --- |
| 目パチの対象 | なし | `EyeLOpen`、`EyeROpen` |
| 目パチの時間 | `interval` 4.0、`closing` 0.1、`closed` 0.05、`opening` 0.15 秒、`seed` 0 | 同左 |
| リップシンクの対象 | なし | `MouthOpenY` |
| ポーズの `fade_duration` | 0.5 秒 | — |

`Breath::standard()` の呼吸 (周期は互いに割り切れない値にして、動きの繰り返しを目立たなくする):

| パラメータ | `offset` | `peak` | `cycle` (秒) | `weight` |
| --- | --- | --- | --- | --- |
| `AngleX` | 0 | 15 | 6.5345 | 0.5 |
| `AngleY` | 0 | 8 | 3.5345 | 0.5 |
| `AngleZ` | 0 | 10 | 5.5345 | 0.5 |
| `BodyAngleX` | 0 | 4 | 15.5345 | 0.5 |
| `Breath` | 0.5 | 0.5 | 3.2345 | 1 |

### エフェクトグループ

モーションのカーブの対象 `effect_group` (`CurveTarget::EffectGroup`) は、次の名前でエフェクトの出力の強さを指定します (`ugoe_model::effects` の定数)。

| 名前 | 定数 | 対象 |
| --- | --- | --- |
| `EyeBlink` | `EYE_BLINK_GROUP` | `eye_blink.parameters` |
| `LipSync` | `LIP_SYNC_GROUP` | `lip_sync.parameters` |

呼吸とポーズはエフェクトグループを持ちません。未知の名前のカーブは無視します。

## フレーム内の適用順

`Puppet::update(dt)` は次の順に適用します。

1. パラメータを前フレームの値から開始する (リセットしない)
2. モーション
3. 表情
4. 目パチ
5. 呼吸
6. 物理
7. ポーズ
8. リップシンク
9. `ModelInstance::update()`

> 未決: Cubism Framework のサンプル実装と同じく、パラメータを「保存した値」に戻してからモーションを適用するか。こうするとモーションが対象にしないパラメータの値がフレームごとに累積しなくなる。T0303 で決める。
