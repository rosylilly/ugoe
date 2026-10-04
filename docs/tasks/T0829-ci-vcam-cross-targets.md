---
id: T0829
title: CI で仮想カメラの Windows と macOS のコードを検査する
status: done
milestone: M9
crates: [justfile, scripts, .github/workflows, docs]
depends_on: [T0816]
assignee: claude-worker-T0829
---

# T0829: CI で仮想カメラの Windows と macOS のコードを検査する

## 目的

`ugoe-studio-vcam` (`apps/studio/vcam`) の `unsafe` の大半 (Windows の Media Foundation と macOS の CoreMediaIO) は
`#[cfg(windows)]`・`#[cfg(target_os = "macos")]` の中にあり、CI (ubuntu の `apps` のグループ) では一度もコンパイルされない。
`clippy::undocumented_unsafe_blocks` (どの `unsafe` にも `// SAFETY:`) と `unsafe_op_in_unsafe_fn` の拒否も、その部分には効いていない
(T0816 のレビューで分かった)。Linux から `--target` を変えて clippy を流せることは確かめてある (`apps/studio/README.md#仮想カメラ`)。

## 受け入れ条件

- [x] `just` のレシピ (例: `just vcam-check`) で `cargo clippy -p ugoe-studio-vcam --all-targets --target x86_64-pc-windows-msvc -- -D warnings` と
      `--target aarch64-apple-darwin` を流せる (ターゲットが無ければ入れ方を出す)
- [x] CI のワークフローで同じ検査を流す (ubuntu のまま。ターゲットを `rustup target add` で足す)
- [x] `apps/studio/README.md#仮想カメラ` と `docs/testing.md` の CI のジョブの説明を更新した
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md、T0816 のメモ
- ADR 0028 の 8. (`unsafe` の例外とその条件)

## メモ

- やったこと
  - `scripts/cross-check.mjs` を足した。定義の正は `CHECKS` (名前・クレート・ターゲット・features)。`run [<名前>...]` は要るターゲットが
    `rustup target list --installed` に無ければ `rustup target add …` の入れ方を出して止まり、あれば各ターゲットで
    `cargo clippy -p … --all-targets --target … -- -D warnings` を流す。`targets` は要るターゲットを出す (CI が使う)、`check` は定義の
    クレートがワークスペースにあるかを見る (`just groups` に足した)、`list` は一覧。純粋な部分は `scripts/cross-check.test.mjs` でテスト
    (`just groups` が実行する)。T0013 などで検査を足すときは `CHECKS` に書くだけで、レシピと CI のジョブに入る。
  - justfile: `just cross-check [<名前>...]` と `just vcam-check` (= `cross-check vcam`)。`just ci` には含めない (godot-check などと同じく
    CI では別のジョブ。手元にターゲットが無い人の `just ci` を落とさないため)。
  - ci.yml: `rust (cross)` のジョブ (ubuntu-latest。`node scripts/cross-check.mjs targets | xargs rustup target add` の後に
    `just cross-check`。rust-cache の shared-key は `cross`、main でだけ保存)。まとめの `rust` のジョブの needs に足した。
  - 文書: docs/testing.md に `rust (cross)` の行と「別のターゲットの検査」の節、apps/studio/README.md#仮想カメラ を `just vcam-check` と
    CI のジョブの説明に直した。範囲外の小さな変更として AGENTS.md のコマンドの表に `just cross-check` / `just vcam-check` の行を足した。
- 確かめたこと
  - この機械 (WSL の Linux) で `just vcam-check` が `x86_64-pc-windows-msvc` と `aarch64-apple-darwin` の両方で通る
    (ターゲットは入っていたので `rustup target add` は不要だった。`node scripts/cross-check.mjs targets | xargs rustup target add` も
    「up to date」で成功)。cmio.rs の `// SAFETY:` を 1 つ消すと `undocumented_unsafe_blocks` で失敗することも確かめた (戻した)。
  - ci.yml は actionlint 1.7.12 (shellcheck 0.9.0 付き) で検査した。足したジョブに指摘は無い (既存の `ls` の SC2012 の info が 2 つ残る)。
- やらなかったこと / 確かめていないこと
  - GitHub Actions の上では動かしていない (push できないため)。ランナーで `rustup target add` が通ること・windows や objc2 の
    クレートのビルドスクリプトが ubuntu で通ることは、手元の Linux で同じコマンドが通ったことからの推測。
  - Windows・macOS でのテストの実行は T0013 の範囲。
- 起票の候補
  - スタジオのバックエンド (`ugoe-studio`) や他のアプリにも `cfg(windows)`・`cfg(target_os = "macos")` のコードがあれば、`CHECKS` に
    足すかを検討する (Tauri のアプリはクロスの check で webkit などのシステムの依存の扱いが要るかもしれない。T0013 で見るとよい)。

