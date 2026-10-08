extends SceneTree
## 適合テスト (tests/conformance/README.md) の Godot の実行器。計画 (plan.json) のケースごとに `UgoePuppet2D` で評価して描き、
## 評価の結果を `<出力先>/<ケース>.json` (形は tests/conformance/src/frame.rs) に、画像を `<ケース>.png` に書く。
## 比べ合わせは `ugoe-conformance compare --binding godot --images` が行う。
## 描画が要るので、ヘッドレスではなく画面 (Xvfb でもよい) のある環境で実行する:
##
##   godot --path bindings/godot/example --rendering-driver opengl3 --script res://tests/conformance.gd -- \
##       <plan.json> <出力先のディレクトリ> [<描き方>]
##
## 描き方 (`gl_compatibility`・`forward_plus` など) を渡すと、Godot がその描き方で動いているかを確かめる (Forward+ の Vulkan を
## 作れないと Godot は黙って Compatibility に戻るため。T0467)。
##
## ふつうは `node tests/conformance/run.mjs --only godot` (か `just godot-check`) から実行する。
##
## - 計画のとおり、読み込む前にパラメータ・段の有効無効・モジュールを設定する (ノードは読み込みの直後に 1 度評価する)。
##   `steps` が空ならすべての段を止め、あれば目パチと呼吸を `effects` のとおりにして、`playing = false` のまま
##   `advance(dt)` を `steps` の順に呼ぶ。計画に `follow` があれば、`advance` の代わりに受け手の口 `follow` を呼ぶ (T0449)。
## - 計画に `conduct` があれば、同じモデルで指揮役のノードをもう 1 つ作って `advance` で進め、指揮役の口
##   (`get_values_before_physics`・`get_lip_sync_applied`・`get_part_opacities_applied`) で読んだ値で受け手を `follow` させる (T0454)。
## - 計画に `motions` があれば、その番号の `advance` の直前に `start_motion` で始め、各 `advance` の後に発火したイベントと出どころ
##   (`get_motion_events`) を結果の `motion_events` に書く (T0462)。
## - 計画に `steps` があれば、各 `advance` (か `follow`) の後の `is_changed()` を結果の `changed_after_load` に書く (T1405。ノードは
##   読み込みの直後に 1 度評価するので、`changed` ではなく、参照がそうして進めた列と比べる)。
## - `before` があれば、まずその値で数フレーム描き、次のフレームで `parameters` に変えて、その直後のフレームを書く
##   (アニメーション中にマスクが 1 フレーム遅れないかを確かめる)。
## - モデルは `.ugoe` バンドルなら `load_model_from_buffer`、違えば (モデルの JSON) `load_model_from_json` で読む。
## - 評価の結果は、画像を読み戻すのと同じフレームでノードから読み戻す (`get_drawable_*`・`get_render_order`)。
##   画像を比べないケースは画像を書かない。
## - キャンバスと同じ大きさ (倍率 1) のビューポートに、不透明な背景 (plan.background) を敷いて描く。

const STAGES := ["physics", "eye_blink", "breath", "lip_sync", "pose"]
const PRIORITIES := {"idle": 0, "normal": 1, "force": 2}
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
## 計画の `motions` のあるケースで発火したモーションのイベント (結果の `motion_events`)。
var _events := []
## 時間を進めるケースの、各 `advance` (か `follow`) の後の `is_changed()` (結果の `changed_after_load`。T1405)。
var _changed := []


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
	var method := RenderingServer.get_current_rendering_method()
	print("godot: rendering method %s, adapter %s" % [method, RenderingServer.get_video_adapter_name()])
	if args.size() >= 3 and method != args[2]:
		printerr("expected the rendering method %s, but Godot runs %s" % [args[2], method])
		quit(2)
		return
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
		if _setup():
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
	var loaded: bool
	if _case["bundle"]:
		loaded = _puppet.load_model_from_buffer(FileAccess.get_file_as_bytes(_case["model"]))
	else:
		loaded = _puppet.load_model_from_json(FileAccess.get_file_as_string(_case["model"]))
	if not loaded:
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


