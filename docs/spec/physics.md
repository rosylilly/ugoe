# 物理演算仕様

対応するコードは、データが `ugoe_model::physics`、アルゴリズムが `crates/ugoe-physics`。

## 概要

物理リグ (`PhysicsRig`) は **入力 → 振り子チェーン → 出力** の 3 段で動きます。

1. **入力**: 顔の角度などのパラメータから、チェーンの根元の平行移動と回転を作ります。
2. **振り子チェーン**: 粒子を距離拘束でつないだ鎖を、重力・風・根元の動きに従ってシミュレーションします。
3. **出力**: 粒子の位置や、隣り合う粒子の成す角を、髪揺れなどのパラメータに書き込みます。

## 座標と単位

- 物理空間は **y 軸下向き** (キャンバスと同じ向き) の 2 次元空間で、長さの単位は任意です (正規化の範囲 `normalization.position` と粒子の `radius` で決まる)。重力の既定は `(0, 1)` (真下) です。
- 角度は **度** で、時計回りが正です ([evaluation.md](evaluation.md) の回転デフォーマと同じ)。
- 係数の時間の基準は **基準フレーム (1/30 秒)** です。`mobility` と `delay` は「基準フレームあたり」の値なので、ステップの周波数 (`fps`) を変えても揺れの速さと減衰はほぼ変わりません。

## 時間の進め方

- 固定ステップで積分します。ステップ幅は `1 / settings.fps` です。`fps` が正の有限値でなければ 60 を使います。
- `update(dt)` は `dt` を累積し、ステップ幅を超えた分だけステップを実行します。`dt` が負・非有限なら 0 とみなします。
- 1 回の `update` で実行するステップ数には上限 (既定 8、`MAX_STEPS_PER_UPDATE`) を設けます。上限を超えた分の時間は捨てます (スパイク対策)。
- 入力 (根元の姿勢) は、前回ステップを実行した時点の姿勢から今回の `update` の姿勢へ、ステップごとに線形補間して与えます。1 フレームで何ステップも進むときに、入力の変化が最初のステップに集中しないようにするためです。
- 出力は、直前 2 ステップの出力値を、余りの時間 (`accumulator / step`) で線形補間して書き込みます。表示のフレームレートと物理のステップが揃わなくても揺れが滑らかになります。代わりに、出力は最大 1 ステップ遅れます。
- 最初の `update` (と `reset` の後の最初の `update`) では、ステップを進める前に、その時点の入力に対する **静止姿勢** (各区間が力 `gravity * acceleration + wind` の向きに垂れた姿勢) に粒子を置きます。頭を傾けたまま読み込んでも、髪が揺れ始めません。

## 入力

各入力 (`PhysicsInput`) は、パラメータの値を正規化してから重み付きで加算します。

```
n = normalize(value, param.min, param.default, param.max, range)   // range は normalization.position または angle
if reflect: n = -n
translation.x += n * weight/100      (source = x)
translation.y += n * weight/100      (source = y)
angle         += n * weight/100      (source = angle、度)
```

`normalize` は、パラメータの default から min 側・max 側で別々に線形写像します。
`value` が default より大きければ `[default, max] → [range.default, range.max]`、小さければ `[min, default] → [range.min, range.default]` に写します。
default と等しい (または写す側の幅が 0) なら `range.default` です。

存在しないパラメータを参照する入力と、`source` が未知の入力は無視します。

## 振り子チェーン

粒子 0 は根元で、`translation + particles[0].position` に固定されます。粒子 i (i >= 1) ごとに、根元に近い順に、各ステップで次の計算をします。
`parent` はこのステップで更新済みの粒子 i-1 の位置です。

```
h        = (step / (1/30)) * delay                        // 基準フレーム単位のステップ幅
gravity' = rotate(settings.gravity, angle)                // 根元の回転だけ重力を回す
last     = position
position = parent + rotate(position - parent, Δangle * 0.2)   // 根元の回転に引きずられる
force    = gravity' * acceleration + settings.wind
position = position + velocity * h + force * h²
position = parent + normalize(position - parent) * radius     // 距離拘束
velocity = (position - last) / h * mobility^(step / (1/30))   // h = 0 なら 0
```

`Δangle` はこのステップでの根元の角度の変化量です。距離拘束で向きが決まらない (親と同じ位置) ときは、初期位置での親からの向きを使います。

