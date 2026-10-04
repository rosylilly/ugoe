# ugoe-godot (Godot 4 の GDExtension)

`.ugoe` のモデルを Godot 4 (4.4 以降) の 2D のノードとして表示し、動かすための GDExtension。
[godot-rust (gdext)](https://github.com/godot-rust/gdext) で `ugoe::Puppet` を直接使う。

**ライセンス:** このクレートは MIT。依存する godot-rust のクレートは MPL-2.0 (ファイル単位のコピーレフト)。
ルートのワークスペースの外に置き、依存のライセンスは `node scripts/check-licenses.mjs --godot` で別に検査する
([ADR 0026](../../docs/adr/0026-godot-binding.md))。

## 構成

| パス | 内容 |
| --- | --- |
| `src/` | GDExtension (Rust)。ノード `UgoePuppet2D` と描画 |
| `example/addons/ugoe/` | 配るアドオン: `ugoe.gdextension`、`bin/` (ビルドしたライブラリ。`build.mjs` が置く)、`.ugoe` / `.ugoemod` をリソースとして読む GDScript (`ugoe_resource_loader.gd`・`ugoe_model.gd`・`ugoe_module.gd`) |
| `example/` | 見本のプロジェクト (`main.tscn`) と検査のスクリプト (`tests/`) |

リソースの読み込み器を GDScript で書いているのは、Godot がそれをワーカースレッドからも呼び、godot-rust のクラスは
メインスレッド以外から呼べないため (ADR 0026)。

## ビルドと見本

リポジトリのルートで:

```sh
just godot-build              # = node bindings/godot/build.mjs。ビルドして example/addons/ugoe/bin/ にライブラリ、example/models/ に見本のモデルを置く
just godot-build --release    # リリースビルド
godot --path bindings/godot/example        # 見本のシーンを開く (エディタなら `godot -e --path ...`)
```

- ビルドはルートの `target/` を使う (`ugoe` のクレートのビルド済みの成果物を使い回す)。`CARGO_TARGET_DIR` で変えられる。
- 見本 (`example/main.tscn`) は `assets/samples/` のモデルを選んで表示し、パラメータごとのスライダーと、表情・モーションの
  ボタン、着せ替えのモジュール (`dress-up.ugoe` 用) のチェックボックスを作る。ライティングを受けるモデル (`avatar-lit.ugoe`)
  では、マウスの位置に点光源を置く。触っていない間は `AngleX` を揺らす。クリックした位置の当たり判定を出力に出す。
  起動の引数で最初のモデルとモジュールを選べる: `godot --path bindings/godot/example -- --model=dress-up.ugoe --modules=sailor.ugoemod`。
- ほかのプロジェクトで使うには、`example/addons/ugoe/` をそのプロジェクトの `addons/ugoe/` に写す (下の「配布」)。

## 検査

```sh
just godot-check        # = build.mjs --check --conformance (Godot と、画面が無ければ xvfb-run が要る)
node bindings/godot/build.mjs --check        # 依存のライセンス + ヘッドレスの Godot で example/tests/smoke.gd
node bindings/godot/build.mjs --conformance  # 配布物の適合テストを Godot で (= node tests/conformance/run.mjs --only godot。画面が要る。無ければ xvfb-run)
node bindings/godot/build.mjs --conformance --only masks-moving   # 計画のケースを絞る (複数可)
cargo test --manifest-path bindings/godot/Cargo.toml   # Godot なしで確かめられる部分 (src/model.rs・src/shader.rs)
cargo clippy --manifest-path bindings/godot/Cargo.toml --all-targets -- -D warnings
```

- Godot の実行ファイルは環境変数 `GODOT` (既定は `godot`) で指定する。見つからなければ入れ方を示して止まる。
- `--conformance` は、配布物の適合テスト ([tests/conformance/README.md](../../tests/conformance/README.md)) の計画のケースを、
  実行器 `example/tests/conformance.gd` で倍率 1・背景 `#808080` で描き、`ugoe-conformance compare --images` で
  `ugoe render` の画像と、ノードから読み戻した評価の結果 (頂点位置・不透明度・描画順の値・乗算色・スクリーン色・マスクの効き具合・
  `visible`・描く順番) を参照と比べる (T0418 で、Godot だけにあったケースと比べ方を適合テストに移した。評価の結果は T0420 から)。
  基準は [docs/spec/rendering.md#互換の基準](../../docs/spec/rendering.md#互換の基準)。ケースは見本 (`avatar`・`avatar-lit`・着せ替え)、
  適合テストが `ugoe_model::fixtures` から作るモデル (入れ子・反転・共通部分のマスク、3 つの合成モード、乗算色とスクリーン色、
  裏面カリング、1 つの要素が 12 個のマスクを読む `masks-many`)、アニメーション中のマスク (`before` のあるケース: 値を変えた
  **直後のフレーム** を比べる。`masks-moving`・`avatar-moving`。入れ子のマスクが 1 フレーム遅れると合わない)、時間を進めた物理と目パチ・呼吸、
  受け手として進めるケース (`*-follow`。`advance` の代わりに `follow` を呼ぶ。T0449)、
  バンドルの例、公開仕様の評価の例 (モデルの JSON。`load_model_from_json` で読み、テクスチャが無いので白で描く)。
- `--conformance` は続けて `example/tests/ktx2.gd` を実行する (`--only` のときは飛ばす)。見本のモデルのテクスチャを、同じ画素の
  KTX2 (`R8G8B8A8_SRGB`・`R8G8B8A8_UNORM`、Godot の `Image.compress` で作る `BC3_UNORM_BLOCK`) に差し替えたバンドルを描き、
  PNG のバンドル (BC3 は展開した画素の PNG) の画像と画素ごとに比べる (適合テストの参照の `ugoe render` は KTX2 を読めないため)。
- 1 フレームの時間は `example/tests/bench.gd` で測る (検査ではない。見本のモデルのパラメータを毎フレーム動かし、`_process` に
  かかった時間を出す。`node bindings/godot/build.mjs --release` の後に
  `xvfb-run -a godot --path bindings/godot/example --rendering-driver opengl3 --script res://tests/bench.gd`)。
- これらは `just ci` に含まれない (ルートのワークスペースの外にあるため)。CI では別のジョブ (`.github/workflows/ci.yml` の
  `godot`) が、版を固定した Godot (4.7.1) と Xvfb の上で `just godot-check` を実行する。

## 使い方 (GDScript)

```gdscript
var puppet := UgoePuppet2D.new()
puppet.model = load("res://models/avatar.ugoe")   # UgoeModel。木に入ると読み込む
add_child(puppet)

puppet.set_parameter("AngleX", 15.0)              # インスペクタの parameters/AngleX でも設定できる
puppet.start_motion("idle", 0)                    # 0 = 待機、1 = 普通、2 = 強制
puppet.set_expression("smile")
puppet.set_lip_sync_level(0.8)
puppet.motion_event.connect(func(value): print(value))

# 着せ替え (docs/spec/modules.md#puppet-での着せ替え)
puppet.attach_module(load("res://models/sailor.ugoemod"))   # 同じスロットのモジュールとは差し替える
puppet.detach_module("Sailor")

# ライティング (docs/spec/rendering.md#ライティング。座標はキャンバス座標 + 奥行き)
if puppet.uses_lighting():
    puppet.ambient_light = Color(0.4, 0.4, 0.45)
    puppet.add_directional_light(Vector3(1, 1, -1), Color.WHITE)       # 光が進む向き
    puppet.add_point_light(Vector3(512, 300, 250), Color(1, 0.9, 0.8), 900.0)
    puppet.use_scene_lights = true      # シーンの PointLight2D / DirectionalLight2D も光源にする

# フェイストラッキング (docs/spec/avatar-profile.md#フェイストラッキングとの対応)
puppet.set_tracking_head(Vector3(yaw, pitch, roll))                 # 度
puppet.set_tracking_blend_shapes({"jawOpen": 0.4, "eyeBlinkLeft": 1.0})

# XR のフェイストラッキング (XRFaceTracker、Unified Expressions)。名前を設定すると毎フレーム読む
puppet.xr_face_tracker = &"/user/face_tracker"
# か、手で渡す: puppet.set_tracking_from_xr_face_tracker(XRServer.get_tracker(&"/user/face_tracker"))
```

| メンバー | 内容 |
| --- | --- |
| `model` | 表示するモデル (`UgoeModel`。`.ugoe` を `load()` したもの) |
| `playing` / `speed` | 時間を進めるか / 速さ。止めている間もパラメータの変更は描き直す。`advance(delta)` で手で進められる |
| `play_in_editor` | エディタの中でも時間を進めるか (既定は止める) |
| `mask_scale` | マスクの解像度の倍率。1 で画面上の大きさと同じ |
| `get_mask_resolution_factor()` | マスクがアトラスに収まらずに解像度に掛けている倍率 (1 なら下げていない。下の「描画のしかた」) |
| `modules` | 取り付ける着せ替えのモジュール (`UgoeModule` の配列、合成の順)。シーンに保存される。モデルを読むときに付ける |
| `ambient_light` / `lights` | 環境光 (既定は白) / 光源の辞書の配列 (`{ "type": "directional", "direction": Vector3, "color": Color }` か `{ "type": "point", "position": Vector3, "color": Color, "range": float }`)。シーンに保存される |
| `use_scene_lights` / `get_scene_lights()` | シーンの `Light2D` も光源にする (`lights` の後ろに足す。下の「Light2D から取る光源」) / それから求めた今の光源 (`lights` と同じ形の辞書) |
| `motion_over_tracking` / `tracking_kept` | トラッキング中に、再生中のモーションが動かすパラメータではモーションを優先するか (既定は false。[docs/spec/motion.md#トラッキング中のモーションの優先](../../docs/spec/motion.md#トラッキング中のモーションの優先)) / それでもトラッキングのままにするパラメータの ID (`PackedStringArray`。目など。モデルに無い ID は無視)。シーンに保存され、モデルを読み直しても保つ |
| `xr_face_tracker` | トラッキングの入力にする `XRFaceTracker` の名前 (`XRServer` に登録した名前、標準は `/user/face_tracker`)。空なら使わない。毎フレーム読み、見つからない間は前の値を保つ |
| `parameters/<ID>` | インスペクタに出るパラメータ (シーンに保存される) |
| `load_model(path)` / `load_model_from_buffer(bytes)` / `load_model_from_json(text)` | 読み込む (`load_model` は `model` を設定する。`from_buffer` と `from_json` は `model` を null にする)。`from_json` はモデルの JSON (テクスチャ・物理・モーションは無く、テクスチャは不透明な白で描く) |
| `get_parameter_ids()` / `get_parameter_info(id)` / `get_parameter(id)` / `set_parameter(id, value)` / `reset_parameters()` | パラメータ |
| `get_load_warnings()` | 直前に読み込んだモデルの読み込みの警告 (`{ "code", "warning", "texture"? / "feature"? / "path"? }` の配列)。`code` と補足のキーは wasm の `load_warnings_json`・C ABI の `ugoe_puppet_load_warning_*` と同じ ([crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md#読み込みの警告-t0125))。`warning` は英語の文で、安定した値ではない |
| `get_module_load_warnings(name)` | 取り付けている名前が `name` の着せ替えのモジュール (`.ugoemod`) の読み込みの警告 (T0456)。要素の形は `get_load_warnings()` と同じで、wasm の `modules_json()` の `warning_details`・C ABI の `ugoe_module_warning_*` に揃える。付いているモジュールの探し方は `get_module_names()` と同じで、付いていなければ空 |
| `get_motion_names()` / `start_motion(name, priority)` / `stop_motions()` / `is_motion_playing()` | モーション |
| `get_expression_names()` / `set_expression(name)` / `stop_expressions()` | 表情 (`stop_expressions` はフェードアウトをかけてすべて止める) |
| `set_lip_sync_level(level)` / `clear_lip_sync_level()` / `reset_physics()` | リップシンク・物理 |
| `set_effects_json(json)` | 目パチ・呼吸・リップシンクの設定をモデルの `effects` と同じ形の JSON で差し替える (再生の状態は初めから、リップシンクの音量は保つ。[docs/spec/motion.md#エフェクト設定の差し替え](../../docs/spec/motion.md#エフェクト設定の差し替え))。読めなければ false |
| `set_physics_json(json)` / `set_base_physics_json(json)` / `get_physics_json()` / `get_base_physics_json()` | 物理の設定を `.ugoe` の物理と同じ形の JSON で差し替える (空の文字列で外す) / 読む (無ければ空)。`base` は本体の設定で、着せ替えのモジュールを付けている間も保ち、付け外しのたびにこれと合成する ([docs/spec/modules.md#puppet-での着せ替え](../../docs/spec/modules.md#puppet-での着せ替え))。`set_physics_json` の設定は次の付け外しで失われる。モデルに合わなければ false (何も変えない) |
| `get_parameter_values()` / `set_base_values(values)` / `follow(values, delta, lip_sync_level, lip_sync_strength, part_opacities)` | 受け手 ([docs/spec/motion.md#受け手として進める](../../docs/spec/motion.md#受け手として進める))。値は `get_parameter_ids()` の順の `PackedFloat32Array`。`follow` は指揮役の値を基準値にして、物理 → リップシンク (`lip_sync_level` が負なら当てない) → パーツの不透明度 (`part_opacities` は `{ "part": get_part_ids() の番号, "value", "weight" }` の配列) → 評価の順に進めて描く。受け手のノードは `playing` を false にして毎フレーム `follow` を呼ぶ (ノードは読み込みの直後に 1 度評価して物理の粒子を置くので、指揮役と同じ形から始めるなら最初の `follow` の前に `reset_physics()` を呼ぶ。適合テストの `*-follow` のケースで C ABI と同じ値になることを確かめている)。失敗したら false (何も変えない) |
| `get_part_ids()` / `get_part_opacity(id)` / `get_part_evaluated_opacity(id)` / `show_pose(part)` | パーツの ID (モデルの順) / 直前の評価でパーツに書いた不透明度の上書き (`{ "value", "weight" }`。モーションのカーブとポーズの結果) / 祖先を掛けた評価後の不透明度 / ポーズで `part` を持つ要素を表示する (フェードで切り替わる。ポーズの要素のパーツでなければ false) |
| `set_stage_enabled(stage, enabled)` / `is_stage_enabled(stage)` | `update` の段 (`"physics"`・`"eye_blink"`・`"breath"`・`"lip_sync"`・`"pose"`) を動かすか (既定はすべて動かす)。モデルを読み直しても保つので、読み込む前に設定すれば読み込みの直後の評価から効く。未知の名前は false |
| `attach_module(module)` / `detach_module(name)` / `clear_modules()` / `get_module_names()` | 着せ替え。付けられなければエラーを出して false (何も変えない)。`get_module_names` は、モデルを読み込んでいれば実際に付いているものだけを返す (読み込むときに付けられなかったモジュールは `modules` に残るが、ここには出ない) |
| `uses_lighting()` / `add_directional_light(direction, color)` / `add_point_light(position, color, range)` / `clear_lights()` | ライティング (光源は先頭の 8 個まで) |
| `set_tracking_from_xr_face_tracker(tracker)` | `XRFaceTracker` のブレンドシェイプを ARKit の形に直して設定する (下の「XRFaceTracker との対応」)。頭の回転は変えない。`XRFaceTracker` でなければ false |
| `set_tracking_head(rotation)` / `set_tracking_body(dict)` / `set_tracking_blend_shapes(dict)` / `set_tracking_blend_shape(name, value)` / `clear_tracking()` / `is_tracking()` / `get_tracking_blend_shape_names()` | トラッキング。頭は度で `x` = ヨー・`y` = ピッチ・`z` = ロール。体は `yaw` `pitch` `roll` `shoulder_left` `shoulder_right` (無いキーは省略)。ブレンドシェイプは ARKit の 52 個の名前 (MediaPipe の名前もそのまま渡せる)。平滑化はホストが行う |
| `hit_test(id, local_position)` / `get_hit_areas_at(local_position)` | 当たり判定 (ノードの座標で渡す) |
| `get_standard_hit_area(name)` / `hit_test_standard(name, local_position)` / `get_standard_hit_areas_at(local_position)` | 標準の当たり判定 (`"Head"`・`"Body"`。`Model.avatar.hit_areas`、[docs/spec/avatar-profile.md#アプリ向けの情報](../../docs/spec/avatar-profile.md#アプリ向けの情報))。対応する ID (無ければ空) / 含むか / 含む標準の名前を手前から |
| `get_slot_ids()` / `get_tracking_targets()` | 本体のモデルの着せ替えのスロットの ID (`UgoeModule` のスロットと比べる) / トラッキングが書き込むパラメータの ID (モデルの `avatar.tracking` から決まる) |
| `get_canvas_size()` / `get_canvas_origin()` | キャンバスの大きさと原点 |
| `get_drawable_ids()` / `get_drawable_positions(index)` / `get_drawable_state(index)` / `get_render_order()` | 直前の評価の結果の読み戻し (`docs/spec/evaluation.md#8-出力`。適合テストで使う)。番号は `get_drawable_ids()` の順 (モデルのメッシュの順)。位置は **キャンバス座標** (ノードの座標は `get_canvas_origin()` を引く)。`get_drawable_state` は `{ "opacity", "draw_order", "multiply_color" (Color), "screen_color" (Color), "mask_strength", "visible" }`。`get_render_order` は奥から手前への番号。範囲の外の番号は空 |
| シグナル `model_loaded` / `motion_event(value)` | 読み込み / モーションのイベント |

有限でない値 (GDScript の `NAN`・`INF`) は C ABI と同じく受け付けない: `advance(delta)` と `speed` は有限で負でない値、
`set_parameter`・`set_lip_sync_level` は NaN 以外 (無限大は値域にクランプ)、`set_tracking_head` と `mask_scale` は有限、
`set_tracking_body` は NaN のキーを省略とみなし無限大を拒む、ブレンドシェイプは有限でない値を無視する、環境光と光源の色は有限で負でない値
(それ以外の光源は警告を出して使わない)、`set_base_values` と `follow` の値は NaN 以外 (無限大は値域にクランプ)、`follow` の `delta` は
有限で負でない値、リップシンクの音量 (当てるとき) と強さ・パーツの不透明度の値と重みは有限 (`ugoe_puppet_follow` と同じ)。
拒んだ値はエラーを出して捨て、前の値を保つ。ほかの入口 (SDK・C ABI・wasm・Unity) と並べた表は
[docs/portability.md#ホストから来る数](../../docs/portability.md#ホストから来る数)。
`set_effects_json`・`set_physics_json`・`set_base_physics_json` と `show_pose` で変えたものは、モデルを読み直すとモデルの設定に戻る。
読み込みの警告 (知らない任意の機能・未知の値 (ADR 0013)・画像の無いテクスチャ) は Godot の警告として出し、`get_load_warnings()` でも読める
(着せ替えのモジュールの警告も Godot の警告として出し、付けている間は `get_module_load_warnings(name)` で読める)。
木の外で `model` を変えたときは、木に戻ったときに読み込む (外す)。

ノードの原点にキャンバスの原点 (`canvas.origin`) が来る。座標はキャンバスのピクセル単位で、Godot の 2D と同じく Y 下向き。
光源の位置はキャンバス座標 (ノードの座標 + `get_canvas_origin()`) で、ノードの変形は陰影に影響しない。ノードの `modulate` は色に掛かる。
`UgoeModel.from_buffer(bytes)` / `UgoeModule.from_buffer(bytes)` でバイト列からリソースを作れる。

`.ugoe` と `.ugoemod` はリソースとして書き出しに含まれる (エクスポートの設定は要らない)。

### Light2D から取る光源

`use_scene_lights` を true にすると、このノードと同じビューポートにある `PointLight2D` と `DirectionalLight2D` を毎フレーム読み、
キャンバス座標の光源 ([docs/spec/rendering.md#ライティング](../../docs/spec/rendering.md#ライティング)) に直して `lights` の後ろに足す
(合わせて先頭の 8 個まで)。ライトを動かすと陰影が付いてくる。ライトの足し外しは木の `node_added` / `node_removed` で気づく。

- 使うライト: `enabled` で、木の中で見えていて (`is_visible_in_tree`)、同じキャンバス (同じ `CanvasLayer`) にあり、
  `range_item_cull_mask` がこのノードの `light_mask` と重なるもの。`editor_only` のライトはエディタの中だけで使う。
- 色: `color × energy`。
- `PointLight2D`: 位置はライトの位置 (このノードの変形を外してキャンバス座標に直す)、奥行きは `height` (ピクセル、手前が正。
  このノードの拡大縮小で直す)。`range` はテクスチャの半径 (幅と高さの大きい方の半分 × `texture_scale` × ライトの拡大縮小)。
  テクスチャが無ければ減衰しない。
- `DirectionalLight2D`: 光はライトの +Y の向き (回転 0 なら画面の下向き) に進み、`height` (0〜1) の分だけ画面の奥へ傾く
  (0 で画面と平行、1 で正面から)。
- 使わないもの: テクスチャの形 (明るさの分布。減衰は参照レンダラと同じ式)、`blend_mode` (いつも足す)、影、Z と層の範囲、
  `DirectionalLight2D.max_distance`。Godot の 2D のライトの見た目そのものではなく、ugoe のライティングの式で照らす。

### XRFaceTracker との対応

Godot の `XRFaceTracker` は Unified Expressions (`BlendShapeEntry` の 143 個) の値を持つ。`xr_face_tracker` か
`set_tracking_from_xr_face_tracker` で、それを ARKit の 52 ブレンドシェイプ ([docs/spec/avatar-profile.md#arkit-のブレンドシェイプ](../../docs/spec/avatar-profile.md#arkit-のブレンドシェイプ)
の入力) に直して渡す (`src/xr.rs`)。左右はどちらもトラッキングされた人の左右なので入れ替えない。

| ARKit | Unified Expressions (`FT_` を省く) |
| --- | --- |
| `eyeBlink*` | `EYE_CLOSED_*` |
| `eyeLookDown*` / `eyeLookIn*` / `eyeLookOut*` / `eyeLookUp*` | `EYE_LOOK_DOWN_*` / `EYE_LOOK_IN_*` / `EYE_LOOK_OUT_*` / `EYE_LOOK_UP_*` |
| `eyeSquint*` / `eyeWide*` | `EYE_SQUINT_*` / `EYE_WIDE_*` |
| `jawForward` / `jawLeft` / `jawRight` / `jawOpen` | `JAW_FORWARD` / `JAW_LEFT` / `JAW_RIGHT` / `JAW_OPEN` |
| `mouthClose` | `MOUTH_CLOSED` |
| `mouthFunnel` | `LIP_FUNNEL_{UPPER,LOWER}_{RIGHT,LEFT}` の平均 (`LIP_FUNNEL` が大きければそれ) |
| `mouthPucker` | `LIP_PUCKER_{UPPER,LOWER}_{RIGHT,LEFT}` の平均 (`LIP_PUCKER` が大きければそれ) |
| `mouthLeft` / `mouthRight` | `MOUTH_UPPER_*` と `MOUTH_LOWER_*` の平均 (`MOUTH_LEFT` / `MOUTH_RIGHT` が大きければそれ) |
| `mouthSmile*` | `MOUTH_CORNER_PULL_*`・`MOUTH_CORNER_SLANT_*`・`MOUTH_SMILE_*` の最大 |
| `mouthFrown*` / `mouthDimple*` / `mouthStretch*` / `mouthPress*` | `MOUTH_FROWN_*` / `MOUTH_DIMPLE_*` / `MOUTH_STRETCH_*` / `MOUTH_PRESS_*` |
| `mouthRollLower` / `mouthRollUpper` | `LIP_SUCK_LOWER_*` / `LIP_SUCK_UPPER_*` の平均 (`LIP_SUCK_LOWER` / `LIP_SUCK_UPPER` が大きければそれ) |
| `mouthShrugLower` / `mouthShrugUpper` | `MOUTH_RAISER_LOWER` / `MOUTH_RAISER_UPPER` |
| `mouthLowerDown*` / `mouthUpperUp*` | `MOUTH_LOWER_DOWN_*` / `MOUTH_UPPER_UP_*` |
| `browDown*` | `BROW_LOWERER_*`・`BROW_PINCH_*`・`BROW_DOWN_*` の最大 |
| `browInnerUp` | `BROW_INNER_UP_RIGHT` と `BROW_INNER_UP_LEFT` の平均 |
| `browOuterUp*` | `BROW_OUTER_UP_*` |
| `cheekPuff` | `CHEEK_PUFF_RIGHT` と `CHEEK_PUFF_LEFT` の平均 (`CHEEK_PUFF` が大きければそれ) |
| `cheekSquint*` / `noseSneer*` / `tongueOut` | `CHEEK_SQUINT_*` / `NOSE_SNEER_*` / `TONGUE_OUT` |

(`*` は `Left` / `Right` と `_LEFT` / `_RIGHT` の対応。)

- 平均は「同時に動く部分を合わせた量」(唇のすぼめ・巻き込みなど)、最大は「どちらか一方でその表情になる」(笑いの口角・眉の下げ) の形に使う。
- `FT_EYE_CLOSED` 以降の合成の形 (左右や上下をまとめたもの) は、トラッカーが書くとは限らないので、書かれていれば基本の形から求めた値との大きい方を使う。
- 対応が無い形 (瞳孔・鼻孔・頬のへこみ・舌の細かい動き・喉・首など) は使わない。値は 0〜1 にクランプする。
- `XRFaceTracker` に頭の回転は無い。頭は `set_tracking_head` で渡す (HMD やカメラの向きから求める方法はアプリが決める)。
- 実機のトラッカー (OpenXR のフェイストラッキングの拡張など) では確かめていない。`example/tests/smoke.gd` で値を流し込んで確かめている。

## 描画のしかた

- 評価 (`ugoe::Puppet::update`) の結果を、描画可能要素ごとのキャンバスアイテム (`RenderingServer`) と `ShaderMaterial` で描く。
  描画順は draw index で決め、頂点位置は変わった要素だけ送り直す。
- テクスチャは読み込み時に乗算済みアルファに変換する (法線マップは変換しない)。式・合成モードの表し方は `src/shader.rs` の冒頭を参照。
- KTX2 は Godot の `Image.load_ktx_from_buffer` で読み、PNG と同じく乗算する ([docs/spec/rendering.md#前提](../../docs/spec/rendering.md#前提))。
  - Godot 4.7 は `R8G8B8(A8)_SRGB` をリニアに変換して読み (8 ビットのまま。戻せない)、`R8G8B8(A8)_UNORM` は読めず、
    転送関数が sRGB の Basis Universal は RGBA8 に展開してリニアに変換する。仕様では値をそのまま使うので、読む前にヘッダを
    書き換える (`src/model.rs` の `ktx2_as_stored`。8 ビットの非圧縮の形式は同じ大きさの `*_UINT` に、転送関数は linear に)。
    法線マップも同じ。Godot の挙動は [modules/ktx/texture_loader_ktx.cpp](https://github.com/godotengine/godot/blob/4.7.1-stable/modules/ktx/texture_loader_ktx.cpp) で調べた。
  - 圧縮した形式は `Image.decompress` で展開してから乗算する (GPU では圧縮しないで持つ)。展開できない形式は警告を出し、
    乗算せずにそのまま描く。Godot が読めない形式 (`B8G8R8A8_*` など) は、PNG と同じく警告を出して白で描く。
  - ミップマップは捨てて最初の段だけを使う。
  - 確かめたもの: `example/tests/ktx2.gd` (下の「検査」) で `R8G8B8A8_SRGB`・`R8G8B8A8_UNORM`・`BC3_UNORM_BLOCK`。
    Basis Universal と ETC2・ASTC は確かめていない (テストで作る道具が無い)。
  ライティングは参照レンダラ (`crates/ugoe-render/src/shaders/drawable.wgsl`) と同じ式で、法線は画面での微分から求めたヤコビ行列で回す。
- マスクは、マスクとして参照されるメッシュのカバレッジを 1 枚のアトラス (`SubViewport`) に並べて描き、要素はキャンバス座標から読む
  (ノードの変形に左右されない)。どのカバレッジがアトラスのどこにあるか・要素がどのマスクを読むかは浮動小数点のテクスチャの表
  (`src/model.rs` の `MaskTable`) に書き、シェーダはループで読む (読めるマスクの数に上限はない)。
  カバレッジの画素の格子はキャンバスの `1 / 解像度` の格子に揃える (倍率 1 なら画面の画素と重なる)。
- 入れ子のマスク (マスク用メッシュ自身がマスクを読む) は段に分け、段ごとのアトラスのビューポートを前の段のビューポートの親にする。
  Godot は子のビューポートを先に描くので、同じフレームのうちに段の順に描かれる。
- アトラスの 1 辺は 4096 画素まで。収まらなければ、収まるまでマスクの解像度に 0.75 を掛けて下げる (`get_mask_resolution_factor()`)。
  下げている間にマスクが動いて小さくなり、1 段上げた解像度の配置が 1 辺 3584 画素 (4096 の 7/8) に収まるようになったら、1 段戻す
  (余裕を持たせて、境目で上げ下げを繰り返さないようにする。`src/model.rs` の `MaskShrink`)。`mask_scale` か画面上の大きさが
  変わったら 1 から測り直す。
  - 画質の目安 (適合テストのマスクのケースを `mask_scale` を下げて描き、`ugoe render` と比べた。Compatibility の Xvfb): 下げても
    違うのはマスクの縁だけで、倍率 0.75 では 3/255 を超える画素が 0.1〜0.4% (縁の多い `masks-many` で 3%)、最大 25/255 ほど
    (縁が 1 画素ほどぼける)。0.56 で最大 30〜90/255、0.42 以下では縁の段差が見え、最大 100/255 を超える (超える画素は 0.3〜0.9%、
    `masks-many` で 4〜7%)。解像度を下げたマスクは互換の基準の対象外 (docs/spec/rendering.md#互換の基準)。
  - 下がるのはマスクが多く、画面上で大きいときだけ (目安として、マスクの外接矩形の面積の合計が 4096² に近いとき。棚詰めの
    隙間があるので、それより小さくても下がることがある)。
    下がるなら `mask_scale` を下げるか、マスクを読む要素を減らす。
- 着せ替えではレンダラを作り直す。変わっていないテクスチャ (`ugoe::DressUpChange`) はデコードし直さない。
- 裏面カリングは CPU で三角形を選ぶ (向きはキャンバス座標だけで決まるため)。

## 配布 (Godot のアセットライブラリ)

アセットライブラリには、アドオンのフォルダ (`addons/ugoe/`) を含む zip を GitHub のリリースに置き、そのリリースを登録する。
**公開は利用者が行う** (ここでは手順だけ。リリースの自動化は ADR 0017 の後続)。

1. 各 OS でリリースビルドを作る: `just godot-build --release`。ライブラリの名前は `.gdextension` の `[libraries]` と合わせる
   (Linux x86_64 / arm64 の `libugoe_godot.so`、Windows x86_64 の `ugoe_godot.dll`、macOS の `libugoe_godot.dylib`)。
   Linux の arm64 と Linux・Windows を分けるなら、`.gdextension` の名前を `libugoe_godot.linux.x86_64.so` などに変える。
   macOS は universal (x86_64 と arm64 を `lipo` でまとめる) にし、配るなら署名と公証をする。
2. 配るフォルダを作る: `example/addons/ugoe/` (`ugoe.gdextension`・`*.gd` と Godot が作る `*.uid`・`bin/`) に、リポジトリの `LICENSE` (MIT) と、
   godot-rust (MPL-2.0) のライセンスの告知 (ソースの入手先 https://github.com/godot-rust/gdext) を足す。
3. 新しい Godot のプロジェクトに写し、エディタで開いて見本のシーンと同じことができるか (モデルの表示・`.ugoe` がリソースとして
   見えるか・書き出したゲームで動くか) を確かめる。
4. `addons/ugoe/` だけを含む zip を GitHub のリリースに置く。
5. https://godotengine.org/asset-library/asset/submit で登録する: カテゴリ「2D Tools」、Godot の版「4.4」、ライセンス「MIT」、
   リポジトリの URL、ダウンロード (リリースの zip の URL) とコミット、アイコン (256x256 の PNG)、スクリーンショット、説明
   (Live2D のファイルは読めないこと、`.ugoe` をエディタで作ること)。審査の後に公開される。版を上げるときは同じ手順で更新を出す。

## 制限 (後続の候補)

- 画面のある Godot のエディタでの操作、Forward+ / Mobile のレンダラ (Vulkan) での見た目、Windows・macOS でのビルドは確かめていない
  (比べ合わせは Compatibility (OpenGL) の Xvfb だけ)。
- `Light2D` から取る光源は、テクスチャの形・影・合成のモードを使わない (上の「Light2D から取る光源」)。
- 指揮役の側の値 (`ugoe::Puppet::values_before_physics`・`lip_sync_applied`・`part_opacities_applied`) は出していない
  (C ABI・wasm にも無い)。Godot のノードを指揮役にして別のノードを `follow` で動かすには、これらを足す必要がある
  (`get_parameter_values()` は物理の後の値なので、それを渡しても物理を持つモデルでは指揮役と同じ形にならない)。
- 着せ替えのモジュールの読み込みの警告は、取り付けている間だけ読める (`get_module_load_warnings(name)`。`UgoeModule` は
  GDScript のリソースで中身を読まないため、付ける前のモジュールからは読めない)。
- 当たり判定・パラメータなどの Godot の API の自動テストは、ヘッドレスの Godot で動く `example/tests/smoke.gd` だけ。
- Godot 4.7.1 で、GDExtension のクラスを持つプロジェクトの初回の `--import` の終わりに異常終了することがある
  (godot-rust の最小のクラスでも起き、2 回目からは起きない)。`build.mjs` は 1 回だけやり直す。

## 関連タスク

- T0406 (`docs/tasks/T0406-godot-extension.md`)、T0411 (`docs/tasks/T0411-godot-followups.md`)、T0415 (`docs/tasks/T0415-godot-distribution.md`)
