---
id: T0829
title: CI で仮想カメラの Windows と macOS のコードを検査する
status: todo
milestone: M9
crates: [justfile, scripts, .github/workflows, docs]
depends_on: [T0816]
assignee:
---

# T0829: CI で仮想カメラの Windows と macOS のコードを検査する

## 目的

`ugoe-studio-vcam` (`apps/studio/vcam`) の `unsafe` の大半 (Windows の Media Foundation と macOS の CoreMediaIO) は
`#[cfg(windows)]`・`#[cfg(target_os = "macos")]` の中にあり、CI (ubuntu の `apps` のグループ) では一度もコンパイルされない。
`clippy::undocumented_unsafe_blocks` (どの `unsafe` にも `// SAFETY:`) と `unsafe_op_in_unsafe_fn` の拒否も、その部分には効いていない
(T0816 のレビューで分かった)。Linux から `--target` を変えて clippy を流せることは確かめてある (`apps/studio/README.md#仮想カメラ`)。

## 受け入れ条件

- [ ] `just` のレシピ (例: `just vcam-check`) で `cargo clippy -p ugoe-studio-vcam --all-targets --target x86_64-pc-windows-msvc -- -D warnings` と
      `--target aarch64-apple-darwin` を流せる (ターゲットが無ければ入れ方を出す)
- [ ] CI のワークフローで同じ検査を流す (ubuntu のまま。ターゲットを `rustup target add` で足す)
- [ ] `apps/studio/README.md#仮想カメラ` と `docs/testing.md` の CI のジョブの説明を更新した
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md、T0816 のメモ
- ADR 0028 の 8. (`unsafe` の例外とその条件)

## メモ
