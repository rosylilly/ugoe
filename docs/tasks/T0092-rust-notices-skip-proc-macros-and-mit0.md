---
id: T0092
title: "公開: Rust のクレートのライセンスの表記から手続きマクロのクレートを外し、選べるライセンスで MIT-0 を Apache-2.0 より先にする"
status: todo
milestone: Release
crates: [scripts, apps/editor, apps/viewer, apps/studio, bindings/web, bindings/unity, bindings/godot, crates/ugoe-ffi, docs]
depends_on: []
assignee:
---

# T0092: 手続きマクロを表記から外し、MIT-0 を先にする

## 目的

T0005 (デスクトップアプリ) と T1407 (MIT の配布物) が cargo-about で作る Rust のクレートの表記には、2 つの載せすぎ・選び方の残りがある。

1. 手続きマクロのクレート (`serde_derive`・`thiserror-impl`・`schemars_derive`・`godot-macros` など) はコンパイルのときだけ動き、配布物の
   バイナリに入らないが、cargo-about は通常の依存と区別しないので表記に載っている。
2. 選べるライセンスで、`dunce` (`CC0-1.0 OR MIT-0 OR Apache-2.0`) は accepted の並び (`scripts/about.toml`) で Apache-2.0 を選んでいる。
   短い MIT-0 を先にして、表記を短くする。

## 受け入れ条件

- [ ] 表記の生成 (`scripts/lib/rust-notices.mjs`) で、`cargo metadata` の target の kind が `proc-macro` だけのクレートを除く
      (cargo-about の JSON の外で除く。除いたクレートの数を生成のログに出す)。手続きマクロだけが使うクレート (`syn`・`quote`・`proc-macro2` など。
      通常の依存からも使われていれば残す) も、バイナリに入らないなら除くかを決めてメモに書く
- [ ] 選べるライセンスの並びで MIT-0 を Apache-2.0 より前にした (`scripts/about.toml` の accepted と、`scripts/check-licenses.mjs` の
      GPL_COMPATIBLE の一致の検査が通る)。`dunce` の表記が MIT-0 の文になる
- [ ] 単体テスト (`scripts/rust-notices.test.mjs`) に、手続きマクロを除くことと、選ぶライセンスの並びを足した
- [ ] `just notices` で 3 つのアプリの `third-party-notices.txt` と、T1407 の 4 つの配布物の表記を作り直してコミットした。`just licenses` が通る
- [ ] docs/release.md#同梱物のライセンスの表記 (と #mit-の配布物) の、載せるクレートの説明を直した
- [ ] `just ci` が通る

## 参考

- T0005・T1407 のメモ (起票の候補)、`scripts/lib/rust-notices.mjs`・`scripts/about.toml`・`scripts/third-party-notices.mjs`・`scripts/check-licenses.mjs`
- docs/release.md#同梱物のライセンスの表記

## メモ

- T0089 で起票した (元の候補: T0005 (手続きマクロのクレートは表記に載る。`dunce` の MIT-0 を先にする)、T1407 (手続きマクロのクレートは配布物の表記に載る))。
  同じ生成のコードを触るので 1 つにまとめた。
- Release のマイルストーン (docs/roadmap.md の Release)。T0005・T1407 と同じ。
- `crates:` は作り直す表記のファイルの置き場所 (アプリの `public/`・バインディングの配布物) を含めるために広く書いた。コードを触るのは `scripts/` だけ。