## 計画の `steps` のとおりに時間を進める (空なら時間を進めずに 1 度評価する)。`follow` があれば受け手として進める。
func _advance() -> void:
	var steps: Array = _case["steps"]
	if steps.is_empty():
		_puppet.advance(0.0)
	var conduct = _case.get("conduct")
	if conduct is Dictionary:
		_puppet.reset_physics()
		_conduct(conduct)
		return
	var follow: Array = _case.get("follow", [])
	var lip_sync: Array = _case.get("follow_lip_sync", [])
	var parts: Array = _case.get("follow_part_opacities", [])
	if not follow.is_empty():
		# ノードは読み込みの直後に 1 度評価して物理の粒子を置くので、置き直して最初の `follow` の値から始める
		# (参照と C ABI は作ったばかりの `Puppet` で `follow` を呼ぶ)。
		_puppet.reset_physics()
	var motions: Array = _case.get("motions", [])
	_events = []
	_changed = []
	for i in steps.size():
		if follow.is_empty():
			for m: Dictionary in motions:
				if int(m["step"]) != i:
					continue
				var started: bool = _puppet.start_motion(m["name"], PRIORITIES[m["priority"]])
				if started != m["accepted"]:
					printerr("%s: starting %s at step %d gave %s" % [_case["name"], m["name"], i, started])
			_puppet.advance(steps[i])
			_changed.append(_puppet.is_changed())
			for e: Dictionary in _puppet.get_motion_events():
				if not motions.is_empty():
					_events.append({"step": i, "value": e["value"], "time": e["time"], "motion": e["motion"], "number": e["number"]})
			continue
		var level := -1.0
		var strength := 0.0
		if i < lip_sync.size() and lip_sync[i] is Array:
			level = lip_sync[i][0]
			strength = lip_sync[i][1]
		var opacities := []
		if i < parts.size():
			for p: Array in parts[i]:
				opacities.append({"part": int(p[0]), "value": p[1], "weight": p[2]})
		if not _puppet.follow(PackedFloat32Array(follow[i]), steps[i], level, strength, opacities):
			printerr("%s: follow failed at frame %d" % [_case["name"], i])
		_changed.append(_puppet.is_changed())


## 計画の `conduct` のとおりに指揮役のノードを作って進め、指揮役の口で読んだ値で `_puppet` を `follow` させる (T0454)。
## 指揮役は描かないので木に入れない。
func _conduct(conduct: Dictionary) -> void:
	_changed = []
	var name: String = _case["name"]
	var conductor: Node2D = ClassDB.instantiate("UgoePuppet2D")
	conductor.playing = false
	for stage in STAGES:
		conductor.set_stage_enabled(stage, stage == "lip_sync" or stage == "pose")
	for id in _case["parameters"]:
		conductor.set_parameter(id, _case["parameters"][id])
	var modules := []
	for path in _case["modules"]:
		modules.append(UgoeModule.from_buffer(FileAccess.get_file_as_bytes(path)))
	conductor.modules = modules
	var loaded: bool
	if _case["bundle"]:
		loaded = conductor.load_model_from_buffer(FileAccess.get_file_as_bytes(_case["model"]))
	else:
		loaded = conductor.load_model_from_json(FileAccess.get_file_as_string(_case["model"]))
	if not loaded:
		printerr("%s: the conductor cannot load %s" % [name, _case["model"]])
		conductor.free()
		return
	var effects = conduct.get("effects")
	if effects is String:
		var result: Dictionary = conductor.set_effects_json(effects)
		if not result.get("ok", false):
			printerr("%s: the conductor rejects the effects: %s" % [name, result])
		conductor.set_stage_enabled("eye_blink", true)
		conductor.set_stage_enabled("breath", true)
	var motion = conduct.get("motion")
	if motion is String and not conductor.start_motion(motion, 1):
		printerr("%s: motion %s did not start" % [name, motion])
	var levels: Array = conduct.get("lip_sync", [])
	var steps: Array = _case["steps"]
	for i in steps.size():
		if i < levels.size():
			conductor.set_lip_sync_level(levels[i])
		conductor.advance(steps[i])
		var lip_sync: Vector2 = conductor.get_lip_sync_applied()
		if not _puppet.follow(conductor.get_values_before_physics(), steps[i], lip_sync.x, lip_sync.y,
				conductor.get_part_opacities_applied()):
			printerr("%s: follow failed at frame %d" % [name, i])
		_changed.append(_puppet.is_changed())
	conductor.free()


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
	_write_frame()
	if _case["image"]:
		var image := _viewport.get_texture().get_image()
		image.convert(Image.FORMAT_RGBA8)
		image.save_png(_out.path_join("%s.png" % _case["name"]))
	_next_case()
	return false


## 評価の結果をノードから読み戻して `<ケース>.json` に書く (形は tests/conformance/src/frame.rs)。
func _write_frame() -> void:
	var ids: PackedStringArray = _puppet.get_drawable_ids()
	var meshes := {}
	for i in ids.size():
		var positions := []
		for p: Vector2 in _puppet.get_drawable_positions(i):
			positions.append([p.x, p.y])
		var state: Dictionary = _puppet.get_drawable_state(i)
		var multiply: Color = state["multiply_color"]
		var screen: Color = state["screen_color"]
		meshes[ids[i]] = {
			"positions": positions,
			"opacity": state["opacity"],
			"draw_order": state["draw_order"],
			"multiply_color": {"r": multiply.r, "g": multiply.g, "b": multiply.b},
			"screen_color": {"r": screen.r, "g": screen.g, "b": screen.b},
			"mask_strength": state["mask_strength"],
			"visible": state["visible"],
		}
	var order := []
	for i in _puppet.get_render_order():
		order.append(ids[i])
	var frame := {"meshes": meshes, "render_order": order}
	if not _case.get("motions", []).is_empty():
		frame["motion_events"] = _events
	if not _case["steps"].is_empty():
		frame["changed_after_load"] = _changed
	var file := FileAccess.open(_out.path_join("%s.json" % _case["name"]), FileAccess.WRITE)
	# 位置の許容誤差は 0.01 px なので、浮動小数点を丸めずに書く。
	file.store_string(JSON.stringify(frame, "", false, true))
	file.close()
