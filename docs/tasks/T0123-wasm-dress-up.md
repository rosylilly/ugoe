---
id: T0123
title: 着せ替え: wasm から付け外しできるようにする
status: done
milestone: M4
crates: [ugoe-wasm, ugoe, docs, apps/editor, apps/viewer, Cargo.toml]
depends_on: [T0121, T0113]
assignee: claude-worker-T0123
---

# T0123: 着せ替え: wasm から付け外しできるようにする

## 目的

ADR 0024。ビューアとエディタのプレビュー (どちらも wasm で描く) で着せ替えを試せるように、wasm に `.ugoemod` の読み込みと付け外しの口を足す (T0121 の後続の候補)。

## 受け入れ条件

- [x] `.ugoemod` のバイト列を読み込み、`Puppet` の `attach_module` / `detach_module` / `set_modules` を呼べる。`DressUpChange` に従ってテクスチャ (と法線マップ) をレンダラに登録・差し替える
- [x] 付けているモジュールの一覧と、モデルのスロットの一覧を返す
- [x] wasm のテスト (または wasm-bindgen を通さない Rust のテスト) と `just wasm-check`。ビューアとエディタの stub.d.ts を合わせる
- [x] `just ci` が通る

## 参考

- `docs/adr/0024-module-authoring.md`、`docs/spec/modules.md#puppet-での着せ替え`

## メモ

- JS の API (`src/web/dress_up.rs`、使い方は `crates/ugoe-wasm/README.md#着せ替え-t0123`):
  `load_module(bytes) -> name` (読み込むだけで付けない)・`attach_module(name)`・`detach_module(name)`・`set_modules(names)`・
  `unload_module(name)`・`attached_modules()`・`modules_json()` (`[{name, slot, requires, attached, textures, warnings}]`)・
  `slots_json()` (本体のスロット。モデルの JSON 形式)。エディタのパネル (T0649) で「読み込む」と「付ける」を分けられるよう、読み込みと付け外しを分けた。
- エラー: `name` が `"DressUpError"` の JS の `Error` に、`code` (`unknown_slot`・`missing_parameter`・`not_attached` など) と
  `module`・`slot`・`parameter`・`mesh`・`attached` を付ける (`dress_up::error_info`)。このために `js-sys` を
  `[workspace.dependencies]` に足した (web-sys 経由で Cargo.lock に既にあった 0.3。wasm32 のときだけ使う)。
  ルートの `Cargo.toml` は範囲外なので `crates:` に足した (team-lead 承認済み)。コードの一覧は
  `crates/ugoe-wasm/README.md#着せ替え-t0123` に書いた (安定した値として扱い、足すときは README も直す)。
- テクスチャ: ネイティブでテストできる `dress_up::Wardrobe` が読み込んだモジュール (デコード済みの画像) と、レンダラに登録した
  番号の範囲を持つ。`DressUpChange::textures` の番号にモジュールの画像と法線マップを登録し直し、使わなくなった番号は
  1x1 の白に置き換えて法線マップを外す (`Renderer` にテクスチャを外す API が無いため。GPU のメモリは解放される)。
  モデルの読み込み (`load_json` / `load_bundle`) も `Wardrobe::load_base` を通るようにし、前のモデルの方が多かったテクスチャも捨てるようにした
  (T0210 の `set_normal_maps` と `normal_map_slots` はこれに置き換えた。光源の JS の API は変えていない)。
- モデルを読み直すと、付けていたモジュールを付け直す (エディタは編集のたびに読み直すため)。まとめて付けられなければ 1 つずつ試し、
  付けられないものは外して警告を返す。そのとき物理は合成した設定ではなく本体の設定を引き継ぐ (`Puppet::base_physics` を
  `ugoe` に足し、`animation::carry_over` で使う)。そうしないとモジュールのリグが新しいモデルと合わず、物理ごと外れていた。
- テスト: `crates/ugoe-wasm/src/dress_up/tests.rs` (登録を記録する `TextureSink` で、付け外し・番号のずれ・捨てる・読み直し・
  新しい版の読み込み・エラーのコード)。`just wasm-check`。
- ヘッドレスの Chrome (WebGL2・SwiftShader) で、`just wasm` の生成物に `dress_up_base` と `LongHair`・`Dress` の `.ugoemod` を
  読ませて確かめた (付け外し・`DressUpError` の `code`・読み直しで付け直す・`LongHair` を外すと `Dress` のテクスチャが 1 番に
  ずれて正しく描かれる)。スクリーンショットは作業時の scratchpad (`wasm-T0123/shot.png`)。
- `crates:` に `apps/editor` と `apps/viewer` を足した (team-lead 承認済み。stub.d.ts に型を 1 行ずつ足すため)。
- 後続の候補:
  - `ugoe-render` にテクスチャを外す API (`remove_texture`)。今は 1x1 の白で置き換えている。
  - モジュールを付けている間の `set_physics_json` (エディタの物理の編集) は次の付け外しで本体の設定に戻る。本体の物理を差し替える API。
  - FFI からの付け外し。
