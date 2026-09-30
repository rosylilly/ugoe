extends SceneTree
## ヘッドレスの Godot で拡張を確かめる (`node bindings/godot/build.mjs --check`)。
## 失敗したら終了コード 1 で終わる。描画の結果 (画素) はヘッドレスでは確かめられない。

var _failures := 0


func _check(ok: bool, what: String) -> void:
	if ok:
		print("ok: ", what)
	else:
		_failures += 1
		printerr("FAILED: ", what)


func _initialize() -> void:
	var puppet: Node2D = ClassDB.instantiate("UgoePuppet2D")
	_check(puppet != null, "UgoePuppet2D is registered")
	if puppet == null:
		quit(1)
		return
	root.add_child(puppet)
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

	# シーンに保存すると、設定したパラメータだけが残る (既定値のものは保存しない)
	var packed := PackedScene.new()
	packed.pack(puppet)
	ResourceSaver.save(packed, "user://smoke.tscn")
	var text := FileAccess.get_file_as_string("user://smoke.tscn")
	_check(text.contains("parameters/AngleX = -5"), "parameter is saved in the scene")
	_check(not text.contains("parameters/AngleY"), "default parameters are not saved")
	var restored: Node2D = packed.instantiate()
	root.add_child(restored)
	_check(is_equal_approx(restored.get_parameter("AngleX"), -5.0), "saved parameter is restored")
	restored.queue_free()

	for i in 120:
		puppet.advance(1.0 / 60.0)
	_check(puppet.get_children(true).size() > 0, "mask viewports are created")
	_check(not puppet.load_model("res://models/missing.ugoe"), "missing file fails")
	_check(not puppet.is_model_loaded(), "unloaded after failure")
	_check(puppet.load_model_from_buffer(FileAccess.get_file_as_bytes("res://models/avatar.ugoe")), "load_model_from_buffer")

	puppet.queue_free()
	print("smoke: %s" % ("FAILED" if _failures > 0 else "ok"))
	quit(1 if _failures > 0 else 0)
