extends SceneTree
## ヘッドレスの Godot で拡張を確かめる (`node bindings/godot/build.mjs --check`)。
## 失敗したら終了コード 1 で終わる。描画の結果 (画素) はヘッドレスでは確かめられない
## (`build.mjs --conformance` で確かめる)。

var _failures := 0
var _started := false


func _check(ok: bool, what: String) -> void:
	if ok:
		print("ok: ", what)
	else:
		_failures += 1
		printerr("FAILED: ", what)


# 木の中で試すため、最初のフレームで始める (`_initialize` の間は root がまだ木に入っていない)。
func _process(_delta: float) -> bool:
	if not _started:
		_started = true
		_run()
	return false


func _run() -> void:
	var puppet: Node2D = ClassDB.instantiate("UgoePuppet2D")
	_check(puppet != null, "UgoePuppet2D is registered")
	if puppet == null:
		quit(1)
		return
	root.add_child(puppet)

	# リソースとしての読み込み (ResourceFormatLoader)
	var model: Resource = load("res://models/avatar.ugoe")
	_check(model is UgoeModel, "load() gives a UgoeModel")
	_check(ResourceLoader.get_recognized_extensions_for_type("Resource").has("ugoemod"), "ugoemod is recognized")
	puppet.model = model
	_check(puppet.is_model_loaded(), "setting model loads it")
	_check(puppet.load_model("res://models/avatar.ugoe"), "load_model")
	_check(puppet.is_model_loaded(), "is_model_loaded")
	var ids: PackedStringArray = puppet.get_parameter_ids()
	_check(ids.size() == 27, "27 parameters (%d)" % ids.size())
	_check(puppet.get_canvas_size() == Vector2(1024, 1024), "canvas size")

	var info: Dictionary = puppet.get_parameter_info("AngleX")
	_check(info.has("min") and info.has("max"), "parameter info")
	puppet.set_parameter("AngleX", 1000.0)
	puppet.advance(1.0 / 60.0)
	_check(is_equal_approx(puppet.get_parameter("AngleX"), info["max"]), "set_parameter clamps")
	# インスペクタのプロパティ (parameters/<ID>) からも設定できる
	puppet.set("parameters/AngleX", -5.0)
	puppet.advance(1.0 / 60.0)
	_check(is_equal_approx(puppet.get("parameters/AngleX"), -5.0), "parameters/AngleX property")
	_check(is_equal_approx(puppet.get_parameter("AngleX"), -5.0), "property reaches the puppet")
	var names := []
	for p in puppet.get_property_list():
		names.append(p["name"])
	_check(names.has("parameters/AngleX"), "property list has parameters/AngleX")

	# シーンに保存すると、設定したパラメータだけが残る (既定値のものは保存しない)。モデルはパスで参照する。
	var packed := PackedScene.new()
	packed.pack(puppet)
	ResourceSaver.save(packed, "user://smoke.tscn")
	var text := FileAccess.get_file_as_string("user://smoke.tscn")
	_check(text.contains("parameters/AngleX = -5"), "parameter is saved in the scene")
	_check(not text.contains("parameters/AngleY"), "default parameters are not saved")
	_check(text.contains("res://models/avatar.ugoe"), "model is saved as a path")
	var restored: Node2D = packed.instantiate()
	root.add_child(restored)
	_check(restored.is_model_loaded(), "saved model is restored")
	_check(is_equal_approx(restored.get_parameter("AngleX"), -5.0), "saved parameter is restored")
	restored.queue_free()

	for i in 120:
		puppet.advance(1.0 / 60.0)
	_check(puppet.get_children(true).size() > 0, "mask atlas viewports are created")

	# トラッキング: 頭の回転とブレンドシェイプがパラメータに届く
	puppet.reset_parameters()
	puppet.set_tracking_head(Vector3(20.0, 0.0, 0.0))
	var count: int = puppet.set_tracking_blend_shapes({"jawOpen": 1.0, "_neutral": 0.5})
	_check(count == 1, "unknown blend shapes are ignored (%d)" % count)
	_check(puppet.is_tracking(), "is_tracking")
	puppet.advance(1.0 / 60.0)
	_check(puppet.get_parameter("AngleX") > 10.0, "head yaw reaches AngleX (%f)" % puppet.get_parameter("AngleX"))
	_check(puppet.get_parameter("MouthOpenY") > 0.5, "jawOpen reaches MouthOpenY (%f)" % puppet.get_parameter("MouthOpenY"))
	_check(puppet.get_tracking_blend_shape_names().size() == 52, "52 blend shape names")
	puppet.clear_tracking()
	_check(not puppet.is_tracking(), "clear_tracking")

	# ライティング: avatar は受けない、avatar-lit は受ける。光源は辞書で持つ
	_check(not puppet.uses_lighting(), "avatar does not use lighting")
	puppet.add_directional_light(Vector3(1, 1, -1), Color.WHITE)
	puppet.add_point_light(Vector3(500, 300, 200), Color(1, 0.5, 0.5), 800.0)
	_check(puppet.lights.size() == 2, "two lights")
	_check(puppet.lights[1]["type"] == "point", "light dictionary")
	_check(puppet.load_model("res://models/avatar-lit.ugoe"), "load avatar-lit")
	_check(puppet.uses_lighting(), "avatar-lit uses lighting")
	puppet.ambient_light = Color(0.3, 0.3, 0.3)
	puppet.advance(1.0 / 60.0)
	puppet.clear_lights()
	_check(puppet.lights.is_empty(), "clear_lights")

	# 着せ替え: モジュールの付け外し
	_check(puppet.load_model("res://models/dress-up.ugoe"), "load dress-up")
	var hair: Resource = load("res://models/long-hair.ugoemod")
	_check(hair is UgoeModule, "load() gives a UgoeModule")
	_check(puppet.attach_module(hair), "attach_module")
	_check(puppet.attach_module(load("res://models/sailor.ugoemod")), "attach a second module")
	var modules: PackedStringArray = puppet.get_module_names()
	_check(modules == PackedStringArray(["LongHair", "Sailor"]), "module names %s" % modules)
	_check(puppet.modules.size() == 2, "modules property follows attach")
	puppet.advance(1.0 / 60.0)
	# 同じスロットのモジュールは差し替える (同じリソースをもう一度付けても増えない)
	_check(puppet.attach_module(hair), "attach the same module again")
	_check(puppet.get_module_names().size() == 2, "same module is replaced")
	_check(puppet.detach_module("LongHair"), "detach_module")
	_check(not puppet.detach_module("missing"), "detaching a missing module fails")
	_check(puppet.get_module_names().size() == 1, "one module left")
	puppet.clear_modules()
	_check(puppet.get_module_names().is_empty(), "clear_modules")
	# モジュールを付けたまま読み直すと付け直す
	puppet.modules = [hair]
	_check(puppet.load_model("res://models/dress-up.ugoe"), "reload with modules")
	_check(puppet.get_module_names().size() == 1, "modules are attached after reload")
	# モジュールと光源はシーンに保存され、インスタンスにしたときに付け直す
	puppet.add_directional_light(Vector3(0, 0, -1), Color.WHITE)
	var dressed := PackedScene.new()
	dressed.pack(puppet)
	var dressed_text := ""
	ResourceSaver.save(dressed, "user://dressed.tscn")
	dressed_text = FileAccess.get_file_as_string("user://dressed.tscn")
	_check(dressed_text.contains("res://models/long-hair.ugoemod"), "modules are saved as paths")
	_check(dressed_text.contains("\"directional\""), "lights are saved")
	var copy: Node2D = dressed.instantiate()
	root.add_child(copy)
	_check(copy.get_module_names() == PackedStringArray(["LongHair"]), "saved modules are attached")
	_check(copy.lights.size() == 1, "saved lights are restored")
	copy.queue_free()
	puppet.clear_lights()

	_check(not puppet.load_model("res://models/missing.ugoe"), "missing file fails")
	_check(not puppet.is_model_loaded(), "unloaded after failure")
	_check(puppet.load_model_from_buffer(FileAccess.get_file_as_bytes("res://models/avatar.ugoe")), "load_model_from_buffer")
	_check(not puppet.load_model_from_buffer(PackedByteArray([1, 2, 3])), "broken bytes fail")
	_check(not puppet.attach_module(UgoeModule.from_buffer(PackedByteArray([1, 2, 3]))), "broken module fails")

	puppet.queue_free()
	print("smoke: %s" % ("FAILED" if _failures > 0 else "ok"))
	quit(1 if _failures > 0 else 0)
