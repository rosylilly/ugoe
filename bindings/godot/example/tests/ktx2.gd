extends SceneTree
## KTX2 のテクスチャを描いて確かめる (`node bindings/godot/build.mjs --conformance` が実行する)。
## 失敗したら終了コード 1 で終わる。描画が要るので、画面 (Xvfb でもよい) のある環境で実行する:
##
##   godot --path bindings/godot/example --rendering-driver opengl3 --script res://tests/ktx2.gd
##
## 見本のモデル (`res://models/avatar.ugoe`) のテクスチャ (PNG) を、同じ画素の KTX2 に差し替えたバンドルを作って描き、
## PNG のままのバンドルの画像と画素ごとに比べる。KTX2 の画素も乗算していないアルファなので
## (`docs/spec/rendering.md#前提`)、読み込みで乗算すれば PNG と同じ画像になる。
##
## - `rgba8-srgb` / `rgba8-unorm`: 圧縮していない KTX2 (`VK_FORMAT_R8G8B8A8_SRGB` / `_UNORM`)。比べる相手は元のバンドル。
##   転送関数が sRGB でも、値はリニアに変換せずにそのまま使う。
## - `bc3`: ブロック圧縮の KTX2 (`VK_FORMAT_BC3_UNORM_BLOCK`。Godot の `Image.compress` で作る)。
##   比べる相手は、圧縮した画像を展開して PNG にしたバンドル (圧縮で変わった画素を同じにするため)。

const MODEL := "res://models/avatar.ugoe"
const WARMUP_FRAMES := 4
const BACKGROUND := Color8(128, 128, 128)

const KTX2_IDENTIFIER := [0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A]
const VK_FORMAT_R8G8B8A8_UNORM := 37
const VK_FORMAT_R8G8B8A8_SRGB := 43
const VK_FORMAT_BC3_UNORM_BLOCK := 137
# Khronos Data Format の値 (KHR_DF_*)
const DF_MODEL_RGBSDA := 1
const DF_MODEL_BC3 := 130
const DF_PRIMARIES_BT709 := 1
const DF_TRANSFER_LINEAR := 1
const DF_TRANSFER_SRGB := 2
const DF_CHANNEL_COLOR := 0
const DF_CHANNEL_GREEN := 1
const DF_CHANNEL_BLUE := 2
const DF_CHANNEL_ALPHA := 15
const DF_QUALIFIER_LINEAR := 0x10

var _failures := 0
var _frames := 0
## [名前, 描くビューポート, 比べる相手のビューポート] の列。
var _pairs := []


func _check(ok: bool, what: String) -> void:
	if ok:
		print("ok: ", what)
	else:
		_failures += 1
		printerr("FAILED: ", what)


func _initialize() -> void:
	var original := FileAccess.get_file_as_bytes(MODEL)
	if original.is_empty():
		printerr("cannot read %s (run `node bindings/godot/build.mjs` first)" % MODEL)
		quit(2)
		return
	# GPU での圧縮 (betsy) を使うと、終了の処理で落ちることがある。
	ProjectSettings.set_setting("rendering/textures/vram_compression/compress_with_gpu", false)
	var chunks := _read_chunks(original)
	var rgba8_srgb := []
	var rgba8_unorm := []
	var bc3 := []
	var bc3_png := []
	for chunk in chunks:
		if chunk["tag"] != "TEXR":
			rgba8_srgb.append(chunk)
			rgba8_unorm.append(chunk)
			bc3.append(chunk)
			bc3_png.append(chunk)
			continue
		var image := Image.new()
		if image.load_png_from_buffer(chunk["payload"]) != OK:
			printerr("a TEXR of %s is not a PNG" % MODEL)
			quit(2)
			return
		image.convert(Image.FORMAT_RGBA8)
		rgba8_srgb.append(_texr(_ktx2_rgba8(image, true)))
		rgba8_unorm.append(_texr(_ktx2_rgba8(image, false)))
		image.compress(Image.COMPRESS_S3TC, Image.COMPRESS_SOURCE_GENERIC)
		if image.get_format() != Image.FORMAT_DXT5:
			printerr("Image.compress did not give DXT5 (%d)" % image.get_format())
			quit(2)
			return
		bc3.append(_texr(_ktx2_bc3(image)))
		image.decompress()
		image.convert(Image.FORMAT_RGBA8)
		bc3_png.append(_texr(image.save_png_to_buffer()))
	var expected := _view(original)
	_pairs = [
		["rgba8-srgb", _view(_write_chunks(rgba8_srgb)), expected],
		["rgba8-unorm", _view(_write_chunks(rgba8_unorm)), expected],
		["bc3", _view(_write_chunks(bc3)), _view(_write_chunks(bc3_png))],
	]


