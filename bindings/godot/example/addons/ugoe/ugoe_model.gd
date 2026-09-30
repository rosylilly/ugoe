@tool
class_name UgoeModel
extends Resource
## ugoe のモデル (`.ugoe` のバイト列)。`load("res://....ugoe")` で得られ、`UgoePuppet2D.model` に設定する。
##
## 中身はここでは読まない。`UgoePuppet2D` が読み込むときに検査する (bindings/godot/README.md)。

## `.ugoe` のバイト列。
@export var data := PackedByteArray()


## バイト列から作る。
static func from_buffer(bytes: PackedByteArray) -> UgoeModel:
	var model := UgoeModel.new()
	model.data = bytes
	return model
