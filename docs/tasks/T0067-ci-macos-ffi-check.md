---
id: T0067
title: "CI: macOS で just ffi-check (std の無い静的ライブラリの形を含む) を流す"
status: done
milestone: M11
crates: [.github/workflows, scripts, docs]
depends_on: []
assignee: claude-worker-T0067
---

# T0067: CI: macOS で just ffi-check (std の無い静的ライブラリの形を含む) を流す

## 目的

T0491 で `just ffi-check` (scripts/ffi.mjs) が、ホスト向けの `std` の無い `ugoe-ffi` の静的ライブラリ (`--profile nostd`) を
C ライブラリとリンクして見本のモデルで動かすようになった。CI では rust (bindings) のグループ (ubuntu) だけで流れていて、macOS では
確かめていない (`aligned_alloc` は macOS 10.15 以降にある。リンクのフラグは `-framework CoreFoundation`)。macOS の CI の
rust (macos) のジョブで `just ffi-check` を流し、C ABI の見本 (std がある形と無い形) が macOS でもリンクでき、動くことを見続ける。

## 受け入れ条件

- [x] `.github/workflows/ci.yml` の `rust-os` の macOS で `just ffi-check` を流す手順を足した (cbindgen の入れ方は rust-group の
      「Install cbindgen」に合わせる。前の手順が落ちても流す `!cancelled()` の扱いもほかの手順と揃える)。Windows で流すか
      (std がある形だけ。std の無い形は MSVC に `aligned_alloc` が無いので飛ばす) も決め、理由をコメントに書いた
- [x] 手元で macOS を使えなければ、使えないことと、push した後の CI で確かめることをメモに書いた (macOS の実機・CI で通ったなら、その結果を書いた)
- [x] docs/testing.md (CI のジョブ・別の OS の検査) と crates/ugoe-ffi/README.md#std-の無い環境 の、どの OS で確かめているかの記述を直した
- [x] `just ci` が通る

## 参考

- T0491 のメモ (確かめていないこと・起票の候補)、scripts/ffi.mjs (`checkNostd`)、justfile の `ffi-check`
- docs/testing.md#windows-と-macos、docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md

## メモ

- T0066 で起票した (元の候補: T0491 (macOS の CI で std の無い形を含む `just ffi-check` を流す))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。

### やったこと

- ci.yml の `rust-os` に、macOS だけで動く 2 つの手順を足した (`if: ${{ !cancelled() && matrix.name == 'macos' }}`。`Clippy (apps)` と同じ扱い)。
  「Install cbindgen」(`taiki-e/install-action@v2`・`cbindgen@0.29.4`。rust (bindings) と同じ) と「C ABI」(`just ffi-check`)。
  `!cancelled()` は、前の手順 (clippy・テスト) が落ちても C ABI の結果を見られるようにし、`cache-on-failure` のキャッシュに
  依存をそろえる (T0051 の方針と同じ)。
- **Windows では流さないと決めた** (ci.yml のコメントと docs/testing.md#windows-と-macos に理由): scripts/ffi.mjs のリンクは gcc 形式の
  引数の `cc` と Linux・macOS のライブラリの一覧 (`system`) を前提にしていて、windows-latest には `cc` が無く、MSVC は開発者のシェル
  (vcvars) の外では使えない。流すには ffi.mjs に MSVC (か clang-cl) の引数と Windows のシステムのライブラリ (`rustc --print native-static-libs`
  が出すもの) を足す必要があり、手元で確かめられない。std の無い形はどのみち `aligned_alloc` が無いので飛ばす。Windows の `ugoe-ffi`
  のビルドは unity.yml の `native (windows-x86_64)` (C の見本は動かさない) が見ている。
- 文書: docs/testing.md (CI のジョブの表の `rust (windows)`・`rust (macos)`、「Windows と macOS」に 1 項、「CI のキャッシュ」の
  `cache-on-failure` の項に rust (macos) のキャッシュの増分)、crates/ugoe-ffi/README.md (ヘッダの生成の「CI の … ジョブで別に実行する」と
  #std-の無い環境 の「検査」)。docs/portability.md と ADR 0031 は「ホスト (Linux・macOS)」と書いてあり、どの CI のジョブかは書いていないので変えていない。
- **範囲外の変更:** crates/ugoe-ffi/README.md (受け入れ条件 3 が求める記述の直し)。
- **キャッシュと時間の見積もり (手元の Linux x86_64、空の target、`CARGO_PROFILE_DEV_DEBUG=0`・`CARGO_INCREMENTAL=0`):**
  `cargo build -p ugoe-ffi` が 19 秒、`--profile nostd` の staticlib が 22 秒。`just test-os` と同じクレートのテストのビルドの後に
  `cargo build -p ugoe-ffi` をすると、ugoe-* 以外でコンパイルしたのは 10 個だけ (std がある形の依存はほぼ重なる)。`nostd` の依存
  (38 種類) の deps・build・.fingerprint を gzip して約 36MB。rust (macos) のキャッシュ (T0051 の計測で約 496MB) への増分は約 40MB の見込み。
  macOS のランナー (3 コア) では 2 つのビルドで 1〜2 分ほど延びる見込み (キャッシュが当たれば ugoe-* だけ)。
- actionlint 1.7.12 (shellcheck 0.9.0): ci.yml の指摘は gpu・godot・conformance の既存の `ls` の SC2012 (info) の 3 件だけで、main と同じ。
- 手元 (Linux) で `just ffi-check`・`just refs`・`just ci` が通った。

### CI で確かめること (push の後。このタスクでは確かめていない。手元に macOS が無い)

- `rust (macos)` の「Install cbindgen」が cbindgen 0.29.4 を入れ (taiki-e/install-action は macOS の aarch64 にも対応)、「C ABI」で
  `header ok`、`sample` の 3 つの実行 (std がある形。リンクのフラグ `-framework CoreFoundation` で足りるか)、`nostd-sample` の 3 つのモデルの
  `ok (std-less static library)` が出ること。
- 落ちたら見るところ:
  - std がある形のリンクの未解決のシンボル → `cargo rustc -p ugoe-ffi --lib --crate-type staticlib -- --print native-static-libs` の出力に
    合わせて scripts/ffi.mjs の `system.darwin` を直す。
  - std の無い形の `aligned_alloc` (macOS 10.15 以降。arm64 の既定の最小の版は 11.0) や `rust_eh_personality` (Mach-O では `_` が付くが
    C と Rust の両方で付くので揃う見込み) の未解決。
  - Apple clang の `-Werror -pedantic` で sample.c・nostd.c・nostd_main.c の警告。
- rust (macos) の時間の延びと、main の 2 回目の run でキャッシュ (`v0-rust-macos-…`) が `nostd` の依存を含んで完全一致で戻ること。

### やらなかったこと

- Windows で `just ffi-check` を流すこと (上の理由)。macOS の実機・CI での確認 (push しないため)。

### 起票の候補

- Windows で `just ffi-check` の std がある形を流す: scripts/ffi.mjs に Windows のリンク (clang か MSVC の `cl` を
  `ilammy/msvc-dev-cmd` などで使う。システムのライブラリは `--print native-static-libs` から取る) を足し、ci.yml の rust (windows) で流す。
  crates: scripts・.github/workflows。std がある形の C の見本が Windows で動くことを見続けられる。
