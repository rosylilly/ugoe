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
  実行器 `example/tests/conformance.gd` で倍率 1・背景 `#808080` で描き、`ugoe-conformance compare --images-only` で
  `ugoe render` の画像と比べる (T0418 で、Godot だけにあったケースと比べ方を適合テストに移した)。
  基準は [docs/spec/rendering.md#互換の基準](../../docs/spec/rendering.md#互換の基準)。ケースは見本 (`avatar`・`avatar-lit`・着せ替え)、
  適合テストが `ugoe_model::fixtures` から作るモデル (入れ子・反転・共通部分のマスク、3 つの合成モード、乗算色とスクリーン色、
  裏面カリング、1 つの要素が 12 個のマスクを読む `masks-many`)、アニメーション中のマスク (`before` のあるケース: 値を変えた
  **直後のフレーム** を比べる。`masks-moving`・`avatar-moving`。入れ子のマスクが 1 フレーム遅れると合わない)、時間を進めた物理と目パチ・呼吸、
  バンドルの例。Godot はモデルの JSON を読まないので、公開仕様の評価の例 (JSON) は飛ばす (`skipped.json`)。
  頂点は比べない (Godot の中で評価しているのは C ABI・wasm と同じ `ugoe::Puppet`)。
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

# フェイストラッキング (docs/spec/avatar-profile.md#フェイストラッキングとの対応)
puppet.set_tracking_head(Vector3(yaw, pitch, roll))                 # 度
puppet.set_tracking_blend_shapes({"jawOpen": 0.4, "eyeBlinkLeft": 1.0})
```

| メンバー | 内容 |
| --- | --- |
| `model` | 表示するモデル (`UgoeModel`。`.ugoe` を `load()` したもの) |
| `playing` / `speed` | 時間を進めるか / 速さ。止めている間もパラメータの変更は描き直す。`advance(delta)` で手で進められる |
| `play_in_editor` | エディタの中でも時間を進めるか (既定は止める) |
| `mask_scale` | マスクの解像度の倍率。1 で画面上の大きさと同じ |
| `modules` | 取り付ける着せ替えのモジュール (`UgoeModule` の配列、合成の順)。シーンに保存される。モデルを読むときに付ける |
| `ambient_light` / `lights` | 環境光 (既定は白) / 光源の辞書の配列 (`{ "type": "directional", "direction": Vector3, "color": Color }` か `{ "type": "point", "position": Vector3, "color": Color, "range": float }`)。シーンに保存される |
| `parameters/<ID>` | インスペクタに出るパラメータ (シーンに保存される) |
| `load_model(path)` / `load_model_from_buffer(bytes)` | 読み込む (`load_model` は `model` を設定する。`from_buffer` は `model` を null にする) |
| `get_parameter_ids()` / `get_parameter_info(id)` / `get_parameter(id)` / `set_parameter(id, value)` / `reset_parameters()` | パラメータ |
| `get_motion_names()` / `start_motion(name, priority)` / `stop_motions()` / `is_motion_playing()` | モーション |
| `get_expression_names()` / `set_expression(name)` | 表情 |
| `set_lip_sync_level(level)` / `clear_lip_sync_level()` / `reset_physics()` | リップシンク・物理 |
| `set_stage_enabled(stage, enabled)` / `is_stage_enabled(stage)` | `update` の段 (`"physics"`・`"eye_blink"`・`"breath"`・`"lip_sync"`・`"pose"`) を動かすか (既定はすべて動かす)。モデルを読み直しても保つので、読み込む前に設定すれば読み込みの直後の評価から効く。未知の名前は false |
| `attach_module(module)` / `detach_module(name)` / `clear_modules()` / `get_module_names()` | 着せ替え。付けられなければエラーを出して false (何も変えない) |
| `uses_lighting()` / `add_directional_light(direction, color)` / `add_point_light(position, color, range)` / `clear_lights()` | ライティング (光源は先頭の 8 個まで) |
| `set_tracking_head(rotation)` / `set_tracking_body(dict)` / `set_tracking_blend_shapes(dict)` / `set_tracking_blend_shape(name, value)` / `clear_tracking()` / `is_tracking()` / `get_tracking_blend_shape_names()` | トラッキング。頭は度で `x` = ヨー・`y` = ピッチ・`z` = ロール。体は `yaw` `pitch` `roll` `shoulder_left` `shoulder_right` (無いキーは省略)。ブレンドシェイプは ARKit の 52 個の名前 (MediaPipe の名前もそのまま渡せる)。平滑化はホストが行う |
| `hit_test(id, local_position)` / `get_hit_areas_at(local_position)` | 当たり判定 (ノードの座標で渡す) |
| `get_canvas_size()` / `get_canvas_origin()` | キャンバスの大きさと原点 |
| シグナル `model_loaded` / `motion_event(value)` | 読み込み / モーションのイベント |

ノードの原点にキャンバスの原点 (`canvas.origin`) が来る。座標はキャンバスのピクセル単位で、Godot の 2D と同じく Y 下向き。
光源の位置はキャンバス座標 (ノードの座標 + `get_canvas_origin()`) で、ノードの変形は陰影に影響しない。ノードの `modulate` は色に掛かる。
`UgoeModel.from_buffer(bytes)` / `UgoeModule.from_buffer(bytes)` でバイト列からリソースを作れる。

`.ugoe` と `.ugoemod` はリソースとして書き出しに含まれる (エクスポートの設定は要らない)。

## 描画のしかた

- 評価 (`ugoe::Puppet::update`) の結果を、描画可能要素ごとのキャンバスアイテム (`RenderingServer`) と `ShaderMaterial` で描く。
  描画順は draw index で決め、頂点位置は変わった要素だけ送り直す。
- テクスチャは読み込み時に乗算済みアルファに変換する (法線マップは変換しない)。式・合成モードの表し方は `src/shader.rs` の冒頭を参照。
  ライティングは参照レンダラ (`crates/ugoe-render/src/shaders/drawable.wgsl`) と同じ式で、法線は画面での微分から求めたヤコビ行列で回す。
- マスクは、マスクとして参照されるメッシュのカバレッジを 1 枚のアトラス (`SubViewport`) に並べて描き、要素はキャンバス座標から読む
  (ノードの変形に左右されない)。どのカバレッジがアトラスのどこにあるか・要素がどのマスクを読むかは浮動小数点のテクスチャの表
  (`src/model.rs` の `MaskTable`) に書き、シェーダはループで読む (読めるマスクの数に上限はない)。
  カバレッジの画素の格子はキャンバスの `1 / 解像度` の格子に揃える (倍率 1 なら画面の画素と重なる)。
- 入れ子のマスク (マスク用メッシュ自身がマスクを読む) は段に分け、段ごとのアトラスのビューポートを前の段のビューポートの親にする。
  Godot は子のビューポートを先に描くので、同じフレームのうちに段の順に描かれる。
- アトラスの 1 辺は 4096 画素まで。収まらなければマスクの解像度を下げる (`mask_scale` か画面上の大きさが変わるまで戻さない)。
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
- 光源を Godot の `Light2D` から取る仕組みはない (`lights` に自分で渡す)。
- 当たり判定・パラメータなどの Godot の API の自動テストは、ヘッドレスの Godot で動く `example/tests/smoke.gd` だけ。
- Godot 4.7.1 で、GDExtension のクラスを持つプロジェクトの初回の `--import` の終わりに異常終了することがある
  (godot-rust の最小のクラスでも起き、2 回目からは起きない)。`build.mjs` は 1 回だけやり直す。

## 関連タスク

- T0406 (`docs/tasks/T0406-godot-extension.md`)、T0411 (`docs/tasks/T0411-godot-followups.md`)
