---
id: T1407
title: "公開: MIT の配布物 (npm の @ugoe/runtime・Unity・Godot・C ABI) に、入っている Rust のクレートのライセンスの表記を入れる"
status: done
milestone: Release
crates: [bindings/web, bindings/unity, bindings/godot, scripts, justfile, .github/workflows, docs]
depends_on: [T0005]
assignee: claude-worker-T1407
---

# T1407: MIT の配布物の第三者ライセンスの表記

## 目的

T0005 はデスクトップアプリ (GPL-3.0-only) の配布物に Rust のクレートの表記を入れる。ランタイムを外へ出す MIT の配布物にも、
中に入っている Rust のクレート (MIT・Apache-2.0 など。表記を配布物に入れることを求める) の表記が無い。

- npm の `@ugoe/runtime`: `dist` の wasm (ugoe-wasm とその依存)。パッケージの `files` は `dist`・`README.md`・`LICENSE` だけ。
- Unity の UPM の tarball: 各プラットフォームの `ugoe-ffi` のネイティブライブラリ (`unity.yml`)。
- Godot のアドオン: GDExtension のライブラリ。godot-rust は MPL-2.0 なので、表記に加えて MPL-2.0 の求めるソースの在り処
  (どこで手に入るか) を示す (ADR 0026)。
- C ABI の配布物 (T1408 で作る。T1408 が先に入っていればそれにも入れる)。

公開の時期は v1.0.0 の完全リリースと判断するまで決めない (docs/roadmap.md の Release)。公開 (npm の publish・アセットストア・Release の公開) は人の手で、
このタスクは配布物に表記が入るところまで。

## 受け入れ条件

- [x] 配布物ごとに、入っている Rust のクレート (ビルドのターゲットと features で決まるもの) の名前・版・ライセンス・ライセンスの全文の表記を作る。
      作り方は T0005 で決めた道具と形を使う (別の道具を足すなら利用者に確認する)
- [x] 表記が配布物に入る: npm のパッケージ (`files` に足す)・UPM の tarball (`Third Party Notices.md` など Unity の慣例の名前)・Godot のアドオンの zip
      (T0421 で作る。まだなら `bindings/godot` のアドオンのフォルダ)。godot-rust の節に MPL-2.0 のソースの在り処を書いた
- [x] 表記を作り直す just のレシピと、依存 (Cargo.lock・bindings/godot の Cargo.lock) が変わったのに表記が古いままなら失敗する検査 (`just licenses` に入れる)
- [x] docs/release.md#同梱物のライセンスの表記 に MIT の配布物の節を足し、各 README (bindings/web・unity・godot) のライセンスの節から辿れるようにした
- [x] `just ci` が通る (bindings/godot・bindings/unity は README にある検査も)

## 参考

- T0005 (デスクトップアプリの表記)・T0811・T0926 のメモ、docs/release.md#同梱物のライセンスの表記、scripts/check-licenses.mjs・scripts/third-party-notices.mjs
- ADR 0018・ADR 0025・ADR 0026、bindings/web/package.json、`.github/workflows/unity.yml`・`web-runtime-release.yml`

## メモ

- T0083 で起票した (元の候補: オーケストレーターの点検 (MIT の配布物の第三者ライセンスの表記: npm の @ugoe/runtime の wasm・Unity の tarball の
  ネイティブライブラリ・Godot のアドオン (godot-rust は MPL-2.0 なのでソースの在り処も) に、入っている Rust のクレートの表記を入れ、
  依存が変わったら古さを検査する。T0005 はデスクトップアプリだけが対象))。
- Release のマイルストーン (docs/roadmap.md の Release)。公開の時期が決まるまで着手しない。
- T0005 で表記の作り方 (道具) を決めるので、その後に行う。T0421 (Godot の zip・UPM の tarball の添付)・T1408 (C ABI の配布物) と同じ配布物を触る。

- 判断 (利用者、2026-10-11): 表記は cargo-about で作る (T0005 と同じ)。

