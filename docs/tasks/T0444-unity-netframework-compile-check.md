---
id: T0444
title: Unity: スタブでの型検査を .NET Framework の API 互換レベルでも行う
status: done
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

- [x] `tests/UnityCompile` を .NET Framework 4.x (Unity の .NET Framework の互換レベルに近いもの) でもビルドする
      (参照アセンブリの NuGet のパッケージを足すなら、ライセンスを確かめて `bindings/unity/README.md` の表に書く)
- [x] `just unity-check` と CI の `unity.yml` の `check` で両方を検査する
- [x] `just ci` が通る

## 参考

- `bindings/unity/tests/UnityCompile/Ugoe.UnityCompile.csproj`、`bindings/unity/README.md`

## メモ

- `tests/UnityCompile/Ugoe.UnityCompile.csproj` を `TargetFrameworks` で `netstandard2.1;net48` にした。net48 の参照アセンブリは
  NuGet の `Microsoft.NETFramework.ReferenceAssemblies` 1.0.3 (と推移的な `.net48`。どちらも licenseUrl は Microsoft/dotnet の LICENSE = MIT)
  から取る (net48 のときだけの `PackageReference`、`PrivateAssets="all"`)。Linux でも Developer Pack なしでビルドできる。README の NuGet の表に足した。
- `build.mjs --check` は csproj を `dotnet build` するだけなので、両方の対象を自動でビルドする (`just unity-check`・`unity.yml` の `check` はそのまま両方を検査する)。
  コメント (`build.mjs`・`justfile`・`unity.yml`) と README (構成の表・ビルドと検査・制限) に net48 のことを書いた。
- 確かめたこと: `Runtime/Core` に `System.Math.Clamp` を使う一時ファイルを置くと、net48 の側で `CS0117` になって `just unity-check` の型検査が失敗する
  (netstandard2.1 だけでは通っていた)。今のパッケージは両方で警告なしに通る。`just unity-check` (dotnet test 44 件・両方の型検査・release.mjs のテスト) と `just ci` は通った。
- 範囲 (`crates: [bindings/unity, docs]`) の外: `justfile` と `.github/workflows/unity.yml` のコメントだけを直した (動作は変えていない)。
- 確かめていないこと: net48 の参照アセンブリは Unity の「.NET Framework」の実際の参照 (Mono のクラスライブラリ) と同じではない。Unity 2021.2 以降の
  「.NET Framework」は .NET Standard 2.1 の API も足していると思われ、net48 は厳しめの近似 (Unity では通る API を断ることはあり得るが、通らない API を見逃すことは減る)。
  実際の Unity での確かめは T0443 で。
- 起票の候補: なし。

