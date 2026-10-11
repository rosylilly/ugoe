---
id: T1410
title: "公開: unity.yml の native ジョブの ugoe-ffi のビルドを scripts/ffi-dist.mjs に寄せる"
status: todo
milestone: Release
crates: [.github/workflows, scripts, bindings/unity, docs]
depends_on: [T1409]
assignee:
---

# T1410: unity.yml の native を ffi-dist.mjs に寄せる

## 目的

`ugoe-ffi` の共有ライブラリを OS ごとに作る手順が 2 か所にある: `unity.yml` の native ジョブ (Unity のプラグインの並び。Android・iOS を含む) と、
`ffi-release.yml` が呼ぶ `scripts/ffi-dist.mjs` (C ABI の配布物)。macOS の universal (`lipo`) と install name (`install_name_tool`) のように
同じことを 2 度書いていて、片方だけ直す (T1409 の soname・install name など) と食い違う。ビルドの手順を `scripts/ffi-dist.mjs` に寄せる。

T1408 はワークフローを分けた (動く条件と成果物を独立に保つため。docs/release.md#ワークフロー)。**ワークフローは分けたまま**、
ビルドの手順 (ターゲットごとのビルド・macOS の universal と install name・できたファイルの名前) だけを共有する。

## 受け入れ条件

- [ ] `scripts/ffi-dist.mjs` に、共有ライブラリだけを指定のターゲット (Android・iOS を含む unity.yml の並び) で作って指定の場所に置く入口を足し
      (例: `node scripts/ffi-dist.mjs build-shared --target <triple> --out <dir>`)、`pack` も同じ関数を使う。単体テスト (scripts/ffi-dist.test.mjs) を足した
- [ ] `unity.yml` の native ジョブがその入口を使い、ジョブの中に残るのは環境の用意 (Android の NDK・ツールチェーン) と成果物のアップロードだけになった。
      できる Unity のプラグインの並びとファイル名は今と同じ (`bindings/unity` の release の検査 (`just unity-check`) が通る)
- [ ] macOS の universal と install name を 1 か所にまとめた (unity.yml の `install_name_tool` の行が無くなる)
- [ ] `ffi-release.yml` と docs/release.md#ワークフロー の「unity.yml とは共有しない」の説明を、「ワークフローは分け、ビルドは ffi-dist.mjs を共有する」に直した。
      bindings/unity/README.md のビルドの説明も
- [ ] GitHub の上で流していないこと (Android・iOS・Windows・macOS のランナー) をメモに書いた。手元では Linux と、入れられるターゲット (cargo-xwin などで Windows)
      で確かめる
- [ ] `just ci`・`just ffi-dist-check`・`just unity-check` が通る (actionlint を流せるなら流す)

## 参考

- T1408 のメモ (起票の候補)、docs/release.md#c-abi-の配布物・#ワークフロー、`.github/workflows/unity.yml` (native ジョブ)・`ffi-release.yml`、`scripts/ffi-dist.mjs`・`scripts/ffi.mjs`

## メモ

- T0089 で起票した (元の候補: T1408 (unity.yml の native ジョブを `scripts/ffi-dist.mjs` のビルドに寄せるか)。利用者の決定 2026-10-11: そのまま起票)。
- Release のマイルストーン (docs/roadmap.md の Release)。
- T1409 (soname) と同じ `scripts/ffi-dist.mjs` のビルドを触り、soname を付ける場所 (C ABI の配布物だけか、すべての Linux のビルドか) の決定が
  Unity の `.so` に関わるので、T1409 の後に行う。T0421 (UPM の tarball を Release に添付) と T0091 (CI の見直し。起動の条件) も `unity.yml` を触る。並べて進めるときは片方が入ってから追従する。
