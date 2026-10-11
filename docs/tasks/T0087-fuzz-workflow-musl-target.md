---
id: T0087
title: "fuzz.yml: CI で cargo-fuzz が musl のターゲットでビルドして、5 つのターゲットがすべて fuzz の前に落ちるのを直す"
status: done
milestone: M11
crates: [.github/workflows, fuzz, justfile, docs]
depends_on: []
assignee: claude-worker-T0087
---

# T0087: fuzz.yml のビルドの失敗を直す

## 目的

2026-10-11 に初めて手で起動した fuzz.yml (run 38098547009、main d7b0aaeb) で、5 つのターゲット (bundle・module・image・json・ffi) がすべて fuzz を始める前のビルドで落ちた。
cargo-fuzz が `cargo build --target x86_64-unknown-linux-musl` を呼び、`sanitizer is incompatible with statically linked libc` と `can't find crate for std` で止まる。
taiki-e/install-action が入れたビルド済みの cargo-fuzz が musl 向けで、既定のターゲット (ホストの triple) を musl と見ていると見られる (T0085 のメモ)。手元の `just fuzz` は通る。

## 受け入れ条件

- [x] 原因を確かめた (cargo-fuzz の既定のターゲットの決め方。出典をメモに書く)
- [x] 直した: `fuzz/run.mjs` (または just のレシピ) がホストの gnu のターゲット (`rustc -vV` の host) を `--target` で明示する、または CI で cargo-fuzz を gnu 向けに入れる。
      手元 (Linux・macOS・Windows の扱い) で振る舞いが変わらないこと
- [x] 手元で `just fuzz <ターゲット> 10` と `just fuzz-check` が通る。musl の cargo-fuzz を模した状況 (`--target` を外したときに musl になる) で直す前は落ちることを、できる範囲で確かめた
- [x] docs/testing.md#fuzz と fuzz/README.md を直した
- [x] `just ci` が通る
- [ ] (人の手かオーケストレーター) push の後に fuzz.yml を workflow_dispatch で流し直して通ることを確かめる

## 参考

- .github/workflows/fuzz.yml、fuzz/run.mjs、ADR 0032、T0085 のメモ (定期のワークフローの初回)

## メモ

- オーケストレーターが、初回の fuzz.yml の失敗から起票した (利用者の方針: CI の失敗は起票してすぐ直す)。
- 原因 (claude-worker-T0087): cargo-fuzz 0.13.2 の `--target` の既定は `utils::default_target()` = crate `current_platform` の
  `CURRENT_PLATFORM` で、cargo-fuzz 自身をビルドしたときのターゲットが埋め込まれる
  (https://github.com/rust-fuzz/cargo-fuzz/blob/0.13.2/src/utils.rs、`src/options.rs` の `BuildOptions::triple`)。
  CI の taiki-e/install-action は cargo-fuzz を一覧に持たず cargo-binstall に落ち、GitHub のリリースの
  `cargo-fuzz-0.13.2-x86_64-unknown-linux-musl.tar.gz` (Linux 向けはこれだけ。ほかは x86_64-apple-darwin と x86_64-pc-windows-msvc)
  を入れていた (run 38098547009 の Install cargo-fuzz のログ)。手元は同じ 0.13.2 を `cargo install` でソースから入れた gnu の版なので通っていた。
  版の違いではなく、ビルドしたターゲットの違い。
- 直し方: `fuzz/run.mjs` が `rustc +<toolchain> -vV` の `host:` を読み、`cargo fuzz build`・`cargo fuzz run` に `--target <ホスト>` を付ける
  (オーケストレーターの推奨のとおり。CI でソースから入れる案は、ビルドの時間が延びるので採らなかった)。
  ソースから入れた cargo-fuzz では既定と同じ値を明示するだけなので、手元の Linux (gnu)・macOS・Windows (msvc) の振る舞いは変わらない
  (macOS・Windows の実機では確かめていない)。fuzz.yml のコメント・docs/testing.md#fuzz・fuzz/README.md に理由を書いた。
- 確かめたこと (WSL2 の Linux): リリースの musl の cargo-fuzz を一時ディレクトリに落として PATH の先頭に置き、直す前の `node fuzz/run.mjs json 5`
  が CI と同じ `sanitizer is incompatible with statically linked libc` で落ちることを再現した。直した後は同じ musl の cargo-fuzz で
  `json 10` と `just fuzz-check` が通った。ふだんの (gnu の) cargo-fuzz でも `just fuzz ffi 10` と `just fuzz-check` が通った。`just ci` も通った。
- やらなかったこと: push と fuzz.yml の流し直し (オーケストレーターが行う)。
- 起票の候補: なし (Apple Silicon の macOS でビルド済みの x86_64-apple-darwin の cargo-fuzz を使うと同じ食い違いが起きるが、この直しで同時に避けられる)。
