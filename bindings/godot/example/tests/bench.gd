extends SceneTree
## 1 フレームの時間を測る (T0450)。見本のモデルのパラメータを毎フレーム動かし、`UgoePuppet2D` の `_process`
## (シーンの光源を読む・評価する・描画の準備) にかかった時間の平均と 5・50・95 パーセンタイルを出す。
##
##   node bindings/godot/build.mjs --release
##   xvfb-run -a godot --path bindings/godot/example --rendering-driver opengl3 --script res://tests/bench.gd
##
## 描画の資源 (RenderingServer) に渡す配列のコピーも測りたいので、ヘッドレス (描画しない) ではなく画面のある環境で実行する。

const FRAMES := 2000
const WARMUP := 100

var _cases := [
	{"name": "avatar", "model": "res://models/avatar.ugoe", "scene_lights": false},
	{"name": "avatar-lit", "model": "res://models/avatar-lit.ugoe", "scene_lights": false},
	{"name": "avatar-lit + scene light", "model": "res://models/avatar-lit.ugoe", "scene_lights": true},
	{"name": "dress-up", "model": "res://models/dress-up.ugoe", "scene_lights": false},
]
var _case := -1
var _frame := 0
var _times: Array[int] = []
var _puppet: Node2D
var _light: PointLight2D


func _initialize() -> void:
	# 光源を読むところ (`_process`) と評価・描画の準備 (`playing` の `_process`) を、前後のノードで挟んで測る。
	var before := Hook.new()
	before.process_priority = -1000
	before.callback = _before
	var after := Hook.new()
	after.process_priority = 1000
	after.callback = _after
	root.add_child(before)
	root.add_child(after)
	_next()


## パラメータを、範囲の中で周期を変えて揺らす (どの要素の頂点も毎フレーム変わるように)。
func _wiggle(t: float) -> void:
	var ids: PackedStringArray = _puppet.get_parameter_ids()
	for i in ids.size():
		var info: Dictionary = _puppet.get_parameter_info(ids[i])
		var s := 0.5 + 0.5 * sin(t * (1.0 + 0.37 * i))
		_puppet.set_parameter(ids[i], lerpf(info["min"], info["max"], s))


func _next() -> void:
	if _puppet != null:
		_report()
		_puppet.queue_free()
		_puppet = null
	if _light != null:
		_light.queue_free()
		_light = null
	_case += 1
	if _case >= _cases.size():
		quit(0)
		return
	var c: Dictionary = _cases[_case]
	_puppet = ClassDB.instantiate("UgoePuppet2D")
	_puppet.playing = true
	_puppet.position = Vector2(640, 512)
	root.add_child(_puppet)
	if not _puppet.load_model(c["model"]):
		printerr("cannot load ", c["model"])
		quit(1)
		return
	if c["scene_lights"]:
		_light = PointLight2D.new()
		_light.position = Vector2(500, 300)
		root.add_child(_light)
		_puppet.use_scene_lights = true
	_frame = 0
	_times.clear()


## 決まった順番で `_process` を呼ぶノード (`UgoePuppet2D` の `_process` の前後で時刻を取るため)。
class Hook:
	extends Node
	var callback: Callable

	func _process(_delta: float) -> void:
		callback.call()


var _start := 0


func _before() -> void:
	if _puppet == null:
		return
	_wiggle(_frame / 60.0)
	if _light != null:
		_light.position = Vector2(500 + 100 * sin(_frame / 30.0), 300)
	_start = Time.get_ticks_usec()


func _after() -> void:
	if _puppet == null:
		return
	var elapsed := Time.get_ticks_usec() - _start
	if _frame >= WARMUP:
		_times.append(elapsed)
	_frame += 1
	if _frame >= WARMUP + FRAMES:
		_next()


func _report() -> void:
	var sorted := _times.duplicate()
	sorted.sort()
	var total := 0
	for t in sorted:
		total += t
	var mean := float(total) / sorted.size()
	var median: int = sorted[sorted.size() / 2]
	var p5: int = sorted[int(sorted.size() * 0.05)]
	var p95: int = sorted[int(sorted.size() * 0.95)]
	print(
		(
			"%-26s mean %7.1f us  p5 %5d us  median %5d us  p95 %5d us  (%d frames)"
			% [_cases[_case]["name"], mean, p5, median, p95, sorted.size()]
		)
	)
