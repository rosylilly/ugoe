---
id: T0459
title: "Godot: 読み込みの上限を指定する口・attach_module の 2 度読み・口の入力をやめる口"
status: done
milestone: M9
crates: [bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0459
---

# T0459: Godot: 読み込みの上限を指定する口・attach_module の 2 度読み・口の入力をやめる口

## 目的

- GDExtension から読み込みの上限 (展開の上限と PNG の一辺。SDK の `read_bundle_with` と `ImageLimits`) を指定できない (T0152 の候補)。
- `attach_module` (`bindings/godot/src/node.rs`) が新しいモジュールを 2 回読み、警告も 2 回出る。1 回目に読んだものを `apply_modules` に渡す (T0456 の候補)。
- `set_lip_sync_level` は負の値を 0 に丸めるので、口の入力をやめられない (C ABI・wasm・Unity は負か null でやめられる。T0454 の候補)。

## 受け入れ条件

- [x] ノード (またはリソースの読み込み) で上限を指定でき、上限を超えたときのエラーを GDScript から読める。テストがある
- [x] `attach_module` が 1 回だけ読み、警告が 1 回だけ出ることを確かめるテストがある
- [x] 口の入力をやめる口 (`clear_lip_sync_level` か、負の値の扱いをほかの組み込み先に揃える) があり、テストがある
- [x] bindings/godot/README.md を更新した
- [x] `cargo clippy --manifest-path bindings/godot/Cargo.toml --all-targets -- -D warnings`・`just godot-check`・`just ci` が通る

## 参考

- T0152・T0456・T0454 のメモ (起票の候補)、ADR 0026

## メモ

- T0026 で起票した (元の候補: T0152 の Godot の上限、T0456 の 2 度読み、T0454 の `set_lip_sync_level`)。
- 2026-10-04 (claude-worker-T0459): 実装した。
  - **読み込みの上限**: `UgoePuppet2D` に `max_chunk_decompressed`・`max_total_decompressed`・`max_image_side` (インスペクタの
    「Load limits」。既定値は SDK の `ReadLimits` と wasm の `max_image_side` と同じ。負と画像の 2^32 以上は拒む) と `get_load_error()` を足した。
    次の読み込み (モデルと着せ替えのモジュール) から効く。画像は Godot がデコードするので、デコードの前に PNG の `IHDR` と KTX2 の見出しの
    大きさで調べる (`src/model.rs` の `image_size`。見出しを読めない画像は調べない)。`get_load_error()` は `{ "code", "error", ... }` で、
    `code` と補足のキーは wasm の `LoadError::info` (`read_limit_exceeded`・`image_limit_exceeded`・`invalid_file` など) と着せ替えの
    `DressUpError::code` に揃えた。成功した読み込みと付け外しで空に戻る。上限の外の失敗にもコードを付けたのは、辞書の形を 1 つにするため。
  - **attach_module の 2 度読み**: `apply_modules` を、リソースを読む部分と読んだものを付ける `apply_parsed_modules` に分け、`attach_module` は
    読んだ組をそのまま渡す。`parse_module` は失敗を `get_load_error()` に残すため `&mut self` にし、`&self` から呼ぶ所 (`detach_module`・
    `get_module_names`) は残さない `read_module` を使う。
  - **口の入力をやめる口**: `clear_lip_sync_level` は前からあった (テストが無かった)。加えて `set_lip_sync_level` の負の値 (`-INF` を含む) を
    C ABI・wasm・Unity と同じく「入力をやめる」に揃えた (タスクの受け入れ条件に挙がっていた案の両方)。前は 0 にしていたので、負を 0 の
    意味で渡していたスクリプトは口を閉じる代わりに入力をやめる (README に書いた)。
  - テスト: `cargo test` (`src/model.rs`: 見出しの大きさ・本体のテクスチャとモジュールの法線マップの一辺の上限・バンドルとモジュールの展開の上限・
    ほかの失敗のコード)。`example/tests/smoke.gd`: 上限 (画像・合計・1 チャンク・モジュール)・`get_load_error()`・負の値の拒否、
    負の音量・`-INF`・`clear_lip_sync_level` で `get_lip_sync_applied()` が `(-1, 1)` になる、`attach_module` の警告の数
    (`OS.add_logger` で数える `example/tests/warning_counter.gd`。`Logger` は Godot 4.5 からなので、それより前は飛ばす)。
    警告の数のテストは、`attach_module` を元に戻すと「6 warnings, 12 logged」で失敗することを確かめた。
  - 文書: `bindings/godot/README.md` (メンバーの表・「読み込みの上限 (T0459)」の節・有限でない値の段落)、`docs/portability.md` の表の Godot の列、
    `docs/spec/file-formats.md#読み込みの上限` に Godot を足した。
  - 確かめたこと (main に rebase した後): `just ci`、`cargo clippy --manifest-path bindings/godot/Cargo.toml --all-targets -- -D warnings`、
    `cargo fmt --check` (bindings/godot)、`node bindings/godot/build.mjs --check --conformance` (Godot 4.7.1・Xvfb。ライセンス ok・smoke ok・
    適合テスト 63 / 63・ktx2 ok)。
  - やらなかったこと: `ugoe_resource_loader.gd` (リソースとして `load()` するとき) は中身を読まないので上限を持たない (上限はノードが読むときに効く)。
    Godot 4.4 での smoke (警告の数は飛ばす) は確かめていない。
  - 起票の候補: なし。
