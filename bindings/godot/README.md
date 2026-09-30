# ugoe-godot (Godot 4 の GDExtension)

`.ugoe` のモデルを Godot 4 (4.4 以降) の 2D のノードとして表示し、動かすための GDExtension。
[godot-rust (gdext)](https://github.com/godot-rust/gdext) で `ugoe::Puppet` を直接使う。

**ライセンス:** このクレートは MIT。依存する godot-rust のクレートは MPL-2.0 (ファイル単位のコピーレフト)。
ルートのワークスペースの外に置き、依存のライセンスは `check-licenses.mjs` で別に検査する ([ADR 0026](../../docs/adr/0026-godot-binding.md))。

## ビルドと見本

リポジトリのルートで:

```sh
node bindings/godot/build.mjs              # ビルドして example/bin/ にライブラリ、example/models/ に見本のモデルを置く
node bindings/godot/build.mjs --release    # リリースビルド
godot --path bindings/godot/example        # 見本のシーンを開く (エディタなら `godot -e --path ...`)
```

- ビルドはルートの `target/` を使う (`ugoe` のクレートのビルド済みの成果物を使い回す)。`CARGO_TARGET_DIR` で変えられる。
- 見本 (`example/main.tscn`) は `assets/samples/avatar.ugoe` を表示し、パラメータごとのスライダーと、表情・モーションのボタンを作る。
  触っていない間は `AngleX` を揺らす。クリックした位置の当たり判定を出力に出す。
- ほかのプロジェクトで使うには、`example/ugoe.gdextension` と `example/bin/` のライブラリをそのプロジェクトに写す。

## 検査

```sh
node bindings/godot/build.mjs --check        # 依存のライセンス + ヘッドレスの Godot で example/tests/smoke.gd
node bindings/godot/build.mjs --conformance  # `ugoe render` の画像と Godot で描いた画像を比べる (画面が要る。無ければ xvfb-run)
cargo test --manifest-path bindings/godot/Cargo.toml   # Godot なしで確かめられる部分 (src/model.rs・src/shader.rs)
cargo clippy --manifest-path bindings/godot/Cargo.toml --all-targets -- -D warnings
```

- Godot の実行ファイルは環境変数 `GODOT` (既定は `godot`) で指定する。
- `--conformance` は `assets/samples/avatar.ugoe` と、`examples/conformance_bundles.rs` が `ugoe_model::fixtures` から作るモデル
  (入れ子・反転・共通部分のマスク、3 つの合成モード、乗算色とスクリーン色、裏面カリング、半透明のテクスチャ) を、
  倍率 1・背景 `#808080` で描いて比べる。基準は [docs/spec/rendering.md#互換の基準](../../docs/spec/rendering.md#互換の基準)。
- これらは `just ci` に含まれない (ルートのワークスペースの外にあるため)。

## 使い方 (GDScript)

```gdscript
var puppet := UgoePuppet2D.new()
puppet.model_path = "res://models/avatar.ugoe"   # 木に入ると読み込む
add_child(puppet)

puppet.set_parameter("AngleX", 15.0)              # インスペクタの parameters/AngleX でも設定できる
puppet.start_motion("idle", 0)                    # 0 = 待機、1 = 普通、2 = 強制
puppet.set_expression("smile")
puppet.set_lip_sync_level(0.8)
puppet.motion_event.connect(func(value): print(value))
```

| メンバー | 内容 |
| --- | --- |
| `model_path` | 読み込む `.ugoe`。書き出すときは、エクスポートの「リソース以外のファイル」に `*.ugoe` を足す |
| `playing` / `speed` | 時間を進めるか / 速さ。止めている間もパラメータの変更は描き直す。`advance(delta)` で手で進められる |
| `play_in_editor` | エディタの中でも時間を進めるか (既定は止める) |
| `mask_scale` | マスクのテクスチャの解像度の倍率。1 で画面上の大きさと同じ |
| `parameters/<ID>` | インスペクタに出るパラメータ (シーンに保存される) |
| `load_model(path)` / `load_model_from_buffer(bytes)` | 読み込む |
| `get_parameter_ids()` / `get_parameter_info(id)` / `get_parameter(id)` / `set_parameter(id, value)` / `reset_parameters()` | パラメータ |
| `get_motion_names()` / `start_motion(name, priority)` / `stop_motions()` / `is_motion_playing()` | モーション |
| `get_expression_names()` / `set_expression(name)` | 表情 |
| `set_lip_sync_level(level)` / `clear_lip_sync_level()` / `reset_physics()` | リップシンク・物理 |
| `hit_test(id, local_position)` / `get_hit_areas_at(local_position)` | 当たり判定 (ノードの座標で渡す) |
| `get_canvas_size()` / `get_canvas_origin()` | キャンバスの大きさと原点 |
| シグナル `model_loaded` / `motion_event(value)` | 読み込み / モーションのイベント |

ノードの原点にキャンバスの原点 (`canvas.origin`) が来る。座標はキャンバスのピクセル単位で、Godot の 2D と同じく Y 下向き。
ノードの `modulate` は色に掛かる。

## 描画のしかた

- 評価 (`ugoe::Puppet::update`) の結果を、描画可能要素ごとのキャンバスアイテム (`RenderingServer`) と `ShaderMaterial` で描く。
  描画順は draw index で決め、頂点位置は変わった要素だけ送り直す。
- テクスチャは読み込み時に乗算済みアルファに変換する。式・合成モードの表し方は `src/shader.rs` の冒頭を参照。
- マスクは、マスクとして参照されるメッシュごとに `SubViewport` を作り、そのメッシュのカバレッジ (入れ子のマスクを含む) を描いて読む。
  テクスチャはマスク用メッシュの外接矩形だけを覆い、キャンバス座標から読むので、ノードの変形に左右されない。
- 裏面カリングは CPU で三角形を選ぶ (向きはキャンバス座標だけで決まるため)。

## 制限 (後続の候補)

- 1 つの要素が読めるマスクは 8 つまで (超えた分は警告を出して無視する)。仕様では上限がない。
- ライティング (法線マップ)・着せ替え (モジュール)・トラッキングは、まだノードに出していない。
- 入れ子のマスクは、依存するビューポートを先に作ることで、同じフレームのうちに描かれる順になるのを期待している (Godot の `SubViewport` の描く順に頼っている)。
- 当たり判定・パラメータなどの Godot の API の自動テストは、ヘッドレスの Godot で動く `example/tests/smoke.gd` だけ。
- Godot 4.7.1 で、GDExtension のクラスを持つプロジェクトの初回の `--import` の終わりに異常終了することがある
  (godot-rust の最小のクラスでも起き、2 回目からは起きない)。`build.mjs` は 1 回だけやり直す。

## 関連タスク

- T0406 (`docs/tasks/T0406-godot-extension.md`)
