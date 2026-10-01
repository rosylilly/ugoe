extends SceneTree
## 適合テスト (tests/conformance/README.md) の Godot の実行器。計画 (plan.json) のケースごとに `UgoePuppet2D` で描き、
## `<出力先>/<ケース>.png` に書く。比べ合わせは `ugoe-conformance compare --binding godot --images-only` が行う。
## 描画が要るので、ヘッドレスではなく画面 (Xvfb でもよい) のある環境で実行する:
##
##   godot --path bindings/godot/example --rendering-driver opengl3 --script res://tests/conformance.gd -- \
##       <plan.json> <出力先のディレクトリ>
##
## ふつうは `node tests/conformance/run.mjs --only godot` (か `just godot-check`) から実行する。
##
## - 計画のとおり、読み込む前にパラメータ・段の有効無効・モジュールを設定する (ノードは読み込みの直後に 1 度評価する)。
##   `steps` が空ならすべての段を止め、あれば目パチと呼吸を `effects` のとおりにして、`playing = false` のまま
##   `advance(dt)` を `steps` の順に呼ぶ。
## - `before` があれば、まずその値で数フレーム描き、次のフレームで `parameters` に変えて、その直後のフレームを書く
##   (アニメーション中にマスクが 1 フレーム遅れないかを確かめる)。
## - `.ugoe` バンドルでないケース (モデルの JSON) と、画像を比べないケースは扱わず、`skipped.json` に理由を書く。
## - キャンバスと同じ大きさ (倍率 1) のビューポートに、不透明な背景 (plan.background) を敷いて描く。

const STAGES := ["physics", "eye_blink", "breath", "lip_sync", "pose"]
const WARMUP_FRAMES := 4

var _plan: Dictionary
var _out := ""
var _index := -1
var _skipped := {}
var _failed := 0

var _case: Dictionary
var _viewport: SubViewport
var _puppet: Node2D
var _frames := 0
var _capture_at := WARMUP_FRAMES


func _vector3(values: Array) -> Vector3:
	return Vector3(values[0], values[1], values[2])


func _color(values: Array) -> Color:
	return Color(values[0], values[1], values[2])


func _set_params(params: Dictionary) -> void:
	for id in params:
		_puppet.set_parameter(id, params[id])


func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() < 2:
		printerr("usage: -- <plan.json> <output directory>")
		quit(2)
		return
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	if not parsed is Dictionary:
		printerr("cannot read %s (use an absolute path)" % args[0])
		quit(2)
		return
	_plan = parsed
	_out = args[1]
	DirAccess.make_dir_recursive_absolute(_out)
	_next_case()


## 次のケースを用意する。無ければ skipped.json を書いて終える。
func _next_case() -> void:
	if _viewport != null:
		_viewport.queue_free()
		_viewport = null
		_puppet = null
	var cases: Array = _plan["cases"]
	while true:
		_index += 1
		if _index >= cases.size():
			var file := FileAccess.open(_out.path_join("skipped.json"), FileAccess.WRITE)
			file.store_string(JSON.stringify(_skipped, "  "))
			file.close()
			print("godot: drew %d cases (%d failed, %d skipped)" % [
				cases.size() - _skipped.size() - _failed, _failed, _skipped.size()
			])
			quit(0 if _failed == 0 else 1)
			return
		_case = cases[_index]
		if not _case["bundle"]:
			_skipped[_case["name"]] = "the Godot binding reads only .ugoe bundles"
		elif not _case["image"]:
			_skipped[_case["name"]] = "the case has no image"
		elif _setup():
			return
		else:
			_failed += 1


## `_case` の `UgoePuppet2D` とビューポートを作る。失敗したら false。
func _setup() -> bool:
	var name: String = _case["name"]
	_puppet = ClassDB.instantiate("UgoePuppet2D")
	_puppet.playing = false
	var steps: Array = _case["steps"]
	for stage in STAGES:
		var enabled := not steps.is_empty()
		if stage == "eye_blink" or stage == "breath":
			enabled = enabled and _case["effects"]
		_puppet.set_stage_enabled(stage, enabled)
	var before: Dictionary = _case.get("before", {})
	_set_params(before if not before.is_empty() else _case["parameters"])
	# モジュールは読み込む前にまとめて設定する (読み込むときに付ける)。
	var modules := []
	for path in _case["modules"]:
		modules.append(UgoeModule.from_buffer(FileAccess.get_file_as_bytes(path)))
	_puppet.modules = modules
	if not _puppet.load_model_from_buffer(FileAccess.get_file_as_bytes(_case["model"])):
		printerr("%s: cannot load %s" % [name, _case["model"]])
		_puppet.free()
		return false
	if _puppet.get_module_names().size() != modules.size():
		printerr("%s: modules are not attached" % name)
		_puppet.free()
		return false
	var lighting = _case.get("lighting")
	if lighting is Dictionary:
		_puppet.ambient_light = _color(lighting.get("ambient", [1, 1, 1]))
		for light in lighting.get("lights", []):
			var color := _color(light.get("color", [1, 1, 1]))
			if light["type"] == "directional":
				_puppet.add_directional_light(_vector3(light["direction"]), color)
			else:
				_puppet.add_point_light(_vector3(light["position"]), color, light.get("range", INF))
	_frames = 0
	if before.is_empty():
		_advance()
		_capture_at = WARMUP_FRAMES
	else:
		_capture_at = WARMUP_FRAMES + 1
	var size: Vector2 = _puppet.get_canvas_size().round()
	_puppet.position = _puppet.get_canvas_origin()

	_viewport = SubViewport.new()
	_viewport.size = Vector2i(size)
	_viewport.disable_3d = true
	_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	var background := ColorRect.new()
	var rgb: Array = _plan["background"]
	background.color = Color8(rgb[0], rgb[1], rgb[2])
	background.size = size
	_viewport.add_child(background)
	_viewport.add_child(_puppet)
	root.add_child(_viewport)
	return true


## 計画の `steps` のとおりに時間を進める (空なら時間を進めずに 1 度評価する)。
func _advance() -> void:
	var steps: Array = _case["steps"]
	if steps.is_empty():
		_puppet.advance(0.0)
	for dt in steps:
		_puppet.advance(dt)


func _process(_delta: float) -> bool:
	if _viewport == null:
		return true
	_frames += 1
	if _frames == WARMUP_FRAMES and _capture_at > WARMUP_FRAMES:
		# このフレームで値を変え、次のフレームで (このフレームの描画の結果を) 書く。
		_set_params(_case["parameters"])
		_advance()
		return false
	if _frames < _capture_at:
		return false
	var image := _viewport.get_texture().get_image()
	image.convert(Image.FORMAT_RGBA8)
	image.save_png(_out.path_join("%s.png" % _case["name"]))
	_next_case()
	return false
