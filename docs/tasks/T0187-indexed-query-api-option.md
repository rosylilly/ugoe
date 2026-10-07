---
id: T0187
title: "ランタイム: 添字を取る問い合わせの API (ParameterValues::get など) を、パニックせず Option を返す形にするかを決める (判断が要る)"
status: todo
milestone: M11
crates: [ugoe-runtime, ugoe, ugoe-ffi, ugoe-wasm, bindings/godot, docs]
depends_on: [T0493]
assignee:
---

# T0187: ランタイム: 添字を取る問い合わせの API (ParameterValues::get など) を、パニックせず Option を返す形にするかを決める (判断が要る)

## 目的

T0184 でランタイム層をパニックしないようにしたが、添字を引数に取る Rust の公開の問い合わせ (`ParameterValues::get`・`set`・`min`・
`max`・`default_value`・`blend`、`ModelInstance::part_opacity`・`part_opacity_override`・`deformer_world`・`deformer_parent_world`・
`mesh_parent_world`・`mesh_local_positions`・`mesh_contains`・`hit_area_contains`、`CompiledModel::mask_refs`・`mask_mode`・
`mask_depth`・`hit_area_mesh`) は、`Vec` の添字と同じく範囲の外でパニックし、`# Panics` に書く約束のまま残した
(docs/portability.md#パニックしない の「パニックしてよいもの」)。上の層 (C ABI・wasm・Godot) が先に範囲を確かめる。
`std` の無い環境ではパニックが abort になるので、この約束を残すか、パニックしない形に変えるかを決める。

**判断が要る** (公開 API の変更)。作業者は下の案から勝手に選ばず、利用者に確かめてから進める。

### 案

- **A. 今のまま (`# Panics` の約束):** `Vec` の添字や `slice::index` と同じ慣習。上の層は範囲を確かめ、lint
  (`indexing_slicing`) の `#[expect]` は理由付きで残る。変更なし。上の層の確かめ忘れが `std` の無い形では abort になる。
- **B. `Option` (または `Result`) を返す形に変える:** `get(index) -> Option<f32>`・`part_opacity(part) -> Option<f32>` など。
  `set` のような書く API は、ほかの書く API (`set_part_opacity_override`) と同じく範囲の外で何もしない (または `bool` を返す)。
  呼び出し側 (`ugoe`・`ugoe-ffi`・`ugoe-wasm`・`bindings/godot`・ツール層) を直す。`#[expect]` が消え、範囲の確かめが型で強制される。
- **C. 両方を持つ:** パニックしない `get`・`try_*` を足し、パニックする形は `*_unchecked` 風の別名か `Index` の実装に残す。
  API が倍になる。

### 推奨

B。リリース前で互換の縛りが無く (利用者の方針)、M11 の「std の無い環境でパニックしない」を型で守れる。毎フレームの経路は
すでに `get` で書いていて、`Option` の分岐の重さは無視できる (変える前と後で `Puppet::update` の時間を比べて確かめる)。

## 受け入れ条件

- [ ] 利用者に案を示して方針を決め、決めたことと理由を docs/portability.md#パニックしない に書いた
- [ ] 決めた方針に合わせて API と呼び出し側を直し、`#[expect(clippy::indexing_slicing, reason = "…(# Panics)")]` の残りを減らした
      (A ならこの項目は不要)。範囲の外の番号で `None` (や何もしないこと) になるテストを足した
- [ ] 公開の型やメソッドの形を変えたなら、crates の README・docs/spec (対応する節があれば英語の `spec/` も) を同じ PR で直した
- [ ] `just nostd-check`・`just wasm-check`・`just conformance` と `just ci` が通る

## 参考

- T0184 のメモ (やらなかったこと・起票の候補 (b))、docs/portability.md#パニックしない
- crates/ugoe-runtime/src/parameters.rs・instance.rs・compile.rs・hit.rs (`# Panics`)

## メモ

- T0066 で起票した (元の候補: T0184 (添字を取る問い合わせの API を Option を返す形にするかの判断))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- 呼び出し側の `ugoe-wasm` を触るので、同じクレートに lint を足す T0493 の後にした (T0493 は判断を待たずに進められる)。
