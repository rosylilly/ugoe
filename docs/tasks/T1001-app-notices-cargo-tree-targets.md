---
id: T1001
title: "公開: アプリのライセンスの表記の対象を cargo tree で求め、ホスト向けの機能だけが引き込むクレートを除く"
status: todo
milestone: Release
crates: [scripts, apps/editor, apps/viewer, apps/studio, docs]
depends_on: []
assignee:
---

# T1001: アプリの表記をターゲットごとの実際の依存に絞る

## 目的

T0092 で、手続きマクロとそれだけが使うクレートを表記から外した。アプリの表記にはまだ、ホスト向けの機能だけが引き込むクレート
(`tauri-utils` のビルド用の機能の依存の `cargo_metadata`・`html5ever`・`cssparser`・`schemars 0.8` など、アプリごとに 30〜40 個) と、
cargo-about のターゲットの合わせ方で入るもの (`windows_x86_64_gnu`・`winapi` など) が残る。cargo-about も `cargo metadata` も機能を
ホスト向けと合わせて解くため。resolver 2 の実際のビルドはホスト向けの機能を分けるので、これらはバイナリに入らない。
実際のビルドと同じ解き方の `cargo tree -e normal,no-proc-macro --target <各ターゲット>` で対象を求め、cargo-about の結果をその集合に絞る。

## 受け入れ条件

- [ ] `scripts/lib/rust-notices.mjs` が、アプリの about.toml の各ターゲットで `cargo tree -e normal,no-proc-macro --target <ターゲット> --prefix none`
      (`--frozen`。`-f '{p}'` など読みやすい形) の和を求め、cargo-about の結果をその集合に絞る (T0092 の `compileOnlyCrates` を置き換えるか、
      両方を使うかを決めてメモに書く)。C ABI・`@ugoe/runtime`・Godot の表記は T0092 で cargo tree と一致しているので、同じ道具に寄せるかも決める
- [ ] 除きすぎないことを確かめた: 絞った後の表記が、各ターゲットの cargo tree の和と一致する (テストか `just notices` のログで)
- [ ] cargo tree の出力の読み取りの単体テスト (版の付いた名前・`(*)`・`(proc-macro)` の印・ワークスペースのパスの付いた行)
- [ ] `just notices` で 3 つのアプリの表記を作り直し、除いたクレートの数をメモに書いた。`just licenses` が通る
- [ ] docs/release.md#同梱物のライセンスの表記 を直した
- [ ] `just ci` が通る

## 参考

- T0092 のメモ (`compileOnlyCrates` と確かめ方・やらなかったこと)、docs/release.md#同梱物のライセンスの表記
- `scripts/lib/rust-notices.mjs`・`scripts/about.toml`・アプリの about.toml

## メモ

- T0095 で起票した (元の候補: T0092 (アプリの表記の対象を cargo tree -e normal,no-proc-macro --target <各ターゲット> で求め、ホスト向けの
  機能だけが引き込む 30〜40 個も除く))。
- Release のマイルストーン。T0092 と同じ。番号は基盤の帯の 2 つ目の範囲 (T0092 と同じ分野)。
