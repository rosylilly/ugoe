# ugoe-fuzz

外から来る入力 (`.ugoe`・`.ugoemod`・テクスチャの PNG と KTX2・JSON 形式) の fuzz のターゲット。
[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer) で、パニック・止まらないループ・大きすぎる確保を探す。
方針は [ADR 0032](../docs/adr/0032-fuzzing-with-cargo-fuzz.md)、使い方の全体は [docs/testing.md](../docs/testing.md#fuzz)。

ルートのワークスペースの外にあり (空の `[workspace]` と独自の `Cargo.lock`)、`just ci` には入らない。
PR ごとの CI (ci.yml の `rust (runtime)`) は、stable でできる検査 (`just fuzz-lint`: 依存のライセンス・整形・clippy) を流す。
`ugoe`・`ugoe-format` の公開 API を変えてターゲットが壊れたら、週に 1 度の fuzz.yml を待たずに PR で気づく。

## ターゲット

| ターゲット | 入力 | 種 |
| --- | --- | --- |
| `bundle` | `read_bundle` → テクスチャと法線マップのデコード → `Puppet` (物理・モーション・表情・極端な `dt`・当たり判定) | `assets/compat` |
| `module` | `read_module` → テクスチャのデコード → 着せ替えの本体 (`assets/compat/dev/dress-up-base.ugoe`) への取り付けと評価 | `assets/compat` |
| `image` | 同じ入力を PNG と KTX2 の両方としてデコード (乗算済みアルファまで) | `fuzz/seeds/image` |
| `json` | 同じ入力を JSON 形式のモデル・モジュール・物理・モーション・表情として読み、読めたものを `Puppet` で動かす | `spec/examples` |

読めたものを使い切る処理は `src/lib.rs` で共有する (`crates/*/tests/broken_input.rs` と同じ流れ)。

## 使い方

nightly のツールチェインと cargo-fuzz が要る:

```sh
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked
```

```sh
just fuzz bundle          # 60 秒
just fuzz all 300         # すべてのターゲットを 300 秒ずつ
just fuzz image 600 -- -jobs=4 -workers=4   # `--` の後ろは libFuzzer に渡す
just fuzz-check           # 依存のライセンス・整形・clippy・すべてのターゲットのビルド
just fuzz-lint            # fuzz-check のうち stable でできるもの (ターゲットのビルド以外)。nightly と cargo-fuzz は要らない
```

- `just fuzz-lint` は nightly も cargo-fuzz も要らないが、`libfuzzer-sys` のビルドスクリプトが同梱の libFuzzer (C++) を
  コンパイルするので C++ のコンパイラが要る。依存のライセンスの検査 (`cargo metadata`) と clippy は `--locked` で流すので、
  ランタイムのクレートに依存を足して `fuzz/Cargo.lock` が古くなると失敗する。そのときは `cargo update --manifest-path fuzz/Cargo.toml --workspace` で直す
  (ほかの依存の版は変えない)。

- 育てた入力は `fuzz/corpus/<target>/`、見つかった入力は `fuzz/artifacts/<target>/` に残る (どちらも git の管理外)。
- 1 つの入力に 10 秒を超えたら止まらないループとみなす (`-timeout=10`)。メモリの上限は libFuzzer の既定 (2048 MB)。
- 再現は `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<ファイル>`、最小にするのは `cargo +nightly fuzz tmin <target> <ファイル>`。
- 直したら、最小にした入力を `crates/ugoe-format/tests/broken_input.rs` (読み込みとデコード) か `crates/ugoe/tests/broken_input.rs`
  (`Puppet`) の決まった入力に足す (stable の `just ci` で再発を止めるため)。

## 種

- `seeds/image/`: 小さな PNG (RGBA・RGB・灰色・灰色とアルファ・パレットと tRNS・16bit) と KTX2 (RGBA8・RGB8・zstd の超圧縮・BC1〜BC5・BC7)。
  どれも `decode_image_premultiplied` で読める正しいファイル。
- `assets/compat` と `spec/examples` は読むだけで、書き換えない。

## ライセンス

このクレートは MIT。依存のライセンスは `just licenses` の対象の外なので、`scripts/check-licenses.mjs --fuzz` (`just fuzz-check`・`just fuzz-lint`) で検査する。

- `libfuzzer-sys`: `(MIT OR Apache-2.0) AND NCSA`。同梱の libFuzzer (LLVM の C++ のソース) が NCSA (寛容なライセンス。
  ソースの見出しは Apache-2.0 WITH LLVM-exception)。検査はこのクレートにだけ NCSA を認める。
- ほかの依存 (`arbitrary`・`cc` など) は寛容なライセンス。
- fuzz のバイナリは配布物に入らない (`publish = false`。`crates/` のクレートはこのクレートに依存しない)。
