---
id: T0456
title: "Godot: 着せ替えのモジュールの読み込みの警告をスクリプトから読む"
status: in-progress
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

- [ ] GDScript から、モジュールの読み込みの警告を `code` と補足の値つきで読める (形は wasm の警告に揃える)
- [ ] 警告の出るモジュールを読むテスト (example/tests の GDScript) がある
- [ ] `bindings/godot/README.md` を更新した
- [ ] `just godot-check` と `just ci` が通る

## 参考

- T0449 のメモ (起票の候補)、T0125
- crates/ugoe/src/warning.rs

## メモ

- T0019 で起票した (元の候補: T0449 の着せ替えの警告)。bindings/godot を触る T0450 の後に着手する。
