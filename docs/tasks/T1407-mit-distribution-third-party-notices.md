---
id: T1407
title: "公開: MIT の配布物 (npm の @ugoe/runtime・Unity・Godot・C ABI) に、入っている Rust のクレートのライセンスの表記を入れる"
status: todo
milestone: Release
crates: [bindings/web, bindings/unity, bindings/godot, scripts, justfile, .github/workflows, docs]
depends_on: [T0005]
assignee:
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

- [ ] 配布物ごとに、入っている Rust のクレート (ビルドのターゲットと features で決まるもの) の名前・版・ライセンス・ライセンスの全文の表記を作る。
      作り方は T0005 で決めた道具と形を使う (別の道具を足すなら利用者に確認する)
- [ ] 表記が配布物に入る: npm のパッケージ (`files` に足す)・UPM の tarball (`Third Party Notices.md` など Unity の慣例の名前)・Godot のアドオンの zip
      (T0421 で作る。まだなら `bindings/godot` のアドオンのフォルダ)。godot-rust の節に MPL-2.0 のソースの在り処を書いた
- [ ] 表記を作り直す just のレシピと、依存 (Cargo.lock・bindings/godot の Cargo.lock) が変わったのに表記が古いままなら失敗する検査 (`just licenses` に入れる)
- [ ] docs/release.md#同梱物のライセンスの表記 に MIT の配布物の節を足し、各 README (bindings/web・unity・godot) のライセンスの節から辿れるようにした
- [ ] `just ci` が通る (bindings/godot・bindings/unity は README にある検査も)

## 参考

- T0005 (デスクトップアプリの表記)・T0811・T0926 のメモ、docs/release.md#同梱物のライセンスの表記、scripts/check-licenses.mjs・scripts/third-party-notices.mjs
- ADR 0018・ADR 0025・ADR 0026、bindings/web/package.json、`.github/workflows/unity.yml`・`web-runtime-release.yml`

## メモ

- T0083 で起票した (元の候補: オーケストレーターの点検 (MIT の配布物の第三者ライセンスの表記: npm の @ugoe/runtime の wasm・Unity の tarball の
  ネイティブライブラリ・Godot のアドオン (godot-rust は MPL-2.0 なのでソースの在り処も) に、入っている Rust のクレートの表記を入れ、
  依存が変わったら古さを検査する。T0005 はデスクトップアプリだけが対象))。
- Release のマイルストーン (docs/roadmap.md の Release)。公開の時期が決まるまで着手しない。
- T0005 で表記の作り方 (道具) を決めるので、その後に行う。T0421 (Godot の zip・UPM の tarball の添付)・T1408 (C ABI の配布物) と同じ配布物を触る。
