---
id: T0157
title: 着せ替えの失敗の補足の値の不足を埋める
status: todo
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, docs]
depends_on: [T0139]
assignee:
---

# T0157: 着せ替えの失敗の補足の値の不足を埋める

## 目的

`DressUpError::fields` (T0412。wasm の `DressUpError` と C ABI の `ugoe_last_error_*` が使う) は、`ComposeError` が持つ値の一部を落としている。
組み込み先が利用者に分かる文を作るのに要る値が無い (T0139 のレビューで見つけた)。

| `code` | 落ちている値 |
| --- | --- |
| `unknown_mask` | `mask` (参照先) |
| `unknown_collider` | `rig`・`collider` |
| `pose_not_exposed` | `part` |
| `texture_out_of_range` | `texture` (番号) |
| `avatar_level` | `required`・`actual` (レベル) |
| `invalid_physics`・`invalid_composition` | 検査のエラー (どれか 1 つでも) |

キーを足すかを決め、足すなら `fields` (値が番号のものは `FieldValue` のような型にするか) と wasm・C ABI の出し方をそろえる。
`code` の値は変えない。

## 受け入れ条件

- [ ] 足すキーと値の型を決め、crates/ugoe/README.md と `DressUpError::fields` の doc に書いた
- [ ] wasm (`crates/ugoe-wasm/src/dress_up.rs`) と C ABI (`crates/ugoe-ffi/src/dress_up.rs`・`error.rs`) が同じキーを出すことをテストで確かめた
- [ ] `just ci` が通る

## 参考

- crates/ugoe/src/dress_up.rs (`DressUpError::info`)
- crates/ugoe-runtime/src/compose.rs (`ComposeError`)
- docs/spec/modules.md#puppet-での着せ替え

## メモ
