---
id: T0455
title: "Unity: 受け手の口 (follow) を足し、適合テストの *-follow の skip をなくす"
status: done
milestone: M9
crates: [bindings/unity, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0455
---

# T0455: Unity: 受け手の口 (follow) を足し、適合テストの *-follow の skip をなくす

## 目的

C ABI・wasm・Godot には受け手の口 (`follow`) があり、適合テストの `*-follow` のケースで参照と比べる (`tests/conformance/src/main.rs` の `FOLLOW_BINDINGS`)。
Unity だけが口を持たず、`*-follow` を skip する (T0449 の候補)。C# から C ABI の `follow` を呼ぶ口を足し、比べ合わせに入れる。

## 受け入れ条件

- [x] `Runtime/Core` に `follow` の口があり、dotnet のテストがある (値の長さの不一致・NaN の扱いは C ABI に揃える)
- [x] `FOLLOW_BINDINGS` に Unity を足し、`*-follow` のケースが skip されない
- [x] `bindings/unity/README.md` と tests/conformance/README.md を更新した
- [x] `just unity-check`・`just conformance`・`just ci` が通る

## 参考

- T0449 のメモ (起票の候補)
- tests/conformance/README.md (`FOLLOW_BINDINGS`)

## メモ

- T0019 で起票した (元の候補: T0449 の Unity の受け手)。
- (T0455) やったこと:
  - `Runtime/Core` の `Puppet.Follow`・`SetBaseValues` と `UgoePuppet.Follow` は T0430 で既にあった。足したのは適合テストの
    実行器 (`tests/conformance/unity/Program.cs`) で計画の `follow`・`follow_lip_sync`・`follow_part_opacities` を
    `Puppet.Follow` で進めることと、`FOLLOW_BINDINGS` への `unity` の追加。飛ばしていた 4 件 (`avatar-follow`・`dress-up-follow`・
    `lip-sync-follow`・`part-switch-follow`) が通る (unity 57 / 57)。実行器は作ったばかりの `Puppet` で呼ぶので `ResetPhysics` は要らない
    (C ABI と同じ)。`UgoePuppet` は読み込みの直後に 1 度評価するので、Godot と同じく受け手を始める前に `ResetPhysics()` を呼ぶことを
    README の「受け手」に書いた。
  - `tests/ReceiverTests.cs` に有限でない値 (値の NaN・音量の NaN・強さと `dt` の無限大・重みの無限大) を断り、失敗で何も変えないことと、
    `values` が null の `ArgumentNullException` を足した (C ABI に揃っている)。
  - `UgoeResult.LimitExceeded` (10、T0152) を足し、`UgoeException.Fields` に読み込みの上限の補足の値 (`limit`・`chunk`・`max`) を足した。
    `tests/ResultTests.cs` が `ugoe.h` の `UGOE_OK`・`UGOE_ERROR_*` と `UgoeResult` の名前と値の一致を確かめる (今後の追加の漏れを検出する)。
  - `bindings/unity/README.md` と `tests/conformance/README.md` を更新した。
- 確かめた範囲: `just unity-check` (dotnet のテスト 88 件 (T0453 を取り込んだ後)・`netstandard2.1` と `net48` の型検査・release.mjs のテスト)、
  `just conformance` (c・wasm・wasm-webgpu・avatar・unity・godot がすべて 57 / 57)、`just ci`。**実際の Unity では動かしていない**
  (`UgoePuppet.Follow` と `ResetPhysics` を呼ぶ順の README の記述は、Godot の知見と LateUpdate のコードを読んだ推論)。
- やらなかったこと: `ugoe_puppet_create_from_bundle_with_limits`・`ugoe_module_create_with_limits`・`ugoe_read_limits_default` の C# への
  宣言と包み (`Native.cs` は「ugoe.h のすべての関数を宣言する」と README にあるが、今はこの 3 つが無い)。
- 起票の候補: Unity の C# に読み込みの上限 (`UgoeReadLimits`・`*_with_limits`) を出す (T0152 の C ABI に揃える。`Puppet.FromBundle`・
  `Module.FromBytes` に上限を渡す多重定義と dotnet のテスト)。`UgoeModel` の取り込みで上限を指定できるかも合わせて検討。

