---
id: T0016
title: テストの一時ディレクトリの手書きの補助を 1 つにまとめる
status: done
milestone: M9
crates: [Cargo.toml, ugoe-editor-core, ugoe-mcp, ugoe-cli, ugoe-headless, ugoe-psd, apps/editor, apps/viewer, apps/studio, docs]
depends_on: []
assignee: claude-worker-T0016
---

# T0016: テストの一時ディレクトリの手書きの補助を 1 つにまとめる

## 目的

M9 のコードレビューの横断の所見 (T0667・T0669。まとめは docs/workflow/code-review.md#m9-の結果)。
テストが `std::env::temp_dir()` とプロセス ID で一時ディレクトリを作る補助を、ファイルごとに手で書いている
(T0008 の時点で `temp_dir()` を使うファイルは editor-core 8・mcp 10・cli 5・エディタ 8・スタジオ 4・ビューア・headless・psd 各 1)。
名前が重なると並列のテストが不安定になり、失敗すると片付けられずに残る (`mcp_tests.rs` の `TempDir` とも不揃い)。

## 受け入れ条件

- [x] `tempfile` を dev-dependency (ワークスペースの依存) に入れるか、共有の補助を 1 つ作るかを決めた (`tempfile` を入れるならライセンスを `just licenses` で確かめる)
- [x] 上のテストの手書きの補助をそれに置き換えた (テストの意味は変えない)
- [x] docs/testing.md に一時ディレクトリの作り方を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0667・T0669 のメモの「横断」

## メモ

- **決めたこと:** `tempfile` (3 系。MIT OR Apache-2.0) をワークスペースの依存に足し、使うクレートの `[dev-dependencies]` だけに入れた
  (ugoe-editor-core・ugoe-mcp・ugoe-cli・ugoe-headless・ugoe-psd・エディタ・ビューア・スタジオ)。ランタイム層には入れていない。
  理由: 名前が毎回違い (並行の `just ci` で重ならない)、panic しても `Drop` で消える。共有の補助のクレートを作るより小さく、
  すでに Cargo.lock に (ほかの依存の依存として) 入っていたので新しくダウンロードするものは無い。`just licenses` は通った。
- **置き換え:** `std::env::temp_dir()` とプロセス ID (`CARGO_TARGET_TMPDIR` とプロセス ID の `ugoe-cli/tests/cli.rs` も) で作っていた
  補助と、各テストの手での片付け (`remove_dir_all`) を `tempfile::tempdir()` にした。テストの意味は変えていない:
  - まだ無いパスを渡していたテスト (保存先のプロジェクトのディレクトリ・設定のディレクトリなど) は `tmp.path().join("…")` にした。
  - 正規化したパスが要る `TempDir` (ugoe-mcp の `root.rs`・`host.rs`・`tests/editor.rs`、エディタの `mcp_tests.rs`) は、
    `tempfile::TempDir` を持つ小さな構造体に置き換えた。スタジオの `Dir` は `tempfile::TempDir` を包むだけにした。
  - テストの途中で消すこと自体を確かめているもの (エディタの `project.rs` の「外で消された」) は消す行を残した。
  - 置き換えで、名前の重なり (ugoe-mcp の `root.rs` の `layout("missing")` が 2 つのテストで同じ名前、スタジオの `api/store.rs` と
    `api/tests.rs` が同じ接頭辞 `ugoe-studio-api-`) と、片付けていなかったテスト (`api/store.rs`・`cli.rs` など) も直った。
- **置き換えなかったもの:**
  - 目で確かめるために絵を残す描画テスト (`ugoe-cli/tests/{m2,m3,lighting}_render_gpu.rs` の `temp_dir()/ugoe-*-render`、
    `render_gpu.rs`・`atlas_render_gpu.rs` の `CARGO_TARGET_TMPDIR`)。docs/testing.md に例外として書いた。
  - 一時ディレクトリを作らずに「あるディレクトリ」や「パスの計算」に `temp_dir()` を使っているもの
    (ugoe-headless `animate.rs`・ugoe-mcp `autorig.rs` の「通常のファイルでない」の確認、エディタの `project.rs`・`dress_up.rs` のパスの計算)。
  - `ugoe-cli/tests/cli.rs` の決まった名前の 1 ファイル (`single_quad.json`・書かれない `never.png`)。

