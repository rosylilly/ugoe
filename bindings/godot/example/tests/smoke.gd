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


## パラメータの値と、すべての描画可能要素の頂点位置が有限か。
func _finite(puppet: Node2D) -> bool:
	for id in puppet.get_parameter_ids():
		if not is_finite(puppet.get_parameter(id)):
			printerr("parameter %s is %f" % [id, puppet.get_parameter(id)])
			return false
	for i in puppet.get_drawable_ids().size():
		for p: Vector2 in puppet.get_drawable_positions(i):
			if not p.is_finite():
				printerr("drawable %d has %s" % [i, p])
				return false
	return true


## リポジトリの中のファイルのパス (example/ から 3 つ上がリポジトリのルート)。
func _repo(path: String) -> String:
	return ProjectSettings.globalize_path("res://").path_join("../../..").path_join(path)


## SDK (ugoe::Puppet) と C ABI にある機能 (T0449)。
func _sdk_features(puppet: Node2D) -> void:
	# 読み込みの警告: 見本には無く、未来の版を模したファイルでは知らない任意の機能
	_check(puppet.load_model("res://models/avatar.ugoe"), "load avatar for the SDK features")
	_check(puppet.get_load_warnings().is_empty(), "avatar has no load warnings")
	var future := FileAccess.get_file_as_bytes(_repo("assets/compat/future/optional-feature.ugoe"))
	_check(puppet.load_model_from_buffer(future), "load a file from a future version")
	var codes := []
	for w: Dictionary in puppet.get_load_warnings():
		codes.append(w["code"])
		if w["code"] == "unknown_optional_feature":
			_check(w.has("feature") and w["warning"] is String, "a warning has its fields (%s)" % w)
	_check(codes.has("unknown_optional_feature"), "load warnings %s" % [codes])

	# 着せ替えのモジュールの読み込みの警告 (T0456): 未来の版を模したモジュールでは知らない任意の機能
	# (未来の版のモジュールは assets/compat/dev/long-hair.ugoemod を元にしたもので、dress-up-base.ugoe に付く)
	var base := FileAccess.get_file_as_bytes(_repo("assets/compat/dev/dress-up-base.ugoe"))
	_check(puppet.load_model_from_buffer(base), "load dress-up-base for the module warnings")
	var dev_module := UgoeModule.from_buffer(FileAccess.get_file_as_bytes(_repo("assets/compat/dev/long-hair.ugoemod")))
	_check(puppet.attach_module(dev_module), "attach the dev long-hair")
	_check(puppet.get_module_load_warnings(puppet.get_module_names()[0]).is_empty(), "the dev long-hair has no load warnings")
	var future_module := UgoeModule.from_buffer(FileAccess.get_file_as_bytes(_repo("assets/compat/future/optional-feature.ugoemod")))
	_check(puppet.attach_module(future_module), "attach a module from a future version")
	var module_names: PackedStringArray = puppet.get_module_names()
	_check(module_names.size() == 1, "the future module replaces the dev long-hair %s" % module_names)
	var module_codes := []
	for w: Dictionary in puppet.get_module_load_warnings(module_names[0]):
		module_codes.append(w["code"])
		if w["code"] == "unknown_optional_feature":
			_check(w.has("feature") and w["warning"] is String, "a module warning has its fields (%s)" % w)
	_check(module_codes.has("unknown_optional_feature"), "module load warnings %s" % [module_codes])
	# attach_module は新しいモジュールを 1 回だけ読み、読み込みの警告を 1 回だけ出す (T0459)
	if ClassDB.class_exists("Logger"):
		puppet.clear_modules()
		var counter: Logger = load("res://tests/warning_counter.gd").new()
		counter.needle = "ugoe: module"
		OS.add_logger(counter)
		_check(puppet.attach_module(UgoeModule.from_buffer(FileAccess.get_file_as_bytes(_repo("assets/compat/future/optional-feature.ugoemod")))), "attach the future module again")
		OS.remove_logger(counter)
		var expected: int = puppet.get_module_load_warnings(module_names[0]).size()
		_check(expected > 0 and counter.count == expected, "attach_module warns once per load warning (%d warnings, %d logged)" % [expected, counter.count])
	else:
		print("skip: Logger needs Godot 4.5 or later")
	_check(puppet.get_module_load_warnings("missing").is_empty(), "a missing module has no warnings")
	puppet.clear_modules()
	_check(puppet.get_module_load_warnings(module_names[0]).is_empty(), "a detached module has no warnings")

	# トラッキングの対象とスロット
	_check(puppet.load_model("res://models/avatar.ugoe"), "reload avatar")
	_check(puppet.get_tracking_targets().has("AngleX"), "tracking targets %s" % puppet.get_tracking_targets())
	_check(puppet.get_slot_ids().is_empty(), "avatar has no slots")

	# 受け手: 指揮役の値を基準値にして、物理より後の段だけを進める
	var receiver: Node2D = ClassDB.instantiate("UgoePuppet2D")
	receiver.playing = false
	root.add_child(receiver)
	receiver.model = load("res://models/avatar.ugoe")
	puppet.set_parameter("AngleX", 20.0)
	puppet.advance(1.0 / 60.0)
	var values: PackedFloat32Array = puppet.get_parameter_values()
	var angle_x: int = puppet.get_parameter_ids().find("AngleX")
	_check(values.size() == puppet.get_parameter_ids().size(), "get_parameter_values (%d)" % values.size())
	_check(receiver.follow(values, 1.0 / 60.0, -1.0, 0.0, []), "follow")
	_check(is_equal_approx(receiver.get_parameter("AngleX"), values[angle_x]), "follow writes the base values (%f)" % receiver.get_parameter("AngleX"))
	_check(receiver.follow(values, 1.0 / 60.0, 0.5, 1.0, [{"part": 0, "value": 0.5, "weight": 1.0}]), "follow with lip sync and part opacities")
	_check(receiver.set_base_values(values), "set_base_values")
	# 数が違う・NaN・負か有限でない時間・有限でない音量・形の違うパーツの不透明度は拒む (C ABI と同じ)
	_check(not receiver.set_base_values(PackedFloat32Array([1.0])), "set_base_values rejects a wrong count")
	var broken := values.duplicate()
	broken[angle_x] = NAN
	_check(not receiver.set_base_values(broken), "set_base_values rejects NAN")
	_check(not receiver.follow(broken, 1.0 / 60.0, -1.0, 0.0, []), "follow rejects NAN values")
	_check(not receiver.follow(values, NAN, -1.0, 0.0, []), "follow rejects a NAN delta")
	_check(not receiver.follow(values, -1.0, -1.0, 0.0, []), "follow rejects a negative delta")
	_check(not receiver.follow(values, 1.0 / 60.0, INF, 1.0, []), "follow rejects an infinite lip sync level")
	_check(not receiver.follow(values, 1.0 / 60.0, NAN, 1.0, []), "follow rejects a NAN lip sync level")
	_check(not receiver.follow(values, 1.0 / 60.0, 0.5, INF, []), "follow rejects an infinite lip sync strength")
	_check(not receiver.follow(values, 1.0 / 60.0, -1.0, 0.0, [{"part": -1, "value": 1.0, "weight": 1.0}]), "follow rejects a negative part")
	_check(not receiver.follow(values, 1.0 / 60.0, -1.0, 0.0, [{"part": 0, "value": INF, "weight": 1.0}]), "follow rejects an infinite part opacity")
	_check(not receiver.follow(values, 1.0 / 60.0, -1.0, 0.0, [1]), "follow rejects a malformed part opacity")
	_check(_finite(receiver), "rejected input keeps the receiver finite")
	# 指揮役の値 (T0454): 物理の段の直前の値・当てたリップシンク・パーツの不透明度のカーブを、そのまま follow に渡せる
	puppet.set_lip_sync_level(0.5)
	puppet.advance(1.0 / 60.0)
	var before: PackedFloat32Array = puppet.get_values_before_physics()
	_check(before.size() == values.size(), "get_values_before_physics (%d)" % before.size())
	var lip_sync: Vector2 = puppet.get_lip_sync_applied()
	_check(is_equal_approx(lip_sync.x, 0.5), "get_lip_sync_applied %s" % lip_sync)
	var applied: Array = puppet.get_part_opacities_applied()
	_check(receiver.follow(before, 1.0 / 60.0, lip_sync.x, lip_sync.y, applied), "follow the conductor values")
	_check(receiver.get_values_before_physics() == before, "the receiver keeps the values it followed")
	_check(receiver.get_lip_sync_applied() == lip_sync, "the receiver keeps the lip sync it followed")
	puppet.set_stage_enabled("lip_sync", false)
	puppet.advance(1.0 / 60.0)
	_check(puppet.get_lip_sync_applied() == Vector2(-1.0, 1.0), "no lip sync is Vector2(-1, 1)")
	puppet.set_stage_enabled("lip_sync", true)
	receiver.queue_free()

	# トラッキング中のモーションの優先 (モデルを読み直しても保つ)
	puppet.motion_over_tracking = true
	puppet.tracking_kept = PackedStringArray(["EyeLOpen"])
	_check(puppet.load_model("res://models/avatar.ugoe"), "reload with motion_over_tracking")
	_check(puppet.motion_over_tracking and puppet.tracking_kept == PackedStringArray(["EyeLOpen"]), "motion over tracking is kept across reloads")
	for over in [false, true]:
		puppet.motion_over_tracking = over
		_check(puppet.load_model("res://models/avatar.ugoe"), "reload for motion over tracking %s" % over)
		puppet.set_tracking_head(Vector3(-20.0, 0.0, 0.0))
		puppet.start_motion("shake", 2)
		for i in 24:
			puppet.advance(1.0 / 60.0)
		var x: float = puppet.get_parameter("AngleX")
		_check((x > 0.0) == over, "motion_over_tracking %s gives AngleX %f" % [over, x])
		puppet.clear_tracking()
	puppet.motion_over_tracking = false
	puppet.tracking_kept = PackedStringArray()

	# 表情を止める
	_check(puppet.load_model("res://models/avatar.ugoe"), "reload for stop_expressions")
	_check(puppet.set_expression("smile"), "set smile")
	for i in 60:
		puppet.advance(1.0 / 60.0)
	_check(puppet.get_parameter("EyeLSmile") > 0.9, "smile is shown (%f)" % puppet.get_parameter("EyeLSmile"))
	puppet.stop_expressions()
	for i in 120:
		puppet.advance(1.0 / 60.0)
	_check(puppet.get_parameter("EyeLSmile") < 0.1, "stop_expressions (%f)" % puppet.get_parameter("EyeLSmile"))

	# 標準の当たり判定 (dress-up は Head と Body を持つ)
	_check(puppet.load_model("res://models/dress-up.ugoe"), "load dress-up for standard hit areas")
	_check(not puppet.get_slot_ids().is_empty(), "dress-up has slots %s" % puppet.get_slot_ids())
	var head: String = puppet.get_standard_hit_area("Head")
	_check(head != "", "standard hit area Head (%s)" % head)
	_check(puppet.get_standard_hit_area("Tail") == "", "an unknown standard name has no hit area")
	var size: Vector2 = puppet.get_canvas_size()
	var origin: Vector2 = puppet.get_canvas_origin()
	var inside := Vector2.INF
	for gy in 32:
		for gx in 32:
			var p := Vector2(gx + 0.5, gy + 0.5) * size / 32.0 - origin
			if puppet.hit_test(head, p):
				inside = p
				break
		if inside != Vector2.INF:
			break
	_check(inside != Vector2.INF, "a point inside the head")
	if inside != Vector2.INF:
		_check(puppet.hit_test_standard("Head", inside), "hit_test_standard")
		_check(puppet.get_standard_hit_areas_at(inside).has("Head"), "get_standard_hit_areas_at %s" % puppet.get_standard_hit_areas_at(inside))
	_check(not puppet.hit_test_standard("Tail", Vector2.ZERO), "hit_test_standard with an unknown name")

	# パーツとポーズ (公開仕様の例 pose-select)
	_check(puppet.load_model_from_json(FileAccess.get_file_as_string(_repo("spec/examples/pose-select/model.json"))), "load pose-select")
	_check(puppet.get_part_ids() == PackedStringArray(["Root", "ArmA", "ArmB", "SleeveB", "Hat"]), "part ids %s" % puppet.get_part_ids())
	puppet.advance(0.0)
	_check(is_equal_approx(puppet.get_part_evaluated_opacity("ArmA"), 1.0), "the first pose entry is shown")
	_check(puppet.get_part_opacity("ArmB")["weight"] > 0.0, "pose overrides ArmB (%s)" % puppet.get_part_opacity("ArmB"))
	_check(puppet.get_part_opacity("missing").is_empty(), "an unknown part has no opacity")
	_check(puppet.show_pose("ArmB"), "show_pose")
	_check(not puppet.show_pose("Hat"), "show_pose for a part outside the pose")
	for i in 60:
		puppet.advance(1.0 / 60.0)
	_check(is_equal_approx(puppet.get_part_evaluated_opacity("ArmB"), 1.0), "ArmB fades in (%f)" % puppet.get_part_evaluated_opacity("ArmB"))
	_check(is_zero_approx(puppet.get_part_evaluated_opacity("ArmA")), "ArmA fades out (%f)" % puppet.get_part_evaluated_opacity("ArmA"))

	# 目パチ・呼吸・リップシンクの設定の差し替え (公開仕様の例 effects-replace)
	_check(puppet.load_model_from_json(FileAccess.get_file_as_string(_repo("spec/examples/effects-replace/model.json"))), "load effects-replace")
	puppet.set_lip_sync_level(0.6)
	puppet.advance(0.1)
	_check(is_equal_approx(puppet.get_parameter("Mouth"), 0.6), "lip sync moves Mouth")
	_check(puppet.set_effects_json('{"lip_sync": {"parameters": ["Mouth2"]}}'), "set_effects_json")
	puppet.advance(0.1)
	_check(is_equal_approx(puppet.get_parameter("Mouth2"), 0.6), "replaced lip sync moves Mouth2 (%f)" % puppet.get_parameter("Mouth2"))
	_check(not puppet.set_effects_json("{"), "broken effects JSON fails")
	# 口の入力をやめる: 負の値 (C ABI・wasm・Unity と同じ) と clear_lip_sync_level (T0459)
	_check(is_equal_approx(puppet.get_lip_sync_applied().x, 0.6), "lip sync is applied %s" % puppet.get_lip_sync_applied())
	puppet.set_lip_sync_level(-1.0)
	puppet.advance(0.1)
	_check(puppet.get_lip_sync_applied() == Vector2(-1.0, 1.0), "a negative level stops the lip sync input %s" % puppet.get_lip_sync_applied())
	puppet.set_lip_sync_level(0.0)
	puppet.advance(0.1)
	_check(puppet.get_lip_sync_applied() == Vector2(0.0, 1.0), "level 0 is still an input %s" % puppet.get_lip_sync_applied())
	puppet.set_lip_sync_level(-INF)
	puppet.advance(0.1)
	_check(puppet.get_lip_sync_applied() == Vector2(-1.0, 1.0), "-INF stops the lip sync input %s" % puppet.get_lip_sync_applied())
	puppet.set_lip_sync_level(0.6)
	puppet.advance(0.1)
	puppet.clear_lip_sync_level()
	puppet.advance(0.1)
	_check(puppet.get_lip_sync_applied() == Vector2(-1.0, 1.0), "clear_lip_sync_level stops the lip sync input %s" % puppet.get_lip_sync_applied())

	# 読み込みの上限 (T0459): 超えたら読み込みに失敗し、get_load_error() で種類と補足の値を読める
	_check(puppet.max_image_side == 16384 and puppet.max_chunk_decompressed == 256 * 1024 * 1024 and puppet.max_total_decompressed == 1024 * 1024 * 1024, "default load limits")
	puppet.max_image_side = 8
	_check(not puppet.load_model("res://models/avatar.ugoe"), "a texture over max_image_side fails")
	var error: Dictionary = puppet.get_load_error()
	_check(error.get("code") == "image_limit_exceeded" and error.get("image") == "texture" and error.get("max") == 8 and error.get("width") > 8 and error.get("texture") is int and error.get("error") is String, "image limit error %s" % error)
	_check(not puppet.is_model_loaded(), "no model after a failed load")
	puppet.max_image_side = -1
	puppet.max_image_side = 1 << 32
	_check(puppet.max_image_side == 8, "max_image_side rejects values outside 0..2^32 (%d)" % puppet.max_image_side)
	puppet.max_image_side = 16384
	var zstd := FileAccess.get_file_as_bytes(_repo("spec/examples/bundles/single-quad-zstd.ugoe"))
	_check(puppet.load_model_from_buffer(zstd), "load a compressed bundle")
	_check(puppet.get_load_error().is_empty(), "a successful load clears the error")
	puppet.max_total_decompressed = 1
	_check(not puppet.load_model_from_buffer(zstd), "a bundle over max_total_decompressed fails")
	error = puppet.get_load_error()
	_check(error.get("code") == "read_limit_exceeded" and error.get("limit") == "total_decompressed" and error.get("max") == 1 and error.get("chunk") is String, "read limit error %s" % error)
	puppet.max_total_decompressed = -1
	_check(puppet.max_total_decompressed == 1, "max_total_decompressed rejects negative values")
	puppet.max_total_decompressed = 1024 * 1024 * 1024
	puppet.max_chunk_decompressed = 1
	_check(not puppet.load_model_from_buffer(zstd), "a bundle over max_chunk_decompressed fails")
	_check(puppet.get_load_error().get("limit") == "chunk_decompressed", "chunk limit error %s" % puppet.get_load_error())
	puppet.max_chunk_decompressed = 256 * 1024 * 1024
	_check(not puppet.load_model_from_buffer(PackedByteArray([1, 2, 3])), "broken bytes fail")
	_check(puppet.get_load_error().get("code") == "invalid_file", "broken bytes are invalid_file %s" % puppet.get_load_error())
	# 着せ替えのモジュールにも効く
	_check(puppet.load_model("res://models/dress-up.ugoe"), "load dress-up for module limits")
	puppet.max_image_side = 8
	_check(not puppet.attach_module(load("res://models/sailor.ugoemod")), "a module texture over max_image_side fails")
	error = puppet.get_load_error()
	_check(error.get("code") == "image_limit_exceeded" and error.get("max") == 8, "module image limit error %s" % error)
	_check(puppet.get_module_names().is_empty(), "the module is not attached")
	puppet.max_image_side = 16384
	_check(puppet.attach_module(load("res://models/sailor.ugoemod")), "attach sailor within the limits")
	_check(puppet.get_load_error().is_empty(), "a successful attach clears the error")
	puppet.clear_modules()
	_check(puppet.load_model_from_json(FileAccess.get_file_as_string(_repo("spec/examples/effects-replace/model.json"))), "reload effects-replace")
	puppet.clear_lip_sync_level()

	# 物理の設定の差し替え (公開仕様の例 physics-pendulum)
	var physics := FileAccess.get_file_as_string(_repo("spec/examples/physics-pendulum/model.physics.json"))
	_check(puppet.load_model_from_json(FileAccess.get_file_as_string(_repo("spec/examples/physics-pendulum/model.json"))), "load physics-pendulum")
	_check(puppet.get_physics_json() == "", "a model JSON has no physics")
	_check(puppet.set_physics_json(physics), "set_physics_json")
	_check(puppet.get_physics_json().contains("\"Hair\""), "get_physics_json")
	_check(not puppet.set_physics_json("{"), "broken physics JSON fails")
	_check(puppet.get_physics_json() != "", "a failed set keeps the physics")
	_check(puppet.set_physics_json(""), "an empty physics JSON removes the physics")
	_check(puppet.get_physics_json() == "", "physics removed")
	_check(puppet.set_base_physics_json(physics), "set_base_physics_json")
	_check(puppet.get_base_physics_json().contains("\"Hair\""), "get_base_physics_json")
	for i in 30:
		puppet.advance(1.0 / 60.0)
	_check(_finite(puppet), "physics keeps values finite")
	_check(puppet.load_model_from_json(FileAccess.get_file_as_string(_repo("spec/examples/single-quad/model.json"))), "load single-quad")
	_check(not puppet.set_physics_json(physics), "physics that does not fit the model fails")
	_check(not puppet.set_base_physics_json(physics), "base physics that does not fit the model fails")


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

	# 段の有効無効 (適合テストの時間を進めない評価に使う)。モデルを読み直しても保つ
	_check(puppet.is_stage_enabled("breath"), "stages are enabled by default")
	_check(puppet.set_stage_enabled("breath", false), "set_stage_enabled")
	_check(not puppet.set_stage_enabled("dance", false), "unknown stage fails")
	_check(puppet.load_model("res://models/avatar.ugoe"), "reload keeps the stages")
	_check(not puppet.is_stage_enabled("breath"), "breath stays disabled")
	puppet.set_parameter("Breath", 0.7)
	puppet.advance(0.5)
	_check(is_equal_approx(puppet.get_parameter("Breath"), 0.7), "disabled breath does not move Breath")
	puppet.set_stage_enabled("breath", true)

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

	# XRFaceTracker (Unified Expressions) から ARKit のブレンドシェイプへ
	_check(puppet.load_model("res://models/avatar.ugoe"), "load avatar for XR tracking")
	puppet.reset_parameters()
	puppet.set_stage_enabled("eye_blink", false)   # 目パチが EyeLOpen / EyeROpen に掛からないように
	var face := XRFaceTracker.new()
	face.name = "/user/ugoe_smoke_face"
	face.set_blend_shape(XRFaceTracker.FT_JAW_OPEN, 1.0)
	face.set_blend_shape(XRFaceTracker.FT_EYE_CLOSED_LEFT, 1.0)
	face.set_blend_shape(XRFaceTracker.FT_MOUTH_CORNER_PULL_LEFT, 1.0)
	face.set_blend_shape(XRFaceTracker.FT_MOUTH_CORNER_PULL_RIGHT, 1.0)
	_check(puppet.set_tracking_from_xr_face_tracker(face), "set_tracking_from_xr_face_tracker")
	_check(not puppet.set_tracking_from_xr_face_tracker(XRPositionalTracker.new()), "other trackers are rejected")
	puppet.advance(1.0 / 60.0)
	_check(puppet.get_parameter("MouthOpenY") > 0.9, "FT_JAW_OPEN reaches MouthOpenY (%f)" % puppet.get_parameter("MouthOpenY"))
	_check(puppet.get_parameter("EyeLOpen") < 0.1, "FT_EYE_CLOSED_LEFT closes EyeLOpen (%f)" % puppet.get_parameter("EyeLOpen"))
	_check(puppet.get_parameter("EyeROpen") > 0.9, "the right eye stays open (%f)" % puppet.get_parameter("EyeROpen"))
	_check(puppet.get_parameter("MouthForm") > 0.9, "corner pull reaches MouthForm (%f)" % puppet.get_parameter("MouthForm"))
	# xr_face_tracker の名前で XRServer から毎フレーム読む
	XRServer.add_tracker(face)
	puppet.xr_face_tracker = face.name
	face.set_blend_shape(XRFaceTracker.FT_JAW_OPEN, 0.0)
	face.set_blend_shape(XRFaceTracker.FT_EYE_CLOSED_LEFT, 0.0)
	await process_frame
	await process_frame
	_check(puppet.get_parameter("MouthOpenY") < 0.1, "xr_face_tracker is read every frame (%f)" % puppet.get_parameter("MouthOpenY"))
	_check(puppet.get_parameter("EyeLOpen") > 0.9, "xr_face_tracker opens the eye (%f)" % puppet.get_parameter("EyeLOpen"))
	puppet.xr_face_tracker = &""
	XRServer.remove_tracker(face)
	puppet.clear_tracking()
	puppet.set_stage_enabled("eye_blink", true)

	# シーンの Light2D から光源を取る (ノードの位置と大きさを考えてキャンバス座標に直す)
	_check(puppet.load_model("res://models/avatar-lit.ugoe"), "load avatar-lit for scene lights")
	var origin: Vector2 = puppet.get_canvas_origin()
	puppet.position = Vector2(100, 50)
	puppet.scale = Vector2(2, 2)
	var point := PointLight2D.new()
	point.position = Vector2(300, 250)
	point.height = 40.0
	point.energy = 2.0
	point.color = Color(1.0, 0.5, 0.25)
	var gradient := GradientTexture2D.new()
	gradient.width = 128
	gradient.height = 64
	point.texture = gradient
	point.texture_scale = 1.5
	root.add_child(point)
	_check(puppet.get_scene_lights().is_empty(), "scene lights are off by default")
	puppet.use_scene_lights = true
	var found: Array = puppet.get_scene_lights()
	_check(found.size() == 1, "one scene light (%d)" % found.size())
	if found.size() == 1:
		var light: Dictionary = found[0]
		var expected := Vector3(100 + origin.x, 100 + origin.y, 20)
		_check(light["type"] == "point", "PointLight2D is a point light")
		_check(light["position"].is_equal_approx(expected), "point light position %s (%s)" % [light["position"], expected])
		_check(is_equal_approx(light["range"], 48.0), "range is the texture radius in canvas units (%f)" % light["range"])
		_check(light["color"].is_equal_approx(Color(2.0, 1.0, 0.5)), "color × energy (%s)" % light["color"])
	# 後から足したライトも見つける (木の node_added)
	var sun := DirectionalLight2D.new()
	sun.rotation_degrees = -90.0   # +Y が右を向く
	sun.height = 0.0
	root.add_child(sun)
	found = puppet.get_scene_lights()
	_check(found.size() == 2, "a light added later is found (%d)" % found.size())
	if found.size() == 2:
		var direction: Vector3 = found[1]["direction"]
		_check(direction.is_equal_approx(Vector3(1, 0, 0)), "DirectionalLight2D travels along its +Y (%s)" % direction)
	# 外したライト・無効なライト・cull mask が重ならないライトは使わない
	sun.enabled = false
	_check(puppet.get_scene_lights().size() == 1, "disabled lights are ignored")
	point.range_item_cull_mask = 2
	_check(puppet.get_scene_lights().is_empty(), "lights whose cull mask misses light_mask are ignored")
	point.range_item_cull_mask = 1
	root.remove_child(sun)
	sun.free()
	_check(puppet.get_scene_lights().size() == 1, "a removed light is dropped")
	puppet.advance(1.0 / 60.0)
	puppet.use_scene_lights = false
	_check(puppet.get_scene_lights().is_empty(), "use_scene_lights = false")
	point.queue_free()
	puppet.position = Vector2.ZERO
	puppet.scale = Vector2.ONE

	# マスクの解像度の倍率 (下げて戻す判断は src/model.rs の MaskShrink のテスト。見本のモデルはマスクが少なく、
	# 1 枚ずつは MaskArea::fit が上限に収めるので、ここでは下がらない)
	_check(puppet.load_model("res://models/avatar.ugoe"), "load avatar for mask resolution")
	puppet.advance(1.0 / 60.0)
	_check(puppet.get_mask_resolution_factor() == 1.0, "masks fit at mask_scale 1")
	puppet.mask_scale = 40.0
	puppet.advance(1.0 / 60.0)
	_check(puppet.get_mask_resolution_factor() <= 1.0, "a large mask_scale still draws")
	puppet.mask_scale = 1.0
	puppet.advance(1.0 / 60.0)
	_check(puppet.get_mask_resolution_factor() == 1.0, "mask resolution factor after mask_scale 1")

	# 有限でない値 (GDScript の NAN・INF) と負の時間は受け付けず、パラメータと頂点を有限に保つ
	_check(puppet.load_model("res://models/avatar.ugoe"), "load avatar for non-finite input")
	puppet.set_parameter("AngleX", NAN)
	puppet.set("parameters/AngleY", NAN)
	puppet.set_lip_sync_level(NAN)
	puppet.set_tracking_head(Vector3(NAN, INF, 0.0))
	puppet.set_tracking_body({"yaw": INF, "roll": NAN})
	_check(puppet.set_tracking_blend_shapes({"jawOpen": NAN, "eyeBlinkLeft": INF}) == 0, "non-finite blend shapes are ignored")
	_check(not puppet.set_tracking_blend_shape("jawOpen", INF), "a non-finite blend shape is rejected")
	puppet.advance(1.0 / 60.0)
	puppet.advance(NAN)
	puppet.advance(INF)
	puppet.advance(-1.0)
	_check(_finite(puppet), "non-finite input keeps parameters and vertices finite")
	puppet.clear_tracking()
	puppet.clear_lip_sync_level()
	var speed: float = puppet.speed
	puppet.speed = INF
	puppet.speed = -1.0
	_check(puppet.speed == speed, "speed rejects non-finite and negative values (%f)" % puppet.speed)
	var mask_scale: float = puppet.mask_scale
	puppet.mask_scale = INF
	_check(puppet.mask_scale == mask_scale, "mask_scale rejects INF (%f)" % puppet.mask_scale)
	var ambient: Color = puppet.ambient_light
	puppet.ambient_light = Color(NAN, 0.0, 0.0)
	_check(puppet.ambient_light == ambient, "ambient_light rejects NAN")
	puppet.lights = [{"type": "point", "position": Vector3.ZERO, "color": Color(INF, 1, 1)}]
	puppet.advance(1.0 / 60.0)
	puppet.clear_lights()

	# 木の外で設定したモデルは、木に戻ったときに読む (`_ready` は最初の 1 回だけなので、それに頼らない)
	var outside: Node2D = ClassDB.instantiate("UgoePuppet2D")
	root.add_child(outside)
	root.remove_child(outside)
	outside.model = load("res://models/avatar.ugoe")
	root.add_child(outside)
	_check(outside.is_model_loaded(), "a model set outside the tree is loaded on re-entering")
	root.remove_child(outside)
	outside.model = null
	root.add_child(outside)
	_check(not outside.is_model_loaded(), "a model cleared outside the tree is unloaded on re-entering")
	outside.queue_free()

	# 読み込みのときに付けられなかったモジュールは、付いているものとして返さない
	_check(puppet.load_model("res://models/dress-up.ugoe"), "load dress-up before a model that does not fit")
	puppet.modules = [hair]
	_check(puppet.get_module_names().size() == 1, "the module is attached to dress-up")
	_check(puppet.load_model("res://models/avatar.ugoe"), "load avatar with a module that does not fit")
	_check(puppet.modules.size() == 1, "the module stays in the modules property")
	_check(puppet.get_module_names().is_empty(), "modules that failed to attach are not reported (%s)" % puppet.get_module_names())
	puppet.modules = []

	_check(not puppet.load_model("res://models/missing.ugoe"), "missing file fails")
	_check(not puppet.is_model_loaded(), "unloaded after failure")
	_check(puppet.load_model_from_buffer(FileAccess.get_file_as_bytes("res://models/avatar.ugoe")), "load_model_from_buffer")
	_check(not puppet.load_model_from_buffer(PackedByteArray([1, 2, 3])), "broken bytes fail")
	# モデルの JSON の読み込みと評価の結果の読み戻し (適合テストで使う)
	puppet.clear_modules()
	var json_path := ProjectSettings.globalize_path("res://").path_join("../../../spec/examples/single-quad/model.json")
	_check(puppet.load_model_from_json(FileAccess.get_file_as_string(json_path)), "load_model_from_json")
	_check(puppet.get_drawable_ids() == PackedStringArray(["Quad"]), "drawable ids")
	_check(puppet.get_drawable_positions(0)[0] == Vector2(64, 64), "drawable positions are canvas coordinates")
	_check(puppet.get_drawable_state(0)["visible"] and puppet.get_drawable_state(0)["opacity"] == 1.0, "drawable state")
	_check(puppet.get_drawable_positions(5).is_empty() and puppet.get_drawable_state(-1).is_empty(), "out of range drawables")
	_check(puppet.get_render_order() == PackedInt32Array([0]), "render order")
	_check(not puppet.load_model_from_json("{"), "broken JSON fails")
	_check(not puppet.attach_module(UgoeModule.from_buffer(PackedByteArray([1, 2, 3]))), "broken module fails")

	_sdk_features(puppet)

	puppet.queue_free()
	print("smoke: %s" % ("FAILED" if _failures > 0 else "ok"))
	quit(1 if _failures > 0 else 0)
