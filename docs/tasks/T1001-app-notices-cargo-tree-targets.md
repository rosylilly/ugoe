---
id: T1001
title: "公開: アプリのライセンスの表記の対象を cargo tree で求め、ホスト向けの機能だけが引き込むクレートを除く"
status: done
milestone: Release
crates: [scripts, apps/editor, apps/viewer, apps/studio, docs]
depends_on: []
assignee: claude-worker-T1001
---

# T1001: アプリの表記をターゲットごとの実際の依存に絞る

## 目的

T0092 で、手続きマクロとそれだけが使うクレートを表記から外した。アプリの表記にはまだ、ホスト向けの機能だけが引き込むクレート
(`tauri-utils` のビルド用の機能の依存の `cargo_metadata`・`html5ever`・`cssparser`・`schemars 0.8` など、アプリごとに 30〜40 個) と、
cargo-about のターゲットの合わせ方で入るもの (`windows_x86_64_gnu`・`winapi` など) が残る。cargo-about も `cargo metadata` も機能を
ホスト向けと合わせて解くため。resolver 2 の実際のビルドはホスト向けの機能を分けるので、これらはバイナリに入らない。
実際のビルドと同じ解き方の `cargo tree -e normal,no-proc-macro --target <各ターゲット>` で対象を求め、cargo-about の結果をその集合に絞る。

## 受け入れ条件

- [x] `scripts/lib/rust-notices.mjs` が、アプリの about.toml の各ターゲットで `cargo tree -e normal,no-proc-macro --target <ターゲット> --prefix none`
      (`--frozen`。`-f '{p}'` など読みやすい形) の和を求め、cargo-about の結果をその集合に絞る (T0092 の `compileOnlyCrates` を置き換えるか、
      両方を使うかを決めてメモに書く)。C ABI・`@ugoe/runtime`・Godot の表記は T0092 で cargo tree と一致しているので、同じ道具に寄せるかも決める
- [x] 除きすぎないことを確かめた: 絞った後の表記が、各ターゲットの cargo tree の和と一致する (テストか `just notices` のログで)
- [x] cargo tree の出力の読み取りの単体テスト (版の付いた名前・`(*)`・`(proc-macro)` の印・ワークスペースのパスの付いた行)
- [x] `just notices` で 3 つのアプリの表記を作り直し、除いたクレートの数をメモに書いた。`just licenses` が通る
- [x] docs/release.md#同梱物のライセンスの表記 を直した
- [x] `just ci` が通る

## 参考

- T0092 のメモ (`compileOnlyCrates` と確かめ方・やらなかったこと)、docs/release.md#同梱物のライセンスの表記
- `scripts/lib/rust-notices.mjs`・`scripts/about.toml`・アプリの about.toml

## メモ

- T0095 で起票した (元の候補: T0092 (アプリの表記の対象を cargo tree -e normal,no-proc-macro --target <各ターゲット> で求め、ホスト向けの
  機能だけが引き込む 30〜40 個も除く))。
- Release のマイルストーン。T0092 と同じ。番号は基盤の帯の 2 つ目の範囲 (T0092 と同じ分野)。

- 完了 (claude-worker-T1001):
  - `scripts/lib/rust-notices.mjs` の `compileOnlyCrates` (`cargo metadata` をたどる) を **置き換えた**。`treeArgs` が
    `cargo tree -e normal,no-proc-macro --target <ターゲット> --prefix none -f '{p}' --frozen --manifest-path <manifest>` の引数を作り、
    `parseCargoTree` が出力から名前と版の集合を読む (末尾の `(*)`・`(proc-macro)` の印を外し、パスの付いた行 (ワークスペースの
    クレート) と手続きマクロの行は入れない。git の URL の付いた行は入れる。読めない行は例外)。`rustNoticeSources` はターゲットごとに
    1 回ずつ流して和を取り (機能の解き方がターゲットで違うため)、cargo-about の JSON といっしょに `target/cargo-about/` に覚える。
    `rustNotices(about, { shipped })` は和の外のクレートを除いて数え、和にあって cargo-about が見つけていないクレートがあれば例外にする
    (除きすぎない検査。載せるものは和と一致する)。両方を使わなかった理由: cargo tree の `no-proc-macro` と `-e normal` が手続きマクロと
    ビルドスクリプトだけの依存も除くので、`compileOnlyCrates` の役目は和に含まれる。
  - C ABI・`@ugoe/runtime`・Unity・Godot の表記も同じ道具に寄せた (`scripts/dist-notices.mjs`)。4 つの表記は変わらなかった
    (T0092 で cargo tree と一致していたとおり)。
  - `just notices` の結果: editor は 400 → 369 (31 個除いた)、viewer は 277 → 246 (31 個)、studio は 321 → 292 (29 個)。足されたものは無い。
    除いたのは `cargo_metadata`・`camino`・`cargo-platform`・`html5ever`・`markup5ever`・`cssparser`・`selectors`・`dom_query`・
    `schemars 0.8.22`・`string_cache`・`tendril`・`swift-rs`・`tempfile`・`winapi`・`windows_x86_64_gnu` (0.52・0.53)・
    `windows_x86_64_msvc 0.52.6`・`uds_windows` など。ログの数 (95・94・90) は cargo-about が見つけたもののうち和の外にあるもの全部
    (手続きマクロとそれだけが使うもの、T0092 で除いていた分を含む)。
  - 確かめ方: 生成とは別に、6 つのターゲットそれぞれで `cargo tree -e normal,no-proc-macro --target <ターゲット>` (既定の木の形) を流し、
    ugoe 自身を除いた名前と版の和を取って、表記のクレートの一覧と突き合わせた。3 つのアプリとも完全に一致し、除いたクレートは
    どのターゲットの木にも無かった。`tauri build` が足す `tauri/custom-protocol` の機能を付けても cargo tree の和は変わらない。
  - 単体テスト (`scripts/rust-notices.test.mjs`): cargo tree の引数、出力の読み取り (版の付いた名前・`+` の付いた版・`(*)`・
    `(proc-macro)`・ワークスペースのパスの行・括弧を含む Windows のパス・git の URL・CRLF・読めない行)、和の外のクレートを除いて数えること、
    和にあって cargo-about に無いクレートで失敗すること。
  - docs/release.md の #同梱物のライセンスの表記 と #mit-の配布物 を直した。`just licenses` と `just ci` が通った。
- やらなかったこと: 実際のビルド (`cargo build --target …` の unit graph) との突き合わせはしていない (unit graph は nightly の `-Z`)。
  cargo tree は実際のビルドと同じ feature resolver を使うので、それを根拠にした。
- 起票の候補: なし。
