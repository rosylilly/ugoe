extends Node2D
## ugoe の見本: モデルを選び、パラメータごとのスライダーで UgoePuppet2D.set_parameter を呼ぶ。
## 着せ替えのモジュール (models/*.ugoemod) を付け外しでき、ライティングを受けるモデルではマウスの位置に点光源を置く。
## 触っていない間は、頭の向き (AngleX) をゆっくり揺らす。
##
## 起動の引数 (`godot --path ... -- --model=dress-up.ugoe --modules=sailor.ugoemod,cat-ears.ugoemod`) で、
## 最初のモデルと付けるモジュールを選べる (スクリーンショット用)。

const MODELS_DIR := "res://models/"

@onready var puppet: UgoePuppet2D = $Puppet
@onready var sliders: VBoxContainer = $Panel/Scroll/Sliders

var _time := 0.0
var _touched := false
var _light := Vector2(-1, -1)


func _ready() -> void:
	var args := {}
	for arg in OS.get_cmdline_user_args():
		var kv := arg.trim_prefix("--").split("=", true, 1)
		if kv.size() == 2:
			args[kv[0]] = kv[1]
	if args.has("model"):
		puppet.load_model(MODELS_DIR + args["model"])
	if args.has("modules"):
		for name in args["modules"].split(","):
			puppet.attach_module(load(MODELS_DIR + name))
	if not puppet.is_model_loaded():
		push_error("model is not loaded; run `node bindings/godot/build.mjs` first")
		return
	_build_panel()


func _files(extension: String) -> PackedStringArray:
	var out := PackedStringArray()
	for file in DirAccess.get_files_at(MODELS_DIR):
		if file.get_extension() == extension:
			out.append(file)
	return out


func _build_panel() -> void:
	for child in sliders.get_children():
		child.queue_free()
	_fit(Rect2(0, 0, 660, 720))

	var models := OptionButton.new()
	var current := puppet.model.resource_path.get_file() if puppet.model else ""
	for file in _files("ugoe"):
		models.add_item(file)
		if file == current:
			models.select(models.item_count - 1)
	models.item_selected.connect(func(i: int) -> void:
		puppet.clear_modules()
		puppet.load_model(MODELS_DIR + models.get_item_text(i))
		_build_panel.call_deferred())
	sliders.add_child(models)

	# 着せ替え: 付けられるモジュールだけチェックボックスを出す (本体に合わなければ付け外しで失敗する)
	for file in _files("ugoemod"):
		var module: UgoeModule = load(MODELS_DIR + file)
		var check := CheckBox.new()
		check.text = "module: %s" % file.get_basename()
		check.button_pressed = puppet.modules.has(module)
		check.toggled.connect(func(on: bool) -> void:
			var names_before := puppet.get_module_names()
			if on:
				puppet.attach_module(module)
			else:
				var list: Array = puppet.modules.duplicate()
				list.erase(module)
				puppet.modules = list
			if puppet.get_module_names() == names_before:
				check.set_pressed_no_signal(puppet.modules.has(module)))
		sliders.add_child(check)
	if puppet.uses_lighting():
		var label := Label.new()
		label.text = "lighting: the point light follows the mouse"
		sliders.add_child(label)
		puppet.ambient_light = Color(0.45, 0.45, 0.5)

	for id in puppet.get_parameter_ids():
		var info := puppet.get_parameter_info(id)
		var label := Label.new()
		label.text = "%s (%s)" % [info["name"], id]
		sliders.add_child(label)
		var slider := HSlider.new()
		slider.min_value = info["min"]
		slider.max_value = info["max"]
		slider.step = 0.01
		slider.value = puppet.get_parameter(id)
		slider.value_changed.connect(func(value: float) -> void:
			_touched = true
			puppet.set_parameter(id, value))
		sliders.add_child(slider)
	for name in puppet.get_expression_names():
		var button := Button.new()
		button.text = "expression: %s" % name
		button.pressed.connect(func() -> void: puppet.set_expression(name))
		sliders.add_child(button)
	for name in puppet.get_motion_names():
		var button := Button.new()
		button.text = "motion: %s" % name
		button.pressed.connect(func() -> void: puppet.start_motion(name, 1))
		sliders.add_child(button)


## キャンバス全体が `area` に収まるように置く (ノードの原点 = キャンバスの原点)。
func _fit(area: Rect2) -> void:
	var size := puppet.get_canvas_size()
	var s := minf(area.size.x / size.x, area.size.y / size.y) * 0.95
	puppet.scale = Vector2(s, s)
	puppet.position = area.get_center() + (puppet.get_canvas_origin() - size / 2.0) * s


func _process(delta: float) -> void:
	if puppet.uses_lighting():
		# 点光源をマウスの位置 (キャンバス座標) の手前に置く
		var p := puppet.to_local(get_global_mouse_position()) + puppet.get_canvas_origin()
		if p != _light:
			_light = p
			var size := puppet.get_canvas_size()
			puppet.lights = [{
				"type": "point",
				"position": Vector3(p.x, p.y, size.x * 0.3),
				"color": Color(1.0, 0.95, 0.85),
				"range": size.x * 1.5,
			}]
	if _touched or not puppet.has_parameter("AngleX"):
		return
	_time += delta
	puppet.set_parameter("AngleX", sin(_time * 1.5) * 20.0)


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed:
		var local := puppet.to_local(get_global_mouse_position())
		var areas := puppet.get_hit_areas_at(local)
		if not areas.is_empty():
			print("hit: ", areas)
