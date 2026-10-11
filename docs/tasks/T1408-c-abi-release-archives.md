---
id: T1408
title: "公開: C ABI の配布物 (ugoe-ffi の共有ライブラリ・静的ライブラリとヘッダ) を OS ごとにまとめ、タグで GitHub Release に添付する"
status: done
milestone: Release
crates: [ugoe-ffi, .github/workflows, scripts, justfile, docs]
depends_on: []
assignee: claude-worker-T1408
---

# T1408: C ABI の配布物

## 目的

docs/portability.md#配布 は「C / その他: `ugoe-ffi` の共有ライブラリとヘッダ」を配布物に挙げているが、docs/release.md の配布物の表にも
ワークフローにも無い (共有ライブラリを作るのは Unity の tarball のための `unity.yml` だけで、ヘッダや静的ライブラリは配っていない)。
C・C++・ほかの言語から使う人のために、OS ごとの配布物を作ってタグで GitHub Release に添付できるようにする。

公開の時期は v1.0.0 の完全リリースと判断するまで決めない (docs/roadmap.md の Release)。タグを打つ・Release を公開するのは人の手で、
このタスクは配布物を作って下書きの Release に添付するところまで。

## 受け入れ条件

- [x] 配布物の中身を決めて docs/release.md に書いた: OS とアーキテクチャ (Linux x86_64・aarch64、Windows x86_64、macOS の universal か
      arm64・x86_64 の別々)、共有ライブラリ (`.so`・`.dll` と `.dll.lib`・`.dylib`)・静的ライブラリ (`.a`・`.lib`)・`include/ugoe.h`・LICENSE・
      README (リンクの例。crates/ugoe-ffi/README.md と C の見本から)、アーカイブの名前と形 (`.tar.gz`・`.zip`)。std の無い静的ライブラリ (T0491) を
      入れるかと理由
- [x] アーカイブを作るスクリプト (と just のレシピ)。手元の OS の分を作れる。ヘッダが `just ffi-check` の生成物と同じか確かめてから詰める
- [x] ワークフロー: OS ごとのランナーで作り、`v*` のタグのときに下書きの GitHub Release に添付する (T0084・T0421 と同じ下書きにそろえる)。
      `unity.yml` のライブラリのビルドと重なるところは共有するか、重ねる理由を書く
- [x] 作ったアーカイブを、C の見本 (crates/ugoe-ffi/examples) をそのアーカイブのヘッダとライブラリだけでビルドして動かす検査をワークフローに入れた (少なくとも Linux)
- [x] docs/release.md の配布物の表に行を足し、docs/portability.md#配布 から手順へ辿れるようにした
- [x] `just ci` が通る

## 参考

- docs/portability.md#配布、docs/release.md、crates/ugoe-ffi/README.md、`.github/workflows/unity.yml` (OS ごとの `ugoe-ffi` のビルド)、
  `scripts/ffi.mjs`、T0401・T0491・T0421

## メモ

- T0083 で起票した (元の候補: オーケストレーターの点検 (C ABI の配布物: ugoe-ffi の共有ライブラリ・静的ライブラリとヘッダを OS ごとにまとめ、
  タグで GitHub Release に添付するワークフローを作り、docs/release.md の表に行を足す。portability.md の配布の表は配布物に挙げているが
  release.md の表とワークフローに無い))。
