---
id: T1410
title: "公開: unity.yml の native ジョブの ugoe-ffi のビルドを scripts/ffi-dist.mjs に寄せる"
status: done
milestone: Release
crates: [.github/workflows, scripts, bindings/unity, docs]
depends_on: [T1409]
assignee: claude-worker-T1410
---

# T1410: unity.yml の native を ffi-dist.mjs に寄せる

## 目的

`ugoe-ffi` の共有ライブラリを OS ごとに作る手順が 2 か所にある: `unity.yml` の native ジョブ (Unity のプラグインの並び。Android・iOS を含む) と、
`ffi-release.yml` が呼ぶ `scripts/ffi-dist.mjs` (C ABI の配布物)。macOS の universal (`lipo`) と install name (`install_name_tool`) のように
同じことを 2 度書いていて、片方だけ直す (T1409 の soname・install name など) と食い違う。ビルドの手順を `scripts/ffi-dist.mjs` に寄せる。

T1408 はワークフローを分けた (動く条件と成果物を独立に保つため。docs/release.md#ワークフロー)。**ワークフローは分けたまま**、
ビルドの手順 (ターゲットごとのビルド・macOS の universal と install name・できたファイルの名前) だけを共有する。

## 受け入れ条件

- [x] `scripts/ffi-dist.mjs` に、共有ライブラリだけを指定のターゲット (Android・iOS を含む unity.yml の並び) で作って指定の場所に置く入口を足し
      (例: `node scripts/ffi-dist.mjs build-shared --target <triple> --out <dir>`)、`pack` も同じ関数を使う。単体テスト (scripts/ffi-dist.test.mjs) を足した
- [x] `unity.yml` の native ジョブがその入口を使い、ジョブの中に残るのは環境の用意 (Android の NDK・ツールチェーン) と成果物のアップロードだけになった。
      できる Unity のプラグインの並びとファイル名は今と同じ (`bindings/unity` の release の検査 (`just unity-check`) が通る)
- [x] macOS の universal と install name を 1 か所にまとめた (unity.yml の `install_name_tool` の行が無くなる)
- [x] `ffi-release.yml` と docs/release.md#ワークフロー の「unity.yml とは共有しない」の説明を、「ワークフローは分け、ビルドは ffi-dist.mjs を共有する」に直した。
      bindings/unity/README.md のビルドの説明も
- [x] GitHub の上で流していないこと (Android・iOS・Windows・macOS のランナー) をメモに書いた。手元では Linux と、入れられるターゲット (cargo-xwin などで Windows)
      で確かめる
- [x] `just ci`・`just ffi-dist-check`・`just unity-check` が通る (actionlint を流せるなら流す)

## 参考

- T1408 のメモ (起票の候補)、docs/release.md#c-abi-の配布物・#ワークフロー、`.github/workflows/unity.yml` (native ジョブ)・`ffi-release.yml`、`scripts/ffi-dist.mjs`・`scripts/ffi.mjs`

## メモ

- T0089 で起票した (元の候補: T1408 (unity.yml の native ジョブを `scripts/ffi-dist.mjs` のビルドに寄せるか)。利用者の決定 2026-10-11: そのまま起票)。
- Release のマイルストーン (docs/roadmap.md の Release)。
- T1409 (soname) と同じ `scripts/ffi-dist.mjs` のビルドを触り、soname を付ける場所 (C ABI の配布物だけか、すべての Linux のビルドか) の決定が
  Unity の `.so` に関わるので、T1409 の後に行う。T0421 (UPM の tarball を Release に添付) と T0091 (CI の見直し。起動の条件) も `unity.yml` を触る。並べて進めるときは片方が入ってから追従する。

- 注意 (T1409 から、オーケストレーター): unity.yml の native を ffi-dist.mjs に寄せるとき、soname と版付きのファイル名は Unity のビルドに入れない (Unity は .so をファイル名で読み、Android のリンカは soname で見分けるため)。pack の配布用のビルドと分ける。

- やったこと (T1410):
  - `scripts/ffi-dist.mjs` に `BUILDS` (C ABI の配布物と Unity のプラグインの並びを合わせたプラットフォームの表。cargo のターゲット・
    cargo-ndk の ABI・プラグインのファイル名) と、`cargoCommands` (ビルドのコマンド)・`place` (写すか lipo でまとめ、インストール名を付ける。
    macOS の universal とインストール名はここだけ) を足し、`pack` と新しい入口 `build-plugin --platform <名前> --out <dir>` が使う。
    入口の名前は受け入れ条件の例 (`build-shared --target <triple>`) から変えた: iOS は静的ライブラリで、macOS は 2 つのターゲットを
    まとめるので、ターゲットの三つ組でなく unity.yml の matrix の名前 (= build.mjs の PLUGINS の from の先頭) で指す。
  - すべてのビルドに `--target` を付けるようにした (pack も。成果物は target/<triple>/release)。Windows の外で Windows (MSVC) 向けに
    作るときは cargo-xwin を使う (手元での確かめ用)。
  - soname は `pack` の Linux だけに付き、`build-plugin` には付かない (T1409 の注意)。macOS のプラグインのインストール名は
    `@rpath/libugoe_ffi.dylib` のまま。
  - unity.yml の native は、rustup のターゲット・Node・cargo-ndk と NDK の用意、`build-plugin`、アップロードだけになった
    (`install_name_tool`・`lipo`・`cp` の行は無くなった)。
  - 単体テスト: `BUILDS` の plugin が build.mjs の PLUGINS の from と一致すること、soname が配布物のときだけ付くこと、Android の
    cargo-ndk の引数、cargo-xwin の切り替え、成果物の場所。scripts/dist-notices.test.mjs は unity.yml の `rust_target` の複数の値
    (macOS) と `BUILDS` のターゲットも見る。
  - 文書: ffi-release.yml の冒頭、docs/release.md#ワークフロー と #手元で作る・soname の節、bindings/unity/README.md のビルドの表。
- 手元で確かめたこと (Linux): `build-plugin` で linux-x86_64 (soname 無し。`pack` の後に作っても付かないこと) と、cargo-xwin で
  windows-x86_64 の DLL ができること。`just ffi-dist` と `just ffi-dist-check`・`just unity-check`・`just script-check`・
  actionlint (shellcheck 込み)・`just ci` (rebase の前に通した。最後の rebase で入ったのは T0093・T0094 のワークフローと文書で、
  その後に actionlint・`just script-check`・`just refs`・`just ffi-dist-check`・`just unity-check` を流し直して通った)。
- 確かめていないこと: GitHub の上で unity.yml・ffi-release.yml を流していない (Windows・macOS・Android・iOS のランナー)。
  特に、macOS の `lipo` と `install_name_tool`、cargo-ndk (`ANDROID_NDK_HOME` を `$GITHUB_ENV` で渡す形)、iOS、Windows のランナーでの
  `--target x86_64-pc-windows-msvc` と、ffi-release.yml の pack が `--target` 付きになったこと (Linux 以外)。次のリリースの前の
  release-checks.yml の手での起動で確かめる。
- 起票の候補: 無し。
