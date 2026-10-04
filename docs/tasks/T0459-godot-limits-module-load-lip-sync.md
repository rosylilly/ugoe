---
id: T0459
title: "Godot: 読み込みの上限を指定する口・attach_module の 2 度読み・口の入力をやめる口"
status: todo
milestone: M9
crates: [bindings/godot, docs]
depends_on: []
assignee:
---

# T0459: Godot: 読み込みの上限を指定する口・attach_module の 2 度読み・口の入力をやめる口

## 目的

- GDExtension から読み込みの上限 (展開の上限と PNG の一辺。SDK の `read_bundle_with` と `ImageLimits`) を指定できない (T0152 の候補)。
- `attach_module` (`bindings/godot/src/node.rs`) が新しいモジュールを 2 回読み、警告も 2 回出る。1 回目に読んだものを `apply_modules` に渡す (T0456 の候補)。
- `set_lip_sync_level` は負の値を 0 に丸めるので、口の入力をやめられない (C ABI・wasm・Unity は負か null でやめられる。T0454 の候補)。

## 受け入れ条件

- [ ] ノード (またはリソースの読み込み) で上限を指定でき、上限を超えたときのエラーを GDScript から読める。テストがある
- [ ] `attach_module` が 1 回だけ読み、警告が 1 回だけ出ることを確かめるテストがある
- [ ] 口の入力をやめる口 (`clear_lip_sync_level` か、負の値の扱いをほかの組み込み先に揃える) があり、テストがある
- [ ] bindings/godot/README.md を更新した
- [ ] `cargo clippy --manifest-path bindings/godot/Cargo.toml --all-targets -- -D warnings`・`just godot-check`・`just ci` が通る

## 参考

- T0152・T0456・T0454 のメモ (起票の候補)、ADR 0026

## メモ

- T0026 で起票した (元の候補: T0152 の Godot の上限、T0456 の 2 度読み、T0454 の `set_lip_sync_level`)。
