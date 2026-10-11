---
id: T0005
title: 3 つのデスクトップアプリの配布物に、Rust のクレートのライセンスの表記を入れる
status: done
milestone: Release
crates: [apps/editor, apps/viewer, apps/studio, scripts, justfile, docs]
depends_on: [T0811]
assignee: claude-worker-T0005
---

# T0005: Rust のクレートのライセンスの表記

## 目的

エディタ・ビューア・スタジオの third-party-notices は、同梱する JS と MediaPipe の資産だけを載せていて、バイナリに入る Rust のクレート (MIT・Apache-2.0 など)
のライセンスの表記が無い (T0811 のメモ)。MIT・Apache-2.0 は配布物に表記を入れることを求めるので、配布の前に揃える。

## 受け入れ条件

- [x] 各アプリのバイナリに入る Rust のクレートと、そのライセンスの全文を、配布物とライセンスの画面に入れる。作り方 (cargo-about などの道具を使うか、
      scripts/check-licenses.mjs を広げて自前で作るか) は、外部の道具を足すなら利用者に確認する
- [x] 表記を作り直す just のレシピと、依存が変わったのに表記が古いままなら失敗する検査
- [x] JS の表記の版を lockfile と突き合わせる検査 (T0811 の後続の候補)
- [x] `just ci` が通る

## 参考

- docs/tasks/T0811-studio-third-party-notices.md・T0723-viewer-ui-and-i18n.md のメモ、scripts/check-licenses.mjs

## メモ

- 判断 (利用者、2026-10-11): 表記は cargo-about (EmbarkStudios、MIT/Apache-2.0) で作る。T1407 も同じ道具を使う。
- **やったこと:**
  - cargo-about 0.9.2 (版は `scripts/lib/rust-notices.mjs` の `CARGO_ABOUT_VERSION`)。0.9 から CLI は `cli` の機能の中にあるので、
    入れ方は `cargo install cargo-about --version 0.9.2 --locked --features cli`。`just cargo-about` が無いか版が違うときだけ入れる。
  - 設定 `scripts/about.toml`: accepted (check-licenses.mjs の GPL_COMPATIBLE と同じ集合。並びが選ぶライセンスの優先で、MIT が最初)・
    targets (Windows・macOS・Linux の x86_64 と aarch64)・ビルドスクリプトとテストだけの依存を除く。accepted と GPL_COMPATIBLE の
    一致は check-licenses.mjs が検査する。
  - handlebars のテンプレートは使わず、`cargo about generate --format json --frozen --fail` の JSON を `scripts/lib/rust-notices.mjs`
    (`rustNotices`・`renderRustNotices`) で組み立てる。理由: ワークスペースのクレート (ugoe 自身) を除く・cargo-about がソースの見出し
    (ring の .rs、schemars の regex_syntax.rs、libdbus-sys の .h など) から拾った「ソースの全文」を除く・ライセンスのファイルを持たない
    クレート (windows・objc2・Tauri のプラグインなど) の標準の文から `<year> <copyright holders>` の行を除き Cargo.toml の authors を添える、
    をするため。T1407 は `cargoAboutJson(root, manifest, { targets })` と同じ 2 つの関数を使い回せる (targets を上書きできる)。
  - `scripts/third-party-notices.mjs` の表記の最後に「Rust crates compiled into the application」の節 (クレートの一覧と、まとめた文と
    使うクレート) を足した。対象のアプリは `APPS` の `manifest` (src-tauri の Cargo.toml)。3 つのファイルは 30〜42 万バイトになった。
  - `just notices` (生成し直す)・`just cargo-about`・`just cargo-about-version`。`just licenses` (`just ci`) の `--check` が Rust の節も
    突き合わせるので、Cargo.lock やアプリの依存が変わって表記が古ければ失敗する。cargo-about はアプリ 1 つで数十秒 (8 コア分の CPU で
    数分) かかるので、入力 (版・設定・Cargo.lock・git が追う Cargo.toml・対象) の SHA-256 ごとに `target/cargo-about/` に JSON を覚える。
    覚えたものを消して生成し直しても同じ表記になることを確かめた (決定的)。
  - CI (ci.yml の rust-checks): `just cargo-about-version` → actions/cache で `~/.cargo/bin/cargo-about` を版ごとに覚える → `just cargo-about`
    → `just licenses`。
  - JS の版とロックファイルの突き合わせ (受け入れ条件 3) は T0926 の生成と `--check` で既に満たしている (表記は node_modules から作り、
    node_modules はロックファイルで決まる)。今回は手を入れていない。
  - テスト: `scripts/rust-notices.test.mjs` (版の読み取り・ライセンスのファイルの判定・ソースの URL と authors・標準の文の欄の除去・
    除外とまとめと並び・文の無いクレートで失敗)。`scripts/third-party-notices.test.mjs` に Rust の節を足した。
  - 文書: docs/release.md#同梱物のライセンスの表記 を書き直した。AGENTS.md・README.md (必要なもの)・docs/testing.md・docs/editor.md・
    3 つのアプリの README の 1 行ずつ。
- **範囲の外の変更 (小さいもの):** `.github/workflows/ci.yml` (cargo-about を入れる 3 段)、`AGENTS.md`・`README.md` の 1 行。
- **確かめていないこと:** CI での実行 (actions/cache と `cargo install` の段)。Tauri の実物のライセンスの画面での表示 (`<pre>` は
  `white-space: pre-wrap` で、標準の文の長い行も折り返す)。macOS・Windows のビルドに入るクレートは cargo-about の targets の解決に
  頼っていて、実際のビルドと突き合わせていない。
- **注意 (ほかの作業者へ):** マージの後は `just ci` に cargo-about が要る。無ければ `just licenses` が `just cargo-about` を案内して失敗する。
- **起票の候補:**
  - 手続きマクロのクレート (serde_derive・schemars_derive など) はバイナリに入らないが、cargo-about は区別しないので載っている。
    除くなら cargo metadata の target の kind (`proc-macro`) で外す。
  - 選べるライセンスで dunce (`CC0-1.0 OR MIT-0 OR Apache-2.0`) は Apache-2.0 を選んでいる (accepted の並び)。短い MIT-0 を
    先にするかは好みの範囲。
