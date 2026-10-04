extends Logger
## Godot の警告とエラーのうち、`needle` を含むものを数える (`smoke.gd` が `OS.add_logger` で付ける)。
## `Logger` は Godot 4.5 から。それより前の Godot では `smoke.gd` がこのスクリプトを読まずに確かめを飛ばす。

var needle := ""
var count := 0
var _mutex := Mutex.new()


func _log_error(_function: String, _file: String, _line: int, code: String, rationale: String, _editor_notify: bool, _error_type: int, _script_backtraces: Array[ScriptBacktrace]) -> void:
	if needle != "" and (code.contains(needle) or rationale.contains(needle)):
		_mutex.lock()
		count += 1
		_mutex.unlock()
