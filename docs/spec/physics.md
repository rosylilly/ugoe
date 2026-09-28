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

## 動的物理 (揺れ用パラメータのいらない物理)

判断の経緯は [ADR 0011](../adr/0011-dynamic-physics.md)。

> 実装待ち: データ定義は T0304、実装は T0305 で行います。パラメータへの出力 (上の節) は T0301 で実装します。

### 出力先

`PhysicsOutput` の出力先 (`target`) を、パラメータ以外にも広げます。

| 出力先 | 内容 |
| --- | --- |
| `parameter` | 従来どおりパラメータに書き込む |
| `deformer_angle` | 回転デフォーマの角度に、粒子の区間の角度 (の `scale` 倍) を足す。粒子の区間とデフォーマを 1 対 1 に対応させると、デフォーマの鎖が揺れもののボーンのように動く |
| `deformer_offset` | 回転デフォーマの原点を、粒子の変位 (の `scale` 倍) だけずらす |
| `vertices` | ワープデフォーマの制御点、またはメッシュの頂点を、重み付きで粒子の変位に追従させる。重みは頂点ごとに持つ (0〜1) |

足し込む値はデフォーマの局所座標系で表します。キーフォームで補間した値に加算します。

### 衝突体

`PhysicsSettings::colliders` に、円 (`circle { center, radius }`) とカプセル (`capsule { a, b, radius }`) の衝突体を置けます。

- 衝突体はデフォーマに取り付け (`attach`)、そのデフォーマのワールド変換で動きます。
- 各ステップの距離拘束の後、衝突体の内側に入った粒子を、衝突体の表面の最も近い点まで押し出します。
- 粒子には衝突用の半径 (`PhysicsParticle::collision_radius`、既定 0) を持たせ、押し出しはこの半径を含めて行います。既存の `radius` は粒子の間の距離 (区間の長さ) なので、衝突には使いません。
- 物理リグごとに、どの衝突体と衝突するかを選べます (`PhysicsRig::colliders`)。

### 重力と風の基準

`PhysicsRig::frame` に基準のデフォーマを指定すると、重力と風のベクトルをそのデフォーマの座標系で解釈します。
たとえば頭の回転デフォーマを指定すると、頭を傾けたときに髪の垂れる向きも傾きます。既定 (`None`) はキャンバスの座標系です。

### 評価との関係

動的物理はデフォーマのワールド座標を必要とするため、評価を 2 段階にします ([evaluation.md#2-評価の順序](evaluation.md#2-評価の順序))。

### 旧ランタイムでの扱い

任意機能 `physics.dynamic` ([ADR 0013](../adr/0013-forward-compatibility.md)) です。知らないランタイムは、パラメータ以外への出力と衝突体を無視します (揺れない)。