- やったこと (T1407):
  - `scripts/dist-notices.mjs` (`DISTS`) が、T0005 の道具 (cargo-about・`scripts/about.toml`・`scripts/lib/rust-notices.mjs`) で配布物ごとの表記を作る。
    対象はライブラリのクレートとターゲット: `@ugoe/runtime` = `ugoe-wasm` の wasm32、Unity = `ugoe-ffi` の `unity.yml` の 8 ターゲット、
    Godot = `bindings/godot` (自分の Cargo.lock) の `.gdextension` の 5 ターゲット、C ABI = `ugoe-ffi` の T1408 の 5 ターゲット。
  - 置き場: `bindings/web/THIRD-PARTY-NOTICES.txt` (`package.json` の `files` に足した)・`bindings/unity/com.ugoe.unity/Third Party Notices.md`
    (と .meta。`release.mjs pack` の必須のファイルに足した)・`bindings/godot/example/addons/ugoe/THIRD-PARTY-NOTICES.txt`・
    `crates/ugoe-ffi/THIRD-PARTY-NOTICES.txt`。Godot の表記の終わりに、MPL-2.0 で使うクレート (godot・godot-cell・godot-core・godot-ffi・
    godot-macros) ごとに crates.io の版のページ・`.crate`・リポジトリの URL を書く (MPL-2.0 の 3.2)。
  - `just notices` が作り直し、`just licenses` が `--check` で古さを検査する。テストは `scripts/dist-notices.test.mjs`。
  - T0005 の `rust-notices.mjs` の直し: `--target` に複数のターゲットを 1 つの引数の列で渡していて cargo-about が拒んでいた
    (T0005 では使われていなかった道) ので、1 つずつ渡す (`aboutArgs`。テストを足した)。結果を覚える鍵に `bindings/godot/Cargo.lock`
    が入っていなかったので、git が追うすべての `Cargo.lock` を入れた。
  - docs/release.md#mit-の配布物 を書き、bindings/web・bindings/unity (と com.ugoe.unity の README)・bindings/godot の README のライセンスの節から辿れるようにした。
    bindings/godot/README.md のアセットライブラリの手順 2 (告知を手で足す) を、生成した表記を使う形に直した。
- 範囲の外の小さな変更: C ABI の表記を `crates/ugoe-ffi/THIRD-PARTY-NOTICES.txt` に置いた (生成物のファイルを足しただけ。コードは触っていない)。
- T1408 が main に入った後 (オーケストレーターの指示): `scripts/ffi-dist.mjs` が `crates/ugoe-ffi/THIRD-PARTY-NOTICES.txt` をアーカイブの
  `THIRD-PARTY-NOTICES.txt` に詰め、`expectedFiles` (と `ffi-dist.test.mjs`)・`crates/ugoe-ffi/archive/README.md` の中身の一覧・
  docs/release.md の C ABI の配布物の節に足した。`just ffi-dist` と `just ffi-dist-check` (展開して見本を静的・共有の両方でビルドして動かす) が
  Linux x86_64 で通った。
- やらなかったこと:
  - Godot の zip (T0421) はまだ無いので、アドオンのフォルダに置いた (zip はフォルダをそのまま詰めれば入る)。
  - 実際の Unity・Godot のエディタでの表示は確かめていない (Unity の .meta の検査と `release.test.mjs`、Godot の `build.mjs --check` は通った)。
- 起票の候補:
  - 手続きマクロのクレート (serde_derive・thiserror-impl・godot-macros など) は配布物のバイナリに入らないが表記に載る (T0005 と同じ。
    cargo-about が通常の依存と区別しない)。載せすぎで害は無いが、外すなら cargo-about の外で除く。
- 検査: `just ci` が通った (`just licenses` の `distribution notices ok (4 distributions)` を含む)。`just unity-check` (.meta と release.test.mjs)、
  `node bindings/godot/build.mjs --check` (ヘッドレスの Godot の smoke まで) も通った。
