---
id: T0464
title: "Unity: 読み込みのエラーの説明とテストを直し、補足の値のキーを C ABI から列挙するか決める (判断が要る)"
status: done
milestone: M9
crates: [bindings/unity, ugoe-ffi, docs]
depends_on: []
assignee: claude-worker-T0464
---

# T0464: Unity: 読み込みのエラーの説明とテストを直し、補足の値のキーを C ABI から列挙するか決める (判断が要る)

## 目的

T0457 で C ABI のモデルの読み込みの失敗にも種類 (`Kind`) が付き、T0458 で Unity の `UgoeException.Fields` のキーを `codes.json` と比べる
`FieldKeys` の写しを置いた。次が残っている (T0457・T0458 の候補)。

1. Unity の `UgoeResult.Format`・`InvalidModel`・`UgoeException` の説明が、モデルの読み込みに種類が無かったころのまま。`Puppet.FromBundle` の失敗の `Kind` を確かめるテストも無い。
2. 着せ替えの失敗で新しいキー (`mask`・`collider` など) が `Fields` に入ることを、実際の失敗 (例: `unknown_mask` のモジュール) で確かめる C# のテストが無い (今は `codes.json` との一覧の比較だけ)。
3. **判断が要る:** C ABI に補足の値のキーを列挙する関数 (`ugoe_last_error_field_count`・`ugoe_last_error_field_at` など) を足せば、Unity の `FieldKeys` の写しが要らなくなる。
   T0458 は「ABI を増やすほどではない」として足さなかった。C ABI の公開 API を増やすかを利用者が決める。

## 受け入れ条件

- [x] `UgoeResult.Format`・`InvalidModel`・`UgoeException` の doc コメントと bindings/unity の README を、読み込みの失敗の種類に合わせて直した
- [x] `Puppet.FromBundle` の失敗 (壊れたバンドル・上限の超過) の `Kind` を確かめる C# のテストがある
- [x] 実際の着せ替えの失敗 (`unknown_mask` のモジュールなど) で、`Fields` に新しいキーが入ることを確かめる C# のテストがある
- [x] キーを列挙する関数を足すかの案と推奨をメモに書き、利用者の判断を得た。足すなら C ABI のヘッダ・`just refs`・Unity の宣言を揃え、`FieldKeys` の写しを外した
- [x] Unity のテストを README の手順で流した (実際の Unity が無ければ、確かめた範囲をメモに)
- [x] `just ci` が通る

## 参考

- T0457・T0458 のメモ (起票の候補)
- crates/ugoe-ffi/README.md、bindings/unity/README.md

## メモ

- T0032 で起票した (元の候補: T0457 (Unity)・T0458 (2 つ))。
- 判断 (2026-10-05、利用者の判断): C ABI に補足の値のキーを列挙する関数 (`ugoe_last_error_field_count`・`ugoe_last_error_field_at` など) を足す (足すだけで ABI の互換は保つ)。Unity は `FieldKeys` の写しをやめてこれを使う。
- やったこと (T0464):
  - C ABI に `ugoe_last_error_field_count()`・`ugoe_last_error_field_key_at(index)`・`ugoe_last_error_field_value_at(index)` を足した
    (crates/ugoe-ffi/src/error.rs。範囲外は NULL、並びは SDK の `fields` の順、値の無いキー (`actual` の None) は入らない)。
    `ugoe_last_error` などと同じく `guard` を通さない (列挙の呼び出しが最後のエラーを上書きしないように)。足しただけなので `ABI_VERSION` は上げていない。
    ヘッダを作り直し、C の見本 (`examples/c/sample.c`)・Rust のテスト (`error.rs`・`dress_up.rs` の全種類・`tests/load_errors.rs` の実際の読み込みの失敗で、
    キーで読んだ値と列挙が一致すること) を足した。crates/ugoe-ffi/README.md と docs/portability.md に書いた。
  - Unity: `UgoeException.FieldKeys` の写しを消し、`FromLastError` は列挙で `Fields` を埋める (種類を持たない失敗では C ABI の数が 0 なので空)。
    `Native.cs` に 3 つの宣言を足した (`ugoe_last_error_field` の宣言はヘッダとの過不足の検査のため残す)。
    `UgoeResult.Format`・`InvalidModel`・`UgoeException`・`Fields` の doc コメントと bindings/unity/README.md を、読み込みの失敗の種類に合わせて直した。
  - Unity のテスト (`tests/ResultTests.cs`): `FieldKeysMatchTheCodes` を、実際の失敗の種類とキーを codes.json と比べるテストに置き換えた。
    バンドルの読み込み (壊れたバンドル `empty-chunk-header.ugoe`・`invalid_file`、`required-feature.ugoe`・`unsupported_features`、
    `assets/compat/future/newer-schema.ugoe`・`unsupported_version`、`invalid-motion.ugoe`・`invalid_model`、上限の超過・`read_limit_exceeded`)、
    JSON の読み込み (`unsupported_version`・`invalid_file`)、着せ替えの `unknown_mask` (互換性テストの本体 `assets/compat/dev/dress-up-base.ugoe` に
    見本の `long-hair.ugoemod` を付ける。`mask` = `Skin`・`mesh` = `Back`)。
  - 確かめたこと: `just ci`・`just wasm-check`・`just ffi-check`・`just refs`・`just unity-check` (dotnet test 96 件、UnityCompile の netstandard2.1 と net48)・
    `just conformance` (c・wasm・wasm-webgpu・avatar・queries・unity・godot) がすべて通る。
- やらなかったこと: 実際の Unity エディタ・プレイヤーでの実行は確かめていない (手元に Unity が無い。README の手順の `just unity-check` の範囲まで)。
  C# の `Fields` は同じキーが 2 度来たら後の値で上書きする (SDK の `fields` にキーの重なりは無い)。
- 起票の候補:
  - `unknown_mask` のテストは互換性テストの資料 (`assets/compat/dev`) と見本 (`assets/samples`) の組み合わせに頼る。どちらかを作り直して
    マスクの参照先が変わると落ちるので、気になるなら Unity のテスト用の小さなモジュールを `tests/Fixtures/` に置く。
