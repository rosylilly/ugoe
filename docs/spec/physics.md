# 物理演算仕様

対応するコードは、データが `ugoe_model::physics`、アルゴリズムが `crates/ugoe-physics`。

## 概要

物理リグ (`PhysicsRig`) は **入力 → 振り子チェーン → 出力** の 3 段で動きます。

1. **入力**: 顔の角度などのパラメータから、チェーンの根元の平行移動と回転を作ります。
2. **振り子チェーン**: 粒子を距離拘束でつないだ鎖を、重力・風・根元の動きに従ってシミュレーションします。
3. **出力**: 粒子の位置や、隣り合う粒子の成す角を、髪揺れなどのパラメータに書き込みます。

## 時間の進め方

- 固定ステップで積分します。ステップ幅は `1 / settings.fps` です。
- `update(dt)` は `dt` を累積し、ステップ幅を超えた分だけステップを実行します。
- 出力は、直前 2 ステップの結果を余りの時間で線形補間して書き込みます。これにより、表示のフレームレートと物理のステップが揃わなくても揺れが滑らかになります。
- 1 回の `update` で実行するステップ数には上限 (既定 8) を設けます。上限を超えた分の時間は捨てます (スパイク対策)。

## 入力

各入力 (`PhysicsInput`) は、パラメータの値を正規化してから重み付きで加算します。

```
n = normalize(value, param.min, param.default, param.max, range)   // range は normalization.position または angle
if reflect: n = -n
translation.x += n * weight/100      (source = x)
translation.y += n * weight/100      (source = y)
angle         += n * weight/100      (source = angle)
```

`normalize` は、パラメータの default から min 側・max 側で別々に線形写像します。
`value` が default より大きければ `[default, max] → [range.default, range.max]`、小さければ `[min, default] → [range.min, range.default]` に写します。

## 振り子チェーン

粒子 0 は根元で、入力から求めた位置に固定されます。粒子 i (i >= 1) ごとに、各ステップで次の計算をします。

```
dir      = 1 つ前の粒子からの方向ベクトル (根元の回転 angle を適用したもの)
force    = (gravity + wind) * acceleration
velocity = (position - last_position) * delay
position = position + velocity + force * dt² * mobility
position = prev.position + normalize(position - prev.position) * radius   // 距離拘束
```

> 未決: 上の式は Cubism の公開資料と挙動観察から組み立てた暫定案です。T0301 で、同じ設定値で Cubism と並べて見た目を比べ、係数の意味 (`mobility`、`delay`、`acceleration`) を確定させます。互換の基準は「見た目の挙動が近いこと」で、数値が一致する必要はありません。

## 出力

```
source = x     → 粒子 i の位置 (1 つ前の粒子からの相対) の x 成分
source = y     → 同じく y 成分
source = angle → 粒子 i-1 → i のベクトルと、粒子 i-2 → i-1 のベクトル (i = 1 なら重力方向) の成す角
value = denormalize(source * scale) を出力パラメータに weight/100 でブレンドして書き込む
if reflect: 符号を反転する
```

## 決定性

物理演算も決定的です。同じ入力列 (パラメータ値の列と `dt` の列) からは、常に同じ出力が得られます。乱数は使いません。
