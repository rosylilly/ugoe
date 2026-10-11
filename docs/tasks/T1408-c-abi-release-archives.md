---
id: T1408
title: "公開: C ABI の配布物 (ugoe-ffi の共有ライブラリ・静的ライブラリとヘッダ) を OS ごとにまとめ、タグで GitHub Release に添付する"
status: in-progress
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

- [ ] 配布物の中身を決めて docs/release.md に書いた: OS とアーキテクチャ (Linux x86_64・aarch64、Windows x86_64、macOS の universal か
      arm64・x86_64 の別々)、共有ライブラリ (`.so`・`.dll` と `.dll.lib`・`.dylib`)・静的ライブラリ (`.a`・`.lib`)・`include/ugoe.h`・LICENSE・
      README (リンクの例。crates/ugoe-ffi/README.md と C の見本から)、アーカイブの名前と形 (`.tar.gz`・`.zip`)。std の無い静的ライブラリ (T0491) を
      入れるかと理由
- [ ] アーカイブを作るスクリプト (と just のレシピ)。手元の OS の分を作れる。ヘッダが `just ffi-check` の生成物と同じか確かめてから詰める
- [ ] ワークフロー: OS ごとのランナーで作り、`v*` のタグのときに下書きの GitHub Release に添付する (T0084・T0421 と同じ下書きにそろえる)。
      `unity.yml` のライブラリのビルドと重なるところは共有するか、重ねる理由を書く
- [ ] 作ったアーカイブを、C の見本 (crates/ugoe-ffi/examples) をそのアーカイブのヘッダとライブラリだけでビルドして動かす検査をワークフローに入れた (少なくとも Linux)
- [ ] docs/release.md の配布物の表に行を足し、docs/portability.md#配布 から手順へ辿れるようにした
- [ ] `just ci` が通る

## 参考

- docs/portability.md#配布、docs/release.md、crates/ugoe-ffi/README.md、`.github/workflows/unity.yml` (OS ごとの `ugoe-ffi` のビルド)、
  `scripts/ffi.mjs`、T0401・T0491・T0421

## メモ

- T0083 で起票した (元の候補: オーケストレーターの点検 (C ABI の配布物: ugoe-ffi の共有ライブラリ・静的ライブラリとヘッダを OS ごとにまとめ、
  タグで GitHub Release に添付するワークフローを作り、docs/release.md の表に行を足す。portability.md の配布の表は配布物に挙げているが
  release.md の表とワークフローに無い))。
- Release のマイルストーン (docs/roadmap.md の Release)。公開の時期が決まるまで着手しない。
- 第三者ライセンスの表記は T1407 で入れる (こちらが先に入れば、T1407 が配布物に足す)。
