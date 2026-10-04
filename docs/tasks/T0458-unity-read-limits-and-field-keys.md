---
id: T0458
title: "Unity: 読み込みの上限を指定する口と、エラーの補足の値のキーを C ABI に揃える"
status: done
milestone: M9
crates: [bindings/unity, scripts, docs]
depends_on: []
assignee: claude-worker-T0458
---

# T0458: Unity: 読み込みの上限を指定する口と、エラーの補足の値のキーを C ABI に揃える

## 目的

- T0152 で C ABI に読み込みの上限 (`ugoe_read_limits_default`・`ugoe_puppet_create_from_bundle_with_limits`・`ugoe_module_create_with_limits`) を足したが、
  Unity の `Native.cs` には宣言が無く、`scripts/check-ffi-bindings.mjs` の OMIT で除いている (T0152・T0455 の候補)。`UgoeResult.LimitExceeded` (10) は T0455 で足した。
- `Runtime/Core/UgoeException.cs` の `FieldKeys` は固定の一覧 (`module`・`slot`・`parameter`・`mesh`・`attached`・`limit`・`chunk`・`max`) で、
  T0157・T0434 で増えたキー (`mask`・`rig`・`collider`・`part`・`texture`・`required`・`actual`・`errors`・`image`・`width`・`height` など) が `Fields` に入らない (T0157 の候補)。
- その一覧が `crates/ugoe-wasm/codes.json` のキーと食い違っても検出できない (T0452 の候補)。

## 受け入れ条件

- [x] `UgoeReadLimits` と、`Puppet.FromBundle`・`Module.FromBytes` に上限を渡す多重定義があり、OMIT から 3 つの関数を外した。dotnet のテストがある
- [x] `UgoeModel` の取り込みで上限を指定できるかを検討し、結果をメモか README に書いた
- [x] `FieldKeys` を `codes.json` のすべてのキーに広げ (または C ABI にキーを列挙する関数を足す)、食い違いを検出するテストがある
- [x] bindings/unity/README.md を更新した
- [x] `just unity-check` と `just ci` が通る (実際の Unity で確かめていないことはメモに)

## 参考

- T0152・T0455・T0157・T0452 のメモ (起票の候補)
- bindings/unity/com.ugoe.unity/Runtime/Core/UgoeException.cs・Native.cs、scripts/check-ffi-bindings.mjs

## メモ

- T0026 で起票した (元の候補: T0152 と T0455 の Unity の上限、T0157 の Unity の `FieldKeys`、T0452 の `FieldKeys` と `codes.json` の比較)。
- 集計にあった「C# の `UgoeResult` に 10 が無い」は T0455 で足してあった (`UgoeResult.LimitExceeded = 10`)。
- やったこと (T0458):
  - `Runtime/Core/Types.cs` に `UgoeReadLimits` (blittable。`Default` は `ugoe_read_limits_default` を呼ぶ)、`Native.cs` に 3 つの関数の宣言、
    `Puppet.FromBundle(bytes, limits)`・`Module.FromBytes(bytes, limits)` を足し、`scripts/check-ffi-bindings.mjs` の Unity の OMIT から 3 つを外した
    (残りは `ugoe_puppet_positions` だけ)。`tests/LimitsTests.cs` で、既定値・バンドル (`spec/examples/bundles/single-quad-zstd.ugoe`) と
    モジュールの合計と 1 チャンクの上限・`Fields` (`limit`・`chunk`・`max`)・上限の外の失敗 (`Format`・`invalid_file`) を確かめる。
    見本 (assets/samples) は圧縮していないので、圧縮したモジュール `bindings/unity/tests/Fixtures/short-hair-zstd.ugoemod`
    (`ugoe build --compress` で作った。作り方は同じフォルダの README.md) を足した。
  - `UgoeModel` の検討: `CreatePuppet(UgoeReadLimits)` の多重定義だけを足した。`UgoePuppet`・`UgoeModule.Module`・取り込みは既定の上限のまま
    (アセットに上限を持たせない。理由は bindings/unity/README.md#読み込みの上限)。
  - `UgoeException.FieldKeys` を `codes.json` の `dress_up_error` と `load_error` のすべてのキー (22 個) に広げ、`tests/ResultTests.cs` の
    `FieldKeysMatchTheCodes` が `codes.json` と比べる (C ABI にキーを列挙する関数は足さなかった。`crates:` の外で、ABI を増やすほどではないため)。
    `load_warning` のキーは警告の口 (`GetLoadWarningField`) で読むので対象の外。
  - `crates:` の外の小さな変更: `crates/ugoe-ffi/README.md` の「Unity はまだこの口を出していない」の 1 行を直した。
- 確かめていないこと: 実際の Unity (Mono・IL2CPP) での `ref UgoeReadLimits` の受け渡しと、`UgoeModel.CreatePuppet(limits)` の実行
  (型検査は tests/UnityCompile のスタブで、実行は dotnet (net9.0) の P/Invoke だけ)。
- 起票の候補:
  - C ABI にエラーの補足の値のキーを列挙する関数 (`ugoe_last_error_field_count`・`ugoe_last_error_field_at` など) を足せば、`FieldKeys` の写しが要らなくなる。
  - 着せ替えの失敗で新しいキー (`mask`・`collider` など) が `Fields` に入ることを、実際の失敗 (例: `unknown_mask` のモジュール) で確かめる C# のテスト
    (今は `codes.json` との一覧の比較だけ)。

