---
id: T0455
title: "Unity: 受け手の口 (follow) を足し、適合テストの *-follow の skip をなくす"
status: todo
milestone: M9
crates: [bindings/unity, tests/conformance, docs]
depends_on: []
assignee:
---

# T0455: Unity: 受け手の口 (follow) を足し、適合テストの *-follow の skip をなくす

## 目的

C ABI・wasm・Godot には受け手の口 (`follow`) があり、適合テストの `*-follow` のケースで参照と比べる (`tests/conformance/src/main.rs` の `FOLLOW_BINDINGS`)。
Unity だけが口を持たず、`*-follow` を skip する (T0449 の候補)。C# から C ABI の `follow` を呼ぶ口を足し、比べ合わせに入れる。

## 受け入れ条件

- [ ] `Runtime/Core` に `follow` の口があり、dotnet のテストがある (値の長さの不一致・NaN の扱いは C ABI に揃える)
- [ ] `FOLLOW_BINDINGS` に Unity を足し、`*-follow` のケースが skip されない
- [ ] `bindings/unity/README.md` と tests/conformance/README.md を更新した
- [ ] `just unity-check`・`just conformance`・`just ci` が通る

## 参考

- T0449 のメモ (起票の候補)
- tests/conformance/README.md (`FOLLOW_BINDINGS`)

## メモ

- T0019 で起票した (元の候補: T0449 の Unity の受け手)。
