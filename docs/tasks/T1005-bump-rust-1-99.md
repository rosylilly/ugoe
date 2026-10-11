---
id: T1005
title: "Rust のツールチェインを 1.99.0 に上げる"
status: done
milestone: Release
crates: [rust-toolchain.toml, crates, apps, bindings, fuzz, tests, scripts, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T1005
---

# T1005: Rust を 1.99.0 に上げる

## 目的

`just tool-updates` (T1004) が、固定している Rust (1.98.1) より新しい 1.99.0 を知らせた。利用者の決定 (2026-10-12): 上げる。

## 受け入れ条件

- [x] `rust-toolchain.toml` と、T1004 の `PINNED_TOOLS` が挙げる写し (ワークフロー・文書など) を 1.99.0 にした (`just tool-updates --list` の写しの検査が通る)
- [x] 新しい clippy の警告・rustfmt の差・非推奨を直した (`-D warnings`)。`#[allow]` で黙らせたものは理由を書いた
- [x] `just ci`・`just wasm-check`・`just nostd-check`・`just cross-check`・`just vcam-check`・`just conformance`・`just godot-check`・`just fuzz-check` (nightly があれば)・`just perf` が通る。perf の値を前と比べてメモに書いた
- [x] mbx のキャッシュと CI の rust-cache の鍵 (T1000) が新しい版で一度外れることを docs に書く必要があれば書いた

## 参考

- T1004 のメモ (`just tool-updates`)、T1000 (rust-cache の鍵)、rust-toolchain.toml、docs/release.md#固定した道具の版

## メモ

- 利用者の決定 (2026-10-12) を受けてオーケストレーターが起票した。
- やったこと:
  - `rust-toolchain.toml` を 1.99.0 にした。`PINNED_TOOLS` の Rust は `mirrors` が無い (写しは無い) ので、`just tool-updates --list` の Rust の行が 1.99.0 になるだけ。
  - 1.99.0 で増えた指摘は 1 つだけ: `AtomicUsize::fetch_update` が非推奨 (`try_update` へ改名) になり、スタジオの API サーバ (`apps/studio/src-tauri/src/api/server.rs`) で `-D warnings` に落ちた。`try_update` に置き換えた。新しい clippy の lint と rustfmt の差は無く、`#[allow]` は足していない。
  - ルートと bindings/godot の `rust-version` (MSRV、1.98) は変えていない。1.99 の新しい機能を使っていないため。
  - docs/release.md の「版を上げる」の Rust の行に、rust-cache の鍵が rustc の版を含むので上げた後の最初の CI は依存をすべてビルドし直すことを書いた (鍵の仕組みは docs/testing.md#ci-のキャッシュに既にある)。手元の mbx も新しい rustc では当たらず、この worktree の最初の `just ci` は 525 misses だった (一度ビルドすれば以後は当たる。docs/workflow/build-cache.md には足していない)。
  - 1.99.0 では手元の rustup にターゲット (aarch64-unknown-none・thumbv7em-none-eabihf・windows・apple など) と rust-src を足す必要があった (`just nostd-check` などが入れ方を表示する)。
- 検査 (1.99.0、手元の Linux): `just ci`・`just wasm-check`・`just nostd-check`・`just cross-check`・`just vcam-check`・`just conformance`・`just godot-check`・`just fuzz-check` (nightly)・`just perf` がすべて通った。Windows・macOS の実機と CI のランナーでは確かめていない (cross-check・vcam-check の clippy だけ)。
- perf (中央値、us。機械は他のワーカーと共有で load average 約 11〜14 のため揺れが大きい。1.98.1 は同じ worktree で前のコミットに戻して測った):

  | 測定 | 1.98.1 | 1.99.0 |
  | --- | --- | --- |
  | native generated update | 69.0 | 46.4 |
  | native generated follow | 66.9 | 50.8 |
  | native generated evaluation only | 56.9 | 44.0 |
  | native avatar update | 21.9 | 22.8 |
  | native avatar follow | 20.3 | 14.5 |
  | wasm generated update | 87.5 | 59.2 |
  | wasm generated follow | 115.2 | 59.0 |
  | wasm avatar update | 29.7 | 17.4 |
  | wasm avatar follow | 31.9 | 18.7 |

  遅くなった兆しは無い (差は負荷の揺れの範囲で、速くなったとは言えない)。上限 (300 / 100 us) には十分に余裕がある。
- 起票の候補: なし。
