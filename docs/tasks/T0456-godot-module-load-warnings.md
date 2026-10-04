---
id: T0456
title: "Godot: 着せ替えのモジュールの読み込みの警告をスクリプトから読む"
status: done
milestone: M9
crates: [bindings/godot, docs]
depends_on: [T0450]
assignee: claude-worker-T0456
---

# T0456: Godot: 着せ替えのモジュールの読み込みの警告をスクリプトから読む

## 目的

着せ替えのモジュールを読み込むときの警告 (物理を外した・テクスチャを代用したなど。`LoadWarning`) は、SDK・wasm (T0125) では構造化されたデータで返るが、
Godot ではスクリプトから読めない (ログに出るだけ。T0449 の候補)。`UgoeModule` のプロパティか、`attach_module` の結果で読めるようにする。

## 受け入れ条件

- [x] GDScript から、モジュールの読み込みの警告を `code` と補足の値つきで読める (形は wasm の警告に揃える)
- [x] 警告の出るモジュールを読むテスト (example/tests の GDScript) がある
- [x] `bindings/godot/README.md` を更新した
- [x] `just godot-check` と `just ci` が通る

## 参考

- T0449 のメモ (起票の候補)、T0125
- crates/ugoe/src/warning.rs

## メモ

- T0019 で起票した (元の候補: T0449 の着せ替えの警告)。bindings/godot を触る T0450 の後に着手する。

### やったこと

- `UgoePuppet2D.get_module_load_warnings(name)` を足した (`bindings/godot/src/node/sdk.rs`)。取り付けている名前が `name` の
  モジュールの読み込みの警告を、`get_load_warnings()` と同じ形 (`{ "code", "warning", "texture"? / "feature"? / "path"? }`) で返す。
  wasm の `modules_json()` の `warning_details`・C ABI の `ugoe_module_warning_*` と同じ `code` と補足のキー (SDK の
  `LoadWarning::code`・`fields`)。辞書にする処理は `get_load_warnings` と共有する関数にした。
- 形の選び方: `UgoeModule` は GDScript のリソースで中身を読まない (ADR 0026。ワーカースレッドから呼ばれる) ので、プロパティにはしなかった。
  `attach_module` の戻り値 (bool) を変えると既存のスクリプトが壊れるので、それも変えなかった。付いているモジュールの探し方は
  `get_module_names()` と同じ (モデルを読み込んでいれば実際に付いているもの、無ければ `modules` から読んだもの)。付いていなければ空。
- smoke (`example/tests/smoke.gd` の `_sdk_features`) に、`assets/compat/dev/dress-up-base.ugoe` に `dev/long-hair.ugoemod` (警告なし) と
  `future/optional-feature.ugoemod` (`unknown_optional_feature` と補足の `feature`) を付けて読む検査を足した。付いていない名前・外した後は空。
- README の表・「有限でない値」の後の段落・「制限」を直した。

### やらなかったこと

- 付ける前のモジュールから警告を読む口 (`UgoeModule` 側) は作っていない (README の「制限」)。
- Godot の警告への出力は前のまま。`attach_module` は同じモジュールを 2 回読むことがあり (付けているものに無いリソースは
  `attach_module` と `apply_modules` の両方で読む)、そのとき Godot の警告も 2 回出る (前からの挙動)。

### 検査

- `cargo test` (36 件)・`cargo clippy --all-targets -D warnings`・`cargo fmt --check` (bindings/godot): 通る
- `node bindings/godot/build.mjs --check --conformance`: ライセンス ok、smoke ok、適合テスト 57 / 57、ktx2 ok
- `just ci`: 通る (main に rebase した後)

### 起票の候補

- `attach_module` でモジュールを 2 回読む (警告が 2 回出る) のを、1 回目に読んだものを `apply_modules` に渡して避ける。
