# 0032: 外から来る入力を cargo-fuzz (libFuzzer、nightly) で続けて fuzz する。ワークスペースの外に置き、`just ci` に入れない

- 状態: Accepted
- 日付: 2026-10-07
- 関連: T0188、T0184 (壊れた入力のテスト)、ADR 0018 (ライセンス)、ADR 0026 (ワークスペースの外のクレート)、
  [portability.md](../portability.md#パニックしない)、[testing.md](../testing.md#fuzz)

## 背景

ランタイムは外から来るファイル (`.ugoe`・`.ugoemod`・テクスチャの PNG と KTX2・JSON 形式) を読む。T0184 で、壊れた入力の
テスト (`crates/ugoe-format/tests/broken_input.rs`・`crates/ugoe/tests/broken_input.rs`) を足した。`assets/compat` のバンドルの
切り詰め・ビットの反転、fixtures の JSON の書き換えなど、決まった入力で stable で動く。決まった入力の外にあるパニック・止まらない
ループ・大きすぎる確保は見つけられない。

利用者の決定 (2026-10-07): T0188 の案 A (cargo-fuzz をワークスペースの外に置き、定期の CI ジョブと `just fuzz` で回す) を選んだ。

## 決定

1. **fuzz のターゲットを、ルートのワークスペースに入らないクレート `fuzz/` (`ugoe-fuzz`、MIT、`publish = false`) に置く。**
   - `fuzz/Cargo.toml` に空の `[workspace]` を書き、独自の `Cargo.lock` を持つ (ADR 0026 の `bindings/godot` と同じ扱い)。
     `ugoe` (参照レンダラ無し) と `ugoe-format` (`png`) にパスで依存する。
   - ターゲットは `bundle` (バンドルの読み込み・画像のデコード・`Puppet` での評価)、`module` (モジュールの読み込み・着せ替えの
     本体への取り付けと評価)、`image` (PNG と KTX2 のデコード)、`json` (JSON 形式のモデル・モジュール・物理・モーション・表情)。
   - 種は `assets/compat` (バンドルとモジュール)・`spec/examples` (JSON)・`fuzz/seeds/image` (小さな PNG と KTX2) を読むだけにし、
     育てた入力 (`fuzz/corpus/`) と見つかった入力 (`fuzz/artifacts/`) は git に入れない。
2. **nightly は fuzz のビルドだけで使う。** `rust-toolchain.toml` (stable) は変えず、`fuzz/run.mjs` が `cargo +nightly fuzz` を呼ぶ
   (`UGOE_FUZZ_TOOLCHAIN` で変えられる)。整形と clippy (`node fuzz/run.mjs --check`) はルートの stable で行う。
3. **`just ci` には入れない。** 手元では `just fuzz <target> [秒]`、CI では定期のワークフロー `.github/workflows/fuzz.yml`
   (週に 1 度と手動。ターゲットごとに数分) で回す。PR ごとの CI は fuzz のクレートをビルドしないので、ランタイムの API の変更で
   ターゲットが壊れても、気づくのは定期のジョブか `just fuzz-check` になる。
4. **見つかった入力は、最小にして `broken_input.rs` の決まった入力に足す。** 直した後も stable の `just ci` で再発を止めるため。
5. **依存のライセンスは `scripts/check-licenses.mjs --fuzz` で別に検査する** (`just licenses` の対象の外なので)。許すのは
   寛容なライセンス (ルートの検査と同じ集合) と、`libfuzzer-sys` に限った NCSA。
   - `libfuzzer-sys` (0.4) のライセンスは `(MIT OR Apache-2.0) AND NCSA`。Rust の部分が MIT か Apache-2.0、同梱の libFuzzer
     (LLVM の C++ のソース、`libfuzzer/` の下) が NCSA (University of Illinois/NCSA Open Source License。BSD に近い寛容な
     ライセンス) と書かれている。同梱のソースの見出しは LLVM の今のライセンス (Apache-2.0 WITH LLVM-exception) で、どちらも
     寛容でコピーレフトは無い。
   - 依存の `arbitrary` (MIT OR Apache-2.0)・`cc`・`jobserver` なども寛容なライセンスで、検査が確かめる。
   - fuzz のバイナリは配布物 (crates.io のクレート・アプリ・バインディング) に入らない。`ugoe-fuzz` は `publish = false` で、
     `crates/` のクレートは fuzz のクレートに依存しない。

## 検討した代替案

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| B. stable の性質のテスト (`proptest`・`arbitrary`) を開発時の依存に足す | 普段の `cargo test` で回る。nightly が要らない | カバレッジを見て入力を変えないので、バンドルのような入れ子の形式の奥まで届きにくい。テストの時間が延びる |
| C. 今のまま (決まった入力のテストだけ) | 依存も nightly も増えない | 決まった入力の外の誤りは見つからない (T0188 の最初の 1 分で、`png` のパレットの展開のパニックが見つかった) |
| fuzz のクレートをワークスペースに入れる | ビルドと検査が 1 つにまとまる | libFuzzer の C++ のビルドが `just ci` に入り、ルートの `Cargo.lock` に fuzz の依存が入る。`cargo fuzz` は nightly の計装を前提にする |
| `rust-toolchain.toml` を nightly にする | 設定が 1 つになる | ワークスペース全体のビルドが不安定なツールチェインに依存する |
