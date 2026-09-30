extends SceneTree
## `ugoe render` の画像と、この拡張で描いた画像を比べる (docs/spec/rendering.md#互換の基準)。
## 描画が要るので、ヘッドレスではなく画面 (Xvfb でもよい) のある環境で実行する:
##
##   godot --path bindings/godot/example --rendering-driver opengl3 --script res://tests/conformance.gd -- \
##       <case.json> <reference.png> [out.png]
##
## case.json は `node bindings/godot/build.mjs --conformance` が書く:
##   { "model": ".ugoe のパス", "params": { ID: 値 }, "modules": [".ugoemod のパス"],
##     "ambient": [r, g, b], "lights": [{ "type": "directional", "direction": [x, y, z], "color": [r, g, b] }
##     または { "type": "point", "position": [x, y, z], "color": [r, g, b], "range": 数 }],
##     "before": { ID: 値 } }
## "before" があれば、まずその値で数フレーム描き、次のフレームで "params" に変えて、その直後のフレームを比べる
## (アニメーション中にマスクが 1 フレーム遅れないかを確かめる)。
##
## 参照の画像は同じ条件の `ugoe render ... --background "#808080"` で作る (キャンバスの大きさ・倍率 1・時間を進めない)。
## 背景を不透明にして比べる。すべてのチャンネルの差が 3/255 以下の画素が 99% 以上なら合格 (終了コード 0)。

const BACKGROUND := Color8(128, 128, 128)
const WARMUP_FRAMES := 4

var _viewport: SubViewport
var _puppet: Node2D
var _case: Dictionary
var _reference: Image
var _out_path := ""
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
		printerr("usage: -- <case.json> <reference.png> [out.png]")
		quit(2)
		return
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	_reference = Image.load_from_file(args[1])
	if not parsed is Dictionary or _reference == null:
		printerr("cannot read %s or %s (use absolute paths)" % [args[0], args[1]])
		quit(2)
		return
	_case = parsed
	if args.size() > 2:
		_out_path = args[2]

	_puppet = ClassDB.instantiate("UgoePuppet2D")
	_puppet.playing = false
	# モジュールは `ugoe render --module` と同じく、読み込む前にまとめて設定する (読み込むときに付ける)。
	var modules := []
	for path in _case.get("modules", []):
		modules.append(UgoeModule.from_buffer(FileAccess.get_file_as_bytes(path)))
	_puppet.modules = modules
	if not _puppet.load_model_from_buffer(FileAccess.get_file_as_bytes(_case["model"])):
		quit(2)
		return
	if _puppet.get_module_names().size() != modules.size():
		printerr("modules are not attached")
		quit(2)
		return
	if _case.has("ambient"):
		_puppet.ambient_light = _color(_case["ambient"])
	for light in _case.get("lights", []):
		if light["type"] == "directional":
			_puppet.add_directional_light(_vector3(light["direction"]), _color(light["color"]))
		else:
			_puppet.add_point_light(
				_vector3(light["position"]), _color(light["color"]), light.get("range", INF)
			)
	if _case.has("before"):
		_set_params(_case["before"])
		_capture_at = WARMUP_FRAMES + 1
	else:
		_set_params(_case.get("params", {}))
	var size: Vector2 = _puppet.get_canvas_size()
	_puppet.position = _puppet.get_canvas_origin()

	_viewport = SubViewport.new()
	_viewport.size = Vector2i(size)
	_viewport.disable_3d = true
	_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	var background := ColorRect.new()
	background.color = BACKGROUND
	background.size = size
	_viewport.add_child(background)
	_viewport.add_child(_puppet)
	root.add_child(_viewport)


func _process(_delta: float) -> bool:
	if _viewport == null:
		return true
	_frames += 1
	if _frames == WARMUP_FRAMES and _case.has("before"):
		# このフレームで値を変え、次のフレームで (このフレームの描画の結果を) 比べる。
		_set_params(_case.get("params", {}))
		_puppet.advance(0.0)
		return false
	if _frames < _capture_at:
		return false
	var image := _viewport.get_texture().get_image()
	image.convert(Image.FORMAT_RGBA8)
	if _out_path != "":
		image.save_png(_out_path)
	_reference.convert(Image.FORMAT_RGBA8)
	if image.get_size() != _reference.get_size():
		printerr("size differs: %s vs %s" % [image.get_size(), _reference.get_size()])
		quit(1)
		return true
	var total := image.get_width() * image.get_height()
	var bad := 0
	var worst := 0
	var a := image.get_data()
	var b := _reference.get_data()
	for i in total:
		var diff := 0
		for c in 4:
			diff = maxi(diff, absi(a[i * 4 + c] - b[i * 4 + c]))
		worst = maxi(worst, diff)
		if diff > 3:
			bad += 1
	var ratio := 1.0 - float(bad) / float(total)
	print("matching pixels: %.3f%% (worst difference %d/255)" % [ratio * 100.0, worst])
	quit(0 if ratio >= 0.99 else 1)
	return true
