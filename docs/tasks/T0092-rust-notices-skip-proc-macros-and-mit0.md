---
id: T0092
title: "公開: Rust のクレートのライセンスの表記から手続きマクロのクレートを外し、選べるライセンスで MIT-0 を Apache-2.0 より先にする"
status: done
milestone: Release
crates: [scripts, apps/editor, apps/viewer, apps/studio, bindings/web, bindings/unity, bindings/godot, crates/ugoe-ffi, docs]
depends_on: []
assignee: claude-worker-T0092
---

# T0092: 手続きマクロを表記から外し、MIT-0 を先にする

## 目的

T0005 (デスクトップアプリ) と T1407 (MIT の配布物) が cargo-about で作る Rust のクレートの表記には、2 つの載せすぎ・選び方の残りがある。

1. 手続きマクロのクレート (`serde_derive`・`thiserror-impl`・`schemars_derive`・`godot-macros` など) はコンパイルのときだけ動き、配布物の
   バイナリに入らないが、cargo-about は通常の依存と区別しないので表記に載っている。
2. 選べるライセンスで、`dunce` (`CC0-1.0 OR MIT-0 OR Apache-2.0`) は accepted の並び (`scripts/about.toml`) で Apache-2.0 を選んでいる。
   短い MIT-0 を先にして、表記を短くする。

## 受け入れ条件

- [x] 表記の生成 (`scripts/lib/rust-notices.mjs`) で、`cargo metadata` の target の kind が `proc-macro` だけのクレートを除く
      (cargo-about の JSON の外で除く。除いたクレートの数を生成のログに出す)。手続きマクロだけが使うクレート (`syn`・`quote`・`proc-macro2` など。
      通常の依存からも使われていれば残す) も、バイナリに入らないなら除くかを決めてメモに書く
- [x] 選べるライセンスの並びで MIT-0 を Apache-2.0 より前にした (`scripts/about.toml` の accepted と、`scripts/check-licenses.mjs` の
      GPL_COMPATIBLE の一致の検査が通る)。`dunce` の表記が MIT-0 の文になる
- [x] 単体テスト (`scripts/rust-notices.test.mjs`) に、手続きマクロを除くことと、選ぶライセンスの並びを足した
- [x] `just notices` で 3 つのアプリの `third-party-notices.txt` と、T1407 の 4 つの配布物の表記を作り直してコミットした。`just licenses` が通る
- [x] docs/release.md#同梱物のライセンスの表記 (と #mit-の配布物) の、載せるクレートの説明を直した
- [x] `just ci` が通る

## 参考

- T0005・T1407 のメモ (起票の候補)、`scripts/lib/rust-notices.mjs`・`scripts/about.toml`・`scripts/third-party-notices.mjs`・`scripts/check-licenses.mjs`
- docs/release.md#同梱物のライセンスの表記

## メモ

- T0089 で起票した (元の候補: T0005 (手続きマクロのクレートは表記に載る。`dunce` の MIT-0 を先にする)、T1407 (手続きマクロのクレートは配布物の表記に載る))。
  同じ生成のコードを触るので 1 つにまとめた。
- Release のマイルストーン (docs/roadmap.md の Release)。T0005・T1407 と同じ。
- `crates:` は作り直す表記のファイルの置き場所 (アプリの `public/`・バインディングの配布物) を含めるために広く書いた。コードを触るのは `scripts/` だけ。

- 完了 (claude-worker-T0092):
  - `scripts/lib/rust-notices.mjs` に `compileOnlyCrates` を足した。`cargo metadata` (`--frozen`、ターゲットは cargo-about と同じものを
    `--filter-platform` に。アプリは about.toml の targets) の依存を、根のクレート (`resolve.root`) から通常の依存 (kind が null) だけで
    たどり、手続きマクロのクレート (target の kind が `proc-macro`) の先はたどらない。たどれないものを、手続きマクロ (`proc-macro`) と
    手続きマクロ・ビルドスクリプトだけが使うもの (`compile-time`) に分けて返し、`rustNotices(about, { compileOnly })` が除いて数える。
    `just notices` のログに 1 行ずつ出す (例: editor は手続きマクロ 41 個・それだけが使うもの 23 個)。結果は cargo-about の JSON と
    いっしょに `target/cargo-about/` に覚える (`cargoAboutJson` は `rustNoticeSources` に替えた)。
  - `syn`・`quote`・`proc-macro2` などの扱い: 除くことにした。手続きマクロのクレートはホスト向けの dylib としてビルドされてコンパイラが
    読み込むだけで、その依存もホスト向けにビルドされて手続きマクロにリンクされるだけなので、配布物のバイナリには入らない
    (ビルドスクリプトの依存も同じで、cargo-about の `ignore-build-dependencies` と揃う)。通常の依存からも使われるものは残る
    (テストに例がある)。
  - 除きすぎないことの根拠: cargo metadata の依存はワークスペースのすべてのクレートの機能を合わせた集合なので、根のビルドに入らない
    クレートを通って残ることはあっても、入るクレートへの道を落とすことはない。さらに、たどるのを cargo-about が見つけたクレートの中に
    限った (根のビルドに入らない `naga` を通って `unicode-ident` が残っていたため)。確かめとして、`cargo tree -e normal,no-proc-macro
    --target <各ターゲット>` の和と表記のクレートを比べた: C ABI (23)・`@ugoe/runtime` (70)・Godot (29) は一致した (Unity は C ABI と
    同じクレートで別のターゲット)。3 つのアプリは表記が cargo tree の上位集合で、cargo tree にあるクレートはすべて残っている。
  - `scripts/about.toml` の accepted で MIT-0 を Apache-2.0 より前にした。`dunce` は `(used under MIT-0)` になった
    (check-licenses.mjs の集合の一致の検査は並びを見ないので通る)。テストは about.toml の先頭が MIT・MIT-0・Apache-2.0 の順かを見る。
  - `just notices` で 3 つのアプリと 4 つの配布物の表記を作り直した。`just licenses` と `just ci` が通った。
  - docs/release.md の #同梱物のライセンスの表記 と #mit-の配布物 を直した。
- やらなかったこと: アプリの表記には、ホスト向けの機能だけが引き込むクレートが残る (`tauri-utils` のビルド用の機能の依存の
  `cargo_metadata`・`html5ever`・`cssparser`・`schemars 0.8` など、アプリごとに 30〜40 個。ほかに `windows_x86_64_gnu`・`winapi` など
  cargo-about のターゲットの合わせ方で入るもの)。cargo-about も cargo metadata も機能をホスト向けと合わせて解くため。
  resolver 2 は実際のビルドでホスト向けの機能を分けるので、これらはバイナリに入らない。
- 起票の候補: アプリの表記の対象を `cargo tree -e normal,no-proc-macro --target <各ターゲット>` (実際のビルドと同じく機能をホスト向けと
  分けて解く) で求め、cargo-about の結果をその集合に絞る (上の 30〜40 個をさらに除ける。cargo tree の出力の読み方とテストが要る)。