func _process(_delta: float) -> bool:
	if _pairs.is_empty():
		return false
	_frames += 1
	if _frames < WARMUP_FRAMES:
		return false
	for pair in _pairs:
		_compare(pair[0], pair[1], pair[2])
	print("ktx2: %d failed" % _failures)
	_pairs = []
	quit(0 if _failures == 0 else 1)
	return false


func _compare(what: String, view: SubViewport, expected: SubViewport) -> void:
	var a := view.get_texture().get_image()
	var b := expected.get_texture().get_image()
	a.convert(Image.FORMAT_RGBA8)
	b.convert(Image.FORMAT_RGBA8)
	if a.get_size() != b.get_size() or a.is_empty():
		_check(false, "%s: image sizes %s and %s" % [what, a.get_size(), b.get_size()])
		return
	var da := a.get_data()
	var db := b.get_data()
	var different := 0
	var worst := 0
	for i in range(0, da.size(), 4):
		var diff := 0
		for c in 3:
			diff = maxi(diff, absi(da[i + c] - db[i + c]))
		if diff > 0:
			different += 1
			worst = maxi(worst, diff)
	_check(different == 0, "%s: KTX2 draws as PNG (%d pixels differ, by up to %d)" % [what, different, worst])


## バンドルを読んだ `UgoePuppet2D` をキャンバスと同じ大きさのビューポートに置く (tests/conformance.gd と同じ描き方)。
func _view(bundle: PackedByteArray) -> SubViewport:
	var puppet: Node2D = ClassDB.instantiate("UgoePuppet2D")
	puppet.playing = false
	for stage in ["physics", "eye_blink", "breath", "lip_sync", "pose"]:
		puppet.set_stage_enabled(stage, false)
	if not puppet.load_model_from_buffer(bundle):
		_check(false, "load_model_from_buffer")
	puppet.advance(0.0)
	var size: Vector2 = puppet.get_canvas_size().round()
	puppet.position = puppet.get_canvas_origin()
	var viewport := SubViewport.new()
	viewport.size = Vector2i(size)
	viewport.disable_3d = true
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	var background := ColorRect.new()
	background.color = BACKGROUND
	background.size = size
	viewport.add_child(background)
	viewport.add_child(puppet)
	root.add_child(viewport)
	return viewport


func _texr(payload: PackedByteArray) -> Dictionary:
	return {"tag": "TEXR", "flags": 0, "payload": payload}


## バンドルのチャンクを表の順に読む (`docs/spec/file-formats.md#バンドル-ugoe`)。
func _read_chunks(bytes: PackedByteArray) -> Array:
	var chunks := []
	var count := bytes.decode_u32(12)
	for i in count:
		var entry := 16 + i * 24
		var offset := bytes.decode_u64(entry + 8)
		var length := bytes.decode_u64(entry + 16)
		chunks.append({
			"tag": bytes.slice(entry, entry + 4).get_string_from_ascii(),
			"flags": bytes.decode_u32(entry + 4),
			"payload": bytes.slice(offset, offset + length),
		})
	return chunks


## チャンクの列をバンドルに書く (ペイロードを 16 バイト境界に揃える)。
func _write_chunks(chunks: Array) -> PackedByteArray:
	var out := PackedByteArray()
	out.append_array("UGOE".to_ascii_buffer())
	out.append_array(_u32(0))
	out.append_array(_u32(0))
	out.append_array(_u32(chunks.size()))
	var cursor := 16 + chunks.size() * 24
	var offsets := []
	for chunk in chunks:
		cursor = _align(cursor, 16)
		offsets.append(cursor)
		cursor += chunk["payload"].size()
	for i in chunks.size():
		out.append_array(chunks[i]["tag"].to_ascii_buffer())
		out.append_array(_u32(chunks[i]["flags"]))
		out.append_array(_u64(offsets[i]))
		out.append_array(_u64(chunks[i]["payload"].size()))
	for i in chunks.size():
		out.resize(offsets[i])
		out.append_array(chunks[i]["payload"])
	return out


