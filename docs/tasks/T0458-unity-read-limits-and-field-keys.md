---
id: T0458
title: "Unity: 読み込みの上限を指定する口と、エラーの補足の値のキーを C ABI に揃える"
status: in-progress
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

- [ ] `UgoeReadLimits` と、`Puppet.FromBundle`・`Module.FromBytes` に上限を渡す多重定義があり、OMIT から 3 つの関数を外した。dotnet のテストがある
- [ ] `UgoeModel` の取り込みで上限を指定できるかを検討し、結果をメモか README に書いた
- [ ] `FieldKeys` を `codes.json` のすべてのキーに広げ (または C ABI にキーを列挙する関数を足す)、食い違いを検出するテストがある
- [ ] bindings/unity/README.md を更新した
- [ ] `just unity-check` と `just ci` が通る (実際の Unity で確かめていないことはメモに)

## 参考

- T0152・T0455・T0157・T0452 のメモ (起票の候補)
- bindings/unity/com.ugoe.unity/Runtime/Core/UgoeException.cs・Native.cs、scripts/check-ffi-bindings.mjs

## メモ

- T0026 で起票した (元の候補: T0152 と T0455 の Unity の上限、T0157 の Unity の `FieldKeys`、T0452 の `FieldKeys` と `codes.json` の比較)。
- 集計にあった「C# の `UgoeResult` に 10 が無い」は T0455 で足してあった (`UgoeResult.LimitExceeded = 10`)。