### 係数の意味

| 係数 | 意味 | Cubism エディタでの名前 |
| --- | --- | --- |
| `radius` | 親の粒子からの距離 (振り子の長さ) | 振り子の長さ |
| `mobility` | 基準フレームあたりに残す速度の割合 (0〜1)。1 で減衰なし、小さいほど揺れがすぐ止まる | 揺れやすさ |
| `delay` | 時間の進み方の倍率。1 で等速、大きいほど入力に機敏に反応して速く揺れ、小さいほどゆっくり揺れる | 反応速度 |
| `acceleration` | 重力の倍率。大きいほど元の向きに戻ろうとする力が強く、揺れが速く収まる。風には掛からない | 収束の速さ |

- 根元の回転は 2 通りに効きます。**重力を根元の角度だけ回す** ので、頭を傾けると静止姿勢も同じ角度だけ傾きます (出力 `angle` の静止値 = 入力の角度)。また、角度が変わった瞬間に、各粒子を親のまわりに変化量の 0.2 倍 (`ROOT_ROTATION_FOLLOW`) だけ回して、鎖を回転に引きずらせます。
- `mobility` が 1 を超えると揺れが増幅して発散します。エディタでは 0〜1 に制限することを推奨します。

### 決め方の根拠 (T0301)

Cubism を並べて動かすことはできないため、公開されているマニュアルの説明 (各係数を 1 より大きく/小さくしたときに何が起きるか) と物理的な解釈から、上の式と係数の意味を決めました。互換の基準は「見た目の挙動が近いこと」で、数値が一致する必要はありません。

