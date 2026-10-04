---
id: T0013
title: CI で feature の下のコードを lint し、Windows と macOS で Rust のテストを流す
status: in-progress
milestone: M9
crates: [justfile, scripts, .github/workflows, docs]
depends_on: [T0829]
assignee: claude-worker-T0013
---

# T0013: CI で feature の下のコードを lint し、Windows と macOS で Rust のテストを流す

## 目的

M9 のコードレビューの横断の所見 (T0211・T0667・T0816。まとめは docs/workflow/code-review.md#m9-の結果)。CI が見ていないコードに誤りが溜まっていた。

- **feature の下のコード (T0211):** CI の clippy は既定の feature だけを見るので、`gpu-tests` などの feature の下の警告が溜まっていた (T0211 で直した)。
- **Windows と macOS (T0667):** 「大文字小文字だけが違う名前を同じファイルとして消す」誤り (T0667 で直した) は、Linux だけの CI では見つからない。
  Rust のテスト (少なくともファイルを書き出す `ugoe-editor-core`・`ugoe-format`・`ugoe-cli`・`ugoe-mcp`) を Windows と macOS でも流す。
- **アプリの OS ごとのコード (T0816):** 仮想カメラは T0829 で `--target` を変えた clippy を足す。エディタ・ビューア・スタジオの `src-tauri` の
  `#[cfg(windows)]`・`#[cfg(target_os = "macos")]` の中も同じく型検査されていない。

## 受け入れ条件

- [ ] `just lint-group` (と CI の rust-group) が、テストで使う feature (`gpu-tests` など) を有効にした clippy も流す (`scripts/ci-groups.mjs` に feature の組を持たせるなど)
- [ ] Windows と macOS のランナーで、ファイルを扱うクレートの Rust のテストを流すジョブを足した (毎回か夜間か、どのクレートかを決めて docs/testing.md#ci-のジョブ に書く)
- [ ] アプリの `src-tauri` の OS ごとのコードを、T0829 と同じ `--target` の clippy で検査できるかを確かめ、できるものは足し、できないものは理由をメモに書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0211・T0667・T0816 のメモの「横断」、T0829

## メモ
