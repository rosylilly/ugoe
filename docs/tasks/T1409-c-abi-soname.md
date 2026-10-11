---
id: T1409
title: "公開: C ABI の Linux の共有ライブラリに ABI の版の soname (libugoe_ffi.so.<ABI の版>) を付ける"
status: in-progress
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

- [ ] soname の版を ABI の版から作る (手で書いた数を 2 か所に持たない)。`ABI_VERSION` とヘッダの `UGOE_ABI_VERSION` と soname が一致することを検査する
      (`scripts/ffi-dist.mjs` の verify か `just ffi-check`)
- [ ] `scripts/ffi-dist.mjs` の pack が Linux で soname 付きの共有ライブラリとリンクを詰め、verify が `readelf -d` などで soname を確かめ、
      アーカイブの中のヘッダとライブラリだけで `-lugoe_ffi` でリンクして見本を動かす (今の `-l:libugoe_ffi.so` の回避を外す)。
      tar の中のシンボリックリンクが展開で保たれることを確かめる
- [ ] soname を付ける場所を決めた: C ABI の配布物のビルドだけか、`ugoe-ffi` のすべての Linux のビルド (build.rs の `cargo:rustc-cdylib-link-arg`) か。
      Unity のプラグイン (unity.yml の Linux と Android の `.so`。Unity と Android はファイル名で読む) と Godot の GDExtension が読む `.so` に
      影響しないこと (か、影響しても読めること) を確かめ、理由をメモに書いた
- [ ] macOS の install name の扱いを決めて、そろえるなら同じ検査を足した
- [ ] docs/release.md#c-abi-の配布物 (中身の表・リンクの例)、`crates/ugoe-ffi/archive/README.md` (リンクの例。rpath)、docs/portability.md の C ABI の版の説明を直した
- [ ] Linux 以外 (macOS・aarch64 のランナー) で確かめていないことをメモに書いた
- [ ] `just ci`・`just ffi-check`・`just ffi-dist`・`just ffi-dist-check` が通る

## 参考

- T1408 のメモ (起票の候補)、docs/release.md#c-abi-の配布物、`scripts/ffi-dist.mjs`・`scripts/ffi.mjs`、`crates/ugoe-ffi/src/lib.rs` (`ABI_VERSION`)、
  `.github/workflows/ffi-release.yml`・`unity.yml` (macOS の `install_name_tool`)

## メモ

- T0089 で起票した (元の候補: T1408 (Linux の共有ライブラリに soname を付けるか)。利用者の決定 2026-10-11)。
- Release のマイルストーン (docs/roadmap.md の Release)。T1408 と同じ。
- **並行の注意:** T1410 (unity.yml の native を ffi-dist.mjs に寄せる) は `scripts/ffi-dist.mjs` と同じビルドを触るので、このタスクの後に行う (depends_on)。
  T0093 (チェックサムと成果物の証明) と docs/release.md#c-abi-の配布物・`ffi-release.yml` を触ることがある。
