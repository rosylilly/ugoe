extends Node2D
## ugoe の見本: モデルのパラメータごとにスライダーを作り、UgoePuppet2D.set_parameter で動かす。
## 触っていない間は、頭の向き (AngleX) をゆっくり揺らす。

@onready var puppet: UgoePuppet2D = $Puppet
@onready var sliders: VBoxContainer = $Panel/Scroll/Sliders

var _time := 0.0
var _touched := false


func _ready() -> void:
	if not puppet.is_model_loaded():
		push_error("model is not loaded; run `node bindings/godot/build.mjs` first")
		return
	_fit(Rect2(0, 0, 660, 720))
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