- Release のマイルストーン (docs/roadmap.md の Release)。公開の時期が決まるまで着手しない。
- 第三者ライセンスの表記は T1407 で入れる (こちらが先に入れば、T1407 が配布物に足す)。
- T1408 でやったこと:
  - 中身 (docs/release.md#c-abi-の配布物): Linux x86_64・aarch64 (glibc)、Windows x86_64 (MSVC)、macOS universal (lipo。unity.yml と同じ形)。
    `ugoe-ffi-<版>-<プラットフォーム>/` の下に `include/ugoe.h`・`lib/` (共有と静的。Windows は `.dll`・`.dll.lib`・`.lib`)・`examples/sample.c`・
    `LICENSE`・`README.md` (元は crates/ugoe-ffi/archive/README.md。リンクの例)。Windows は `.zip`、ほかは `.tar.gz`、隣に `.sha256`。
    std の無い静的ライブラリはターゲットごとに作るもので、ホストの OS 向けは使い道が無いので入れない (README から作り方へ案内)。
    `dist/` は .gitignore にあるので、README の元はクレートの `archive/` に置いた。
  - `scripts/ffi-dist.mjs` (`just ffi-dist` / `just ffi-dist-check`): pack はヘッダを `just ffi-check` と同じく cbindgen の生成物と比べてから詰める
    (`--tag` でタグと版も確かめる)。verify はアーカイブを展開し、一覧と SHA-256 を確かめ、中のヘッダ・ライブラリ・見本だけで静的と共有の
    両方をビルドして見本のモデルで動かす。scripts/ffi.mjs は見本のコンパイルと実行・静的リンクの OS のライブラリを export し、
    `ffi-check` と共有した (振る舞いは同じ)。単体テストは scripts/ffi-dist.test.mjs (`just script-check`)。
  - `.github/workflows/ffi-release.yml`: 4 つのランナーで pack と verify、タグのときは `release` ジョブが T0084 の `scripts/github-release.mjs` で
    タグの下書きの Release に添付する (editor-build.yml の release と同じ形)。unity.yml とは共有しない (理由はワークフローの先頭と docs/release.md#ワークフロー)。
  - 添付の共通のスクリプトは、先に main に入った T0084 の `scripts/github-release.mjs` にそろえた (T1408 で作っていた release-draft.mjs は消した)。
    release-draft.mjs にだけあった「選んだ下書きの id に添付する」を github-release.mjs に足した (`gh release upload` はタグで引くので、添付の始まった
    下書きが重なって残るとどれに添付するか決まらないため)。使い方 (`upload <タグ> <ファイル...>`) は変えず、editor-build.yml はそのまま。
    docs/release.md#github-release-の下書き の表に C ABI の行と、id で添付することを足した。
- 確かめたこと: Linux x86_64 で `just ffi-dist` と `just ffi-dist-check` (静的・共有とも見本の 3 つのモデルで動く)、中身の一覧
  (LICENSE・README.md・examples/sample.c・include/ugoe.h・lib/libugoe_ffi.a・lib/libugoe_ffi.so。tar.gz で約 17MB)、アーカイブの README の
  Linux のリンクの例 (rpath `$ORIGIN/lib`) を手で、`just ffi-check`・`just refs`・`just cross-check`、actionlint (shellcheck なし)、
  `github-release.mjs` を偽の gh で (新規の下書きが重なったときに古い方の id へ添付する)。Windows のファイル名 (`ugoe_ffi.dll`・`ugoe_ffi.dll.lib`・`ugoe_ffi.lib`) は
  cargo-xwin でビルドして確かめた。
- 確かめていないこと: ワークフローそのもの (push・workflow_dispatch をしていない)。Linux aarch64 (`ubuntu-24.04-arm` のランナーと
  taiki-e/install-action の cbindgen)、Windows (System32 の bsdtar での zip の作成と展開、clang での `.dll.lib` へのリンクと実行、静的ライブラリ)、
  macOS (universal の `lipo`・`install_name_tool` と、`.a` の universal、rpath での共有ライブラリの実行)、本物の GitHub API への下書きの作成と添付。
  アーカイブの README の Windows・macOS のリンクの例も手で試していない。
- 起票の候補:
  - 初めてタグで動かしたとき (または workflow_dispatch で) ffi-release.yml の 4 つのランナーが通るか見て、Windows・macOS で落ちたら直す。
  - Linux の共有ライブラリに soname (`-Wl,-soname,libugoe_ffi.so.<ABI の版>`) を付けるか (今は無く、`-l:libugoe_ffi.so` でリンクする)。
  - unity.yml の native ジョブを `scripts/ffi-dist.mjs` のビルドに寄せるか (今は重ねている)。

