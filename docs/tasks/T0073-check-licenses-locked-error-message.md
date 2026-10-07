---
id: T0073
title: "check-licenses.mjs: --locked で失敗したとき、スタックトレースでなく古い lock のファイルと直し方を短く出す"
status: done
milestone: M11
crates: [scripts, docs]
depends_on: []
assignee: claude-worker-T0073
---

# T0073: check-licenses.mjs: --locked で失敗したとき、スタックトレースでなく古い lock のファイルと直し方を短く出す

## 目的

`scripts/check-licenses.mjs` の `--fuzz` (と `--godot`) は `cargo metadata --locked` を使う。ランタイムのクレートに依存が増えて
`fuzz/Cargo.lock` や `bindings/godot/Cargo.lock` が古くなると、Node のスタックトレースが出て、何を直せばよいか分かりにくい (T0072 の候補)。

## 受け入れ条件

- [x] `--locked` で lock のファイルが古いために失敗したときは、どの lock のファイルが古いかと直し方 (`cargo update --manifest-path <…>/Cargo.toml --workspace`) を短く出し、終了コード 1 で止まる (スタックトレースを出さない)。ほかの失敗は今までどおり詳しく出す
- [x] スクリプトのテストで、古い lock の場合の文を確かめた
- [x] docs/testing.md (fuzz・godot の検査) の直し方の説明とそろえた
- [x] `just ci` が通る

## 参考

- T0072 のメモ、scripts/check-licenses.mjs、fuzz/README.md

## メモ

- T0072 のワーカーの報告から起票した (2026-10-07)。

### 作業の記録 (claude-worker-T0073)

- **判定と文:** `scripts/lib/cargo-locked.mjs` を足した。cargo の標準エラー出力の「cannot update the lock file <path> because --locked
  was passed」(今の cargo) と「the lock file <path> needs to be updated but --locked was passed」(古い cargo) から lock のパスを取り出し、
  リポジトリのルートからの相対で `error: fuzz/Cargo.lock が古い …` と `直し方: cargo update --manifest-path fuzz/Cargo.toml --workspace …`
  の 2 行を作る。`check-licenses.mjs` の `cargoMetadata` を `spawnSync` にし、lock が古いときはその 2 行を出して終了コード 1。
  ほかの失敗は今までどおり cargo の出力をそのまま出し、例外 (スタック) で落ちる。成功したときの cargo の標準エラー出力も今までどおり出す。
- **テスト:** `scripts/cargo-locked.test.mjs` (`just script-check` に足した)。2 つの文言、fuzz・godot・ルートの lock、ルートの外のパス、
  ほかの失敗。
- **手元で確かめた (一時的な変更は戻した。コミットしていない):** T0072 と同じく `crates/ugoe-core/Cargo.toml` に `base64.workspace = true`
  を足すと、`node scripts/check-licenses.mjs --fuzz`・`--godot`・`just fuzz-lint`・`node bindings/godot/build.mjs --check` がどれも
  上の 2 行だけを出して終了コード 1 で止まった。`fuzz/Cargo.toml` を壊すと (ほかの失敗) cargo の詳しいエラーとスタックが今までどおり出た。
  戻した後に `--fuzz` は通った。
- **範囲の外の小さな変更:** `bindings/godot/build.mjs` (`--check` のライセンスの検査を `cargo build --locked` の前に移し、失敗は
  `process.exit` で止める。前のままだとビルドが先に lock で落ちて、cargo のエラーと Node のスタックが出ていた)、
  `bindings/godot/README.md`・`fuzz/README.md` (直し方の説明)、`justfile` (`script-check` にテストを足した)。
- **文書:** docs/testing.md#fuzz の直し方の説明 (godot も同じことを書いた)、テストの種類の表の `just script-check`。
- `just ci` は通った。

### 起票の候補

- 無し。
