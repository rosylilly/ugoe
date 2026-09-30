@tool
class_name UgoeModule
extends Resource
## ugoe の着せ替えのモジュール (`.ugoemod` のバイト列)。`load("res://....ugoemod")` で得られ、
## `UgoePuppet2D.attach_module()` や `UgoePuppet2D.modules` に渡す。
##
## 中身はここでは読まない。`UgoePuppet2D` が取り付けるときに検査する (bindings/godot/README.md)。

## `.ugoemod` のバイト列。
@export var data := PackedByteArray()


## バイト列から作る。
static func from_buffer(bytes: PackedByteArray) -> UgoeModule:
	var module := UgoeModule.new()
	module.data = bytes
	return module
