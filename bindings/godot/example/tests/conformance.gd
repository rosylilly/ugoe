extends SceneTree
## `ugoe render` の画像と、この拡張で描いた画像を比べる (docs/spec/rendering.md#互換の基準)。
## 描画が要るので、ヘッドレスではなく画面 (Xvfb でもよい) のある環境で実行する:
##
##   godot --path bindings/godot/example --rendering-driver opengl3 --script res://tests/conformance.gd -- \
##       <model.ugoe> <reference.png> [out.png]
##
## 参照の画像は `ugoe render <model.ugoe> -o <reference.png> --background "#808080"` で作る
## (キャンバスの大きさ・倍率 1・時間を進めない)。背景を不透明にして比べる。
## すべてのチャンネルの差が 3/255 以下の画素が 99% 以上なら合格 (終了コード 0)。

const BACKGROUND := Color8(128, 128, 128)
const WARMUP_FRAMES := 4

var _viewport: SubViewport
var _reference: Image
var _out_path := ""
var _frames := 0


func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() < 2:
		printerr("usage: -- <model.ugoe> <reference.png> [out.png]")
		quit(2)
		return
	_reference = Image.load_from_file(args[1])
	if args.size() > 2:
		_out_path = args[2]

	var puppet: Node2D = ClassDB.instantiate("UgoePuppet2D")
	puppet.playing = false
	if not puppet.load_model_from_buffer(FileAccess.get_file_as_bytes(args[0])):
		quit(2)
		return
	var size: Vector2 = puppet.get_canvas_size()
	puppet.position = puppet.get_canvas_origin()

	_viewport = SubViewport.new()
	_viewport.size = Vector2i(size)
	_viewport.disable_3d = true
	_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	var background := ColorRect.new()
	background.color = BACKGROUND
	background.size = size
	_viewport.add_child(background)
	_viewport.add_child(puppet)
	root.add_child(_viewport)


func _process(_delta: float) -> bool:
	_frames += 1
	if _frames < WARMUP_FRAMES:
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
