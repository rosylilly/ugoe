---
id: T0016
title: テストの一時ディレクトリの手書きの補助を 1 つにまとめる
status: todo
milestone: M9
crates: [Cargo.toml, ugoe-editor-core, ugoe-mcp, ugoe-cli, ugoe-headless, ugoe-psd, apps/editor, apps/viewer, apps/studio, docs]
depends_on: []
assignee:
---

# T0016: テストの一時ディレクトリの手書きの補助を 1 つにまとめる

## 目的

M9 のコードレビューの横断の所見 (T0667・T0669。まとめは docs/workflow/code-review.md#m9-の結果)。
テストが `std::env::temp_dir()` とプロセス ID で一時ディレクトリを作る補助を、ファイルごとに手で書いている
(T0008 の時点で `temp_dir()` を使うファイルは editor-core 8・mcp 10・cli 5・エディタ 8・スタジオ 4・ビューア・headless・psd 各 1)。
名前が重なると並列のテストが不安定になり、失敗すると片付けられずに残る (`mcp_tests.rs` の `TempDir` とも不揃い)。

## 受け入れ条件

- [ ] `tempfile` を dev-dependency (ワークスペースの依存) に入れるか、共有の補助を 1 つ作るかを決めた (`tempfile` を入れるならライセンスを `just licenses` で確かめる)
- [ ] 上のテストの手書きの補助をそれに置き換えた (テストの意味は変えない)
- [ ] docs/testing.md に一時ディレクトリの作り方を書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0667・T0669 のメモの「横断」

## メモ