- 「反応速度」は 1 で等倍、1 より大きいと機敏になる、「収束の速さ」は 1 で等倍、1 より大きいと揺れが速く収まる、と説明されています。前者を時間の倍率 (`delay`)、後者を重力の倍率 (`acceleration`) として表しました ([How to Set Up Physics](https://docs.live2d.com/en/cubism-editor-manual/physical-operation-setting/)、[About Physics](https://docs.live2d.com/en/cubism-editor-manual/physics-operation/))。
- 基準フレームを 1/30 秒とし、`fps = 30` のとき `mobility` がそのまま 1 ステップあたりの速度の残存率になるようにしました。`fps` を上げると `mobility` を累乗して、単位時間あたりの減衰を揃えます。
- フィクスチャ (`ugoe_model::fixtures::physics_hair_settings`) で頭を 0.3 秒かけて動かすと、揺れの周期はおよそ 0.3〜0.7 秒、2 秒ほどで収まり、髪揺れとして自然な範囲に入ることを確かめました。
- Cubism の出力の角度はラジアンに scale を掛けたものです ([SDK Manual: Physics](https://docs.live2d.com/en/cubism-sdk-manual/physics/))。ugoe は度で扱うので、Cubism の設定を移すときは `scale` に `π / 180` を掛けます。

## 出力

```
source = x     → 粒子 i の位置 (1 つ前の粒子からの相対) の x 成分
source = y     → 同じく y 成分
source = angle → 粒子 i-2 → i-1 のベクトル (i = 1 なら settings.gravity。根元の角度で回さない) から
                 粒子 i-1 → i のベクトルへの符号付きの角度 (度、時計回りが正)
value = source * scale
if reflect: value = -value
value を出力パラメータの値域にクランプし、現在値に weight/100 の割合でブレンドする
  (param = param + (value - param) * weight/100)
```

`particle` が 0 または粒子数以上の出力、存在しないパラメータへの出力、`source` が未知の出力は無視します。
同じパラメータへの出力が複数あるときは、リグ・出力の順に続けてブレンドします。

## 決定性

物理演算も決定的です。同じ入力列 (パラメータ値の列と `dt` の列) からは、常に同じ出力が得られます。乱数は使いません。

## 動的物理 (揺れ用パラメータのいらない物理)

判断の経緯は [ADR 0011](../adr/0011-dynamic-physics.md)。

> 実装待ち: データ定義 (`ugoe_model::physics`) は T0304 で入れました。シミュレーションは T0305 で行います。パラメータへの出力 (上の節) は T0301 で実装済みです。
> 振り子チェーンの積分 (`ugoe_physics::chain::Chain::step`) は、距離拘束の直後に粒子の位置を補正するフック (衝突体からの押し出し用) と、重力・風を引数に取るので、T0305 はこれを使います。

### 出力先

`PhysicsOutput::target` (`PhysicsTarget`) で出力先を選びます。パラメータ以外の出力先が動的物理です。

| 出力先 (`type`) | フィールド | 内容 |
| --- | --- | --- |
| `parameter` | `id` (パラメータ) | 従来どおりパラメータに書き込む |
| `deformer_angle` | `deformer` (回転デフォーマ) | 回転デフォーマの角度に、粒子の区間の角度 (の `scale` 倍) を足す。粒子の区間とデフォーマを 1 対 1 に対応させると、デフォーマの鎖が揺れもののボーンのように動く |
| `deformer_offset` | `deformer` (回転デフォーマ) | 回転デフォーマの原点を、粒子の変位 (の `scale` 倍) だけずらす |
| `vertices` | `owner`、`weights` | ワープデフォーマの制御点 (`owner: {"warp": ID}`)、またはメッシュの頂点 (`owner: {"mesh": ID}`) を、重み付きで粒子の変位に追従させる。`weights` は頂点 (制御点) ごとの重み (0〜1) で、並びと長さは `Mesh::uvs` または `WarpKeyform::points` と同じ |

```json
{ "target": { "type": "deformer_angle", "deformer": "HairFront1" },
  "particle": 1, "source": "angle", "scale": 1.0, "weight": 100.0 }
{ "target": { "type": "vertices", "owner": { "mesh": "Skirt" }, "weights": [0.0, 0.5, 1.0] },
  "particle": 2, "source": "x", "scale": 1.0, "weight": 100.0 }
```

- 足し込む値はデフォーマの局所座標系で表します。キーフォームで補間した値に加算します。
- `particle`・`source`・`scale`・`weight`・`reflect` の意味はパラメータへの出力と同じです。
- 旧形式 (`target` の代わりに `"parameter": "HairFront"` を持つ出力) も読めます。`parameter` 出力先として読み、書き出しは常に `target` を使います。両方あれば `target` を使います。

### 衝突体

`PhysicsSettings::colliders` (`PhysicsCollider { id, name, attach, shape }`) に、円 (`{"type": "circle", "center", "radius"}`) とカプセル (`{"type": "capsule", "a", "b", "radius"}`。線分 a–b から `radius` 以内の領域) の衝突体を置けます。

- 衝突体はデフォーマに取り付け (`attach`)、そのデフォーマのワールド変換で動きます。形状の位置 (`center`・`a`・`b`) は取り付け先の局所座標系で、半径はキャンバスのピクセル単位で表します。`attach` を省略するとキャンバスに固定します。
- 各ステップの距離拘束の後、衝突体の内側に入った粒子を、衝突体の表面の最も近い点まで押し出します。
- 粒子には衝突用の半径 (`PhysicsParticle::collision_radius`、既定 0) を持たせ、押し出しはこの半径を含めて行います。既存の `radius` は粒子の間の距離 (区間の長さ) なので、衝突には使いません。
- 物理リグごとに、どの衝突体と衝突するかを衝突体の ID の列で選びます (`PhysicsRig::colliders`、既定は空)。

### 重力と風の基準

`PhysicsRig::frame` に基準のデフォーマを指定すると、重力と風のベクトルをそのデフォーマの座標系で解釈します。
たとえば頭の回転デフォーマを指定すると、頭を傾けたときに髪の垂れる向きも傾きます。既定 (省略、`None`) はキャンバスの座標系です。

### 評価との関係

動的物理はデフォーマのワールド座標を必要とするため、評価を 2 段階にします ([evaluation.md#2-評価の順序](evaluation.md#2-評価の順序))。

### 旧ランタイムでの扱い

任意機能 `physics.dynamic` ([ADR 0013](../adr/0013-forward-compatibility.md)) です。知らないランタイムは、パラメータ以外への出力と衝突体を無視します (揺れない)。
1.0 の基準の機能なので、1.0 以降のファイルでは宣言しません ([file-formats.md#10-の基準の機能](file-formats.md#10-の基準の機能))。

1.0 より後に出力先や衝突体の形状が増えたときは、知らない `type` を `Unknown` として読み、その出力・衝突体を無視します ([file-formats.md#未知の値の読み込み](file-formats.md#未知の値の読み込み))。

### 検証

参照先と値の検査は `validate_physics` で行います ([model.md#物理の検証](model.md#物理の検証))。