## 圧縮していない RGBA8 の KTX2 (アルファは乗算していない)。
func _ktx2_rgba8(image: Image, srgb: bool) -> PackedByteArray:
	var alpha := DF_CHANNEL_ALPHA | (DF_QUALIFIER_LINEAR if srgb else 0)
	var samples := [
		_sample(0, 8, DF_CHANNEL_COLOR, 255),
		_sample(8, 8, DF_CHANNEL_GREEN, 255),
		_sample(16, 8, DF_CHANNEL_BLUE, 255),
		_sample(24, 8, alpha, 255),
	]
	var transfer := DF_TRANSFER_SRGB if srgb else DF_TRANSFER_LINEAR
	var dfd := _dfd(DF_MODEL_RGBSDA, transfer, [0, 0, 0, 0], 4, samples)
	var vk_format := VK_FORMAT_R8G8B8A8_SRGB if srgb else VK_FORMAT_R8G8B8A8_UNORM
	return _ktx2(vk_format, image, dfd, 4)


## BC3 (DXT5) の KTX2。
func _ktx2_bc3(image: Image) -> PackedByteArray:
	var samples := [
		_sample(0, 64, DF_CHANNEL_ALPHA, 0xFFFFFFFF),
		_sample(64, 64, DF_CHANNEL_COLOR, 0xFFFFFFFF),
	]
	var dfd := _dfd(DF_MODEL_BC3, DF_TRANSFER_LINEAR, [3, 3, 0, 0], 16, samples)
	return _ktx2(VK_FORMAT_BC3_UNORM_BLOCK, image, dfd, 16)


## KTX2 のファイル (ミップマップなし、1 枚)。https://registry.khronos.org/KTX/specs/2.0/ktxspec.v2.html
func _ktx2(vk_format: int, image: Image, dfd: PackedByteArray, align: int) -> PackedByteArray:
	var data := image.get_data()
	var dfd_offset := 80 + 24
	var data_offset := _align(dfd_offset + dfd.size(), align)
	var out := PackedByteArray(KTX2_IDENTIFIER)
	for value in [vk_format, 1, image.get_width(), image.get_height(), 0, 0, 1, 1, 0]:
		out.append_array(_u32(value))
	out.append_array(_u32(dfd_offset))
	out.append_array(_u32(dfd.size()))
	out.append_array(_u32(0))
	out.append_array(_u32(0))
	out.append_array(_u64(0))
	out.append_array(_u64(0))
	out.append_array(_u64(data_offset))
	out.append_array(_u64(data.size()))
	out.append_array(_u64(data.size()))
	out.append_array(dfd)
	out.resize(data_offset)
	out.append_array(data)
	return out


## データ形式の記述 (dfdTotalSize と基本の記述ブロック 1 つ)。アルファは乗算していない (flags = 0)。
func _dfd(model: int, transfer: int, block: Array, bytes_plane0: int, samples: Array) -> PackedByteArray:
	var size := 24 + 16 * samples.size()
	var out := PackedByteArray()
	out.append_array(_u32(4 + size))
	out.append_array(_u32(0))  # vendorId = Khronos、descriptorType = basic
	out.append_array(_u32(2 | (size << 16)))  # versionNumber = 1.3、descriptorBlockSize
	out.append_array(PackedByteArray([model, DF_PRIMARIES_BT709, transfer, 0]))
	out.append_array(PackedByteArray(block))
	out.append_array(PackedByteArray([bytes_plane0, 0, 0, 0, 0, 0, 0, 0]))
	for sample in samples:
		out.append_array(sample)
	return out


func _sample(bit_offset: int, bits: int, channel: int, upper: int) -> PackedByteArray:
	var out := PackedByteArray()
	out.resize(16)
	out.encode_u16(0, bit_offset)
	out[2] = bits - 1
	out[3] = channel
	out.encode_u32(8, 0)
	out.encode_u32(12, upper)
	return out


func _align(value: int, to: int) -> int:
	return (value + to - 1) / to * to


func _u32(value: int) -> PackedByteArray:
	var out := PackedByteArray()
	out.resize(4)
	out.encode_u32(0, value)
	return out


func _u64(value: int) -> PackedByteArray:
	var out := PackedByteArray()
	out.resize(8)
	out.encode_u64(0, value)
	return out
