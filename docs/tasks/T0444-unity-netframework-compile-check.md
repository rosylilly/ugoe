---
id: T0444
title: Unity: スタブでの型検査を .NET Framework の API 互換レベルでも行う
status: in-progress
milestone: M9
crates: [bindings/unity, docs]
depends_on: [T0426]
assignee: claude-worker-T0444
---

# T0444: Unity: .NET Framework の API 互換レベルでの型検査

## 目的

Unity 2022.3 は API 互換レベルに .NET Standard 2.1 と .NET Framework を選べる。`tests/UnityCompile` は `netstandard2.1` だけで型検査するので、
.NET Framework に無い API (T0426 で見つけた `Math.Clamp` など) を使っても検査を通ってしまう。.NET Framework 4.x の参照アセンブリでも
型検査し、パッケージがどちらの互換レベルでもコンパイルできることを保つ。

## 受け入れ条件

- [ ] `tests/UnityCompile` を .NET Framework 4.x (Unity の .NET Framework の互換レベルに近いもの) でもビルドする
      (参照アセンブリの NuGet のパッケージを足すなら、ライセンスを確かめて `bindings/unity/README.md` の表に書く)
- [ ] `just unity-check` と CI の `unity.yml` の `check` で両方を検査する
- [ ] `just ci` が通る

## 参考

- `bindings/unity/tests/UnityCompile/Ugoe.UnityCompile.csproj`、`bindings/unity/README.md`

## メモ
