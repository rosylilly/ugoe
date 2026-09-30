@tool
class_name UgoeResourceLoader
extends ResourceFormatLoader
## `.ugoe` を `UgoeModel`、`.ugoemod` を `UgoeModule` として読む。
##
## `class_name` を持つ `ResourceFormatLoader` は Godot が自動で登録する。登録すると、これらのファイルは
## ファイルシステムのドックに型付きで出て、`load()` で読め、書き出しにも含まれる。
## Godot は読み込み器をワーカースレッドからも呼ぶので、GDExtension (godot-rust) ではなく GDScript で書いている。

## ファイルの先頭の 4 バイト。
const MAGIC := "UGOE"


func _get_recognized_extensions() -> PackedStringArray:
	return PackedStringArray(["ugoe", "ugoemod"])


func _handles_type(type: StringName) -> bool:
	return type == &"Resource"


func _get_resource_type(path: String) -> String:
	match path.get_extension().to_lower():
		"ugoe", "ugoemod":
			return "Resource"
	return ""


func _get_resource_script_class(path: String) -> String:
	match path.get_extension().to_lower():
		"ugoe":
			return "UgoeModel"
		"ugoemod":
			return "UgoeModule"
	return ""


func _load(path: String, _original_path: String, _use_sub_threads: bool, _cache_mode: int) -> Variant:
	var bytes := FileAccess.get_file_as_bytes(path)
	if bytes.is_empty():
		return FileAccess.get_open_error()
	if bytes.slice(0, 4) != MAGIC.to_ascii_buffer():
		push_error("ugoe: `%s` is not a ugoe file" % path)
		return ERR_FILE_UNRECOGNIZED
	match path.get_extension().to_lower():
		"ugoe":
			return UgoeModel.from_buffer(bytes)
		"ugoemod":
			return UgoeModule.from_buffer(bytes)
	return ERR_FILE_UNRECOGNIZED
