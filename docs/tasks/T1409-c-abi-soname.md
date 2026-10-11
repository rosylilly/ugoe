---
id: T1409
title: "公開: C ABI の Linux の共有ライブラリに ABI の版の soname (libugoe_ffi.so.<ABI の版>) を付ける"
status: done
milestone: Release
crates: [ugoe-ffi, scripts, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T1409
---

# T1409: C ABI の soname

## 目的

C ABI の配布物 (T1408。docs/release.md#c-abi-の配布物) の Linux の共有ライブラリには soname が無く、利用者は `-l:libugoe_ffi.so` でリンクする。
ABI を壊す変更 (`UGOE_ABI_VERSION` を上げる) をした版を同じ場所に入れると、古い ABI でビルドしたプログラムが黙って新しいライブラリを読む。
soname に ABI の版を入れ、動的リンカが食い違いを見分けられるようにする。

## 決まっていること (利用者、2026-10-11)

- soname は ABI の版で付ける: `libugoe_ffi.so.<ABI の版>` (今は `ugoe_ffi::ABI_VERSION` = 3 なので `libugoe_ffi.so.3`)。
  配布物の `lib/` には実体の `libugoe_ffi.so.<ABI の版>` と、そこへのシンボリックリンクの `libugoe_ffi.so` (`-lugoe_ffi` でのリンク用) を置く。
- macOS の install name (今は `@rpath/libugoe_ffi.dylib`) も同じ考えでそろえるか (`@rpath/libugoe_ffi.<ABI の版>.dylib` と、そこへのリンク) を
  このタスクで決め、理由をメモと docs/release.md に書く。Windows の DLL には該当するものが無いので変えない。

## 受け入れ条件

- [x] soname の版を ABI の版から作る (手で書いた数を 2 か所に持たない)。`ABI_VERSION` とヘッダの `UGOE_ABI_VERSION` と soname が一致することを検査する
      (`scripts/ffi-dist.mjs` の verify か `just ffi-check`)
- [x] `scripts/ffi-dist.mjs` の pack が Linux で soname 付きの共有ライブラリとリンクを詰め、verify が `readelf -d` などで soname を確かめ、
      アーカイブの中のヘッダとライブラリだけで `-lugoe_ffi` でリンクして見本を動かす (今の `-l:libugoe_ffi.so` の回避を外す)。
      tar の中のシンボリックリンクが展開で保たれることを確かめる
- [x] soname を付ける場所を決めた: C ABI の配布物のビルドだけか、`ugoe-ffi` のすべての Linux のビルド (build.rs の `cargo:rustc-cdylib-link-arg`) か。
      Unity のプラグイン (unity.yml の Linux と Android の `.so`。Unity と Android はファイル名で読む) と Godot の GDExtension が読む `.so` に
      影響しないこと (か、影響しても読めること) を確かめ、理由をメモに書いた
- [x] macOS の install name の扱いを決めて、そろえるなら同じ検査を足した
- [x] docs/release.md#c-abi-の配布物 (中身の表・リンクの例)、`crates/ugoe-ffi/archive/README.md` (リンクの例。rpath)、docs/portability.md の C ABI の版の説明を直した
- [x] Linux 以外 (macOS・aarch64 のランナー) で確かめていないことをメモに書いた
- [x] `just ci`・`just ffi-check`・`just ffi-dist`・`just ffi-dist-check` が通る

## 参考

- T1408 のメモ (起票の候補)、docs/release.md#c-abi-の配布物、`scripts/ffi-dist.mjs`・`scripts/ffi.mjs`、`crates/ugoe-ffi/src/lib.rs` (`ABI_VERSION`)、
  `.github/workflows/ffi-release.yml`・`unity.yml` (macOS の `install_name_tool`)

## メモ

- T0089 で起票した (元の候補: T1408 (Linux の共有ライブラリに soname を付けるか)。利用者の決定 2026-10-11)。
- Release のマイルストーン (docs/roadmap.md の Release)。T1408 と同じ。
- **並行の注意:** T1410 (unity.yml の native を ffi-dist.mjs に寄せる) は `scripts/ffi-dist.mjs` と同じビルドを触るので、このタスクの後に行う (depends_on)。
  T0093 (チェックサムと成果物の証明) と docs/release.md#c-abi-の配布物・`ffi-release.yml` を触ることがある。

### 作業の記録 (claude-worker-T1409)

- **やったこと:**
  - `scripts/ffi-dist.mjs`: ABI の版を `crates/ugoe-ffi/src/lib.rs` の `ABI_VERSION` から読み、ヘッダの `UGOE_ABI_VERSION` と食い違えば `pack` が止まる。
    Linux は `cargo rustc --locked --release -p ugoe-ffi --lib -- -C link-arg=-Wl,-soname,libugoe_ffi.so.<ABI の版>` で作り、
    `lib/` に実体 `libugoe_ffi.so.<ABI の版>` とシンボリックリンク `libugoe_ffi.so` を置く。
    `verify` はアーカイブの中のヘッダの版で、シンボリックリンクが展開で保たれたか (lstat と readlink)・`readelf -d` の SONAME・
    `-lugoe_ffi` でリンクした見本の NEEDED が版の付いた名前かを確かめてから見本を動かす (`-l:libugoe_ffi.so` の回避は外した)。
  - 単体テスト (`scripts/ffi-dist.test.mjs`): 版の読み取り、lib.rs とコミットしたヘッダの版の一致 (`just script-check` で `just ci` に入る)、
    soname・インストール名、`readelf -d` の出力の読み取り、ファイルの一覧。
  - 文書: docs/release.md#c-abi-の配布物 (表・名前の規則と理由・付ける場所・検査)、`crates/ugoe-ffi/archive/README.md` (表とリンクの例を
    `-L lib -lugoe_ffi` に)、`crates/ugoe-ffi/README.md#abi-の版`、docs/portability.md の C ABI の版。
- **付ける場所:** 配布物のビルド (`ffi-dist.mjs pack`) だけ。build.rs で `ugoe-ffi` のすべての Linux のビルドに付けることはしない。
  Unity のプラグイン (unity.yml の Linux・Android の `libugoe_ffi.so`) はファイル名を指定して読み込み、ほかのライブラリがリンクすることも
  無いので soname は働かない。一方で Android の動的リンカは soname で読み込み済みのライブラリを区別するので、ファイル名と違う soname は
  食い違いのもとになりうる。テスト・`just ffi-check`・`just conformance` は静的ライブラリを使い、Godot の GDExtension は `ugoe-ffi` を使わない。
  よって Unity・Godot が読む `.so` は変わらない (`just unity-check`・`just conformance` が通ることを確かめた)。
- **macOS:** そろえた。実体 `libugoe_ffi.<ABI の版>.dylib`、インストール名 `@rpath/libugoe_ffi.<ABI の版>.dylib`、シンボリックリンク
  `libugoe_ffi.dylib`。dyld もインストール名で探すので Linux と同じ守りになり、`libfoo.<版>.dylib` は macOS の慣例の形で、リンクの書き方
  (`-L lib -lugoe_ffi`) も Linux と同じになる。`verify` は `otool -D` でインストール名を、`otool -L` で見本が記録した名前を確かめる。
  unity.yml の dylib (`@rpath/libugoe_ffi.dylib`) は変えていない (Unity はファイル名で読む)。Windows は変えない。
- **確かめたこと (Linux x86_64):** `just ffi-dist`・`just ffi-dist-check` (soname `libugoe_ffi.so.3`、tar の中の `libugoe_ffi.so -> libugoe_ffi.so.3`)・
  `just ffi-check`・`just conformance`・`just unity-check`・`just refs`。アーカイブの README のリンクの例 (Linux の共有と静的) を展開したディレクトリで
  そのまま流して見本が動くこと、実行ファイルの NEEDED が `libugoe_ffi.so.3`・RUNPATH が `$ORIGIN/lib` であること。シンボリックリンクを実体に
  置き換えた (`tar -h` で作り直した) アーカイブを `verify` が拒むこと。
- **確かめていないこと:** macOS (lipo の後の `install_name_tool -id`・シンボリックリンク・`otool` の検査・`-L lib -lugoe_ffi` の例) と
  linux-aarch64 のランナー (`readelf` は ubuntu のランナーの binutils にある前提) では動かしていない。ffi-release.yml は変えていない
  (`pack`・`verify` を呼ぶだけで足りる)。Actions は止まっているので CI では流していない。
- **起票の候補:**
  - T1410 (unity.yml の native を ffi-dist.mjs に寄せる) で寄せるときは、Unity 向けのビルドには soname・版の付いた名前を付けない
    (`pack` の build と分ける) ことに注意する。
