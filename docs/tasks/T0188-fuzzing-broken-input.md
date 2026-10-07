---
id: T0188
title: "壊れた入力のテストを、cargo-fuzz などで続けて回す fuzzing に広げるかを決める (判断が要る)"
status: done
milestone: M11
crates: [ugoe-format, ugoe, .github/workflows, docs]
depends_on: [T0186]
assignee: claude-worker-T0188
---

# T0188: 壊れた入力のテストを、cargo-fuzz などで続けて回す fuzzing に広げるかを決める (判断が要る)

## 目的

T0184 で、壊れた入力のテスト (`crates/ugoe-format/tests/broken_input.rs`・`crates/ugoe/tests/broken_input.rs`) を足した。
`assets/compat` のバンドルの切り詰め・ビットの反転、fixtures の JSON の書き換えなど、決まった入力で stable で動き、依存を足していない
(乱数の fuzz や `proptest` は使っていない)。決まった入力の外にあるパニック・止まらないループ・大きすぎる確保を見つけるために、
続けて回す fuzzing に広げるかを決める。

**判断が要る** (ツールチェインと CI の方針。nightly と新しい依存)。作業者は下の案から勝手に選ばず、利用者に確かめてから進める。

### 案

- **A. cargo-fuzz (libFuzzer、nightly) をワークスペースの外に置く:** `fuzz/` (ワークスペースの外、bindings/godot と同じ扱い) に
  ターゲット (バンドルの読み込み・モジュールの読み込み・画像のデコード・読めたモデルの `Puppet` での評価) を置き、
  `assets/compat` を種の集まりにする。`just ci` には入れず、CI の定期のジョブ (週に 1 度、数分ずつ) と `just fuzz <target>` で回す。
  nightly はそのジョブだけ。見つかった入力は `broken_input.rs` の決まった入力に足す。
- **B. stable の性質のテスト (`proptest`・`arbitrary` など) を開発時の依存に足す:** 普段の `cargo test` で回る。カバレッジを見て
  入力を変える fuzz ほどは深く探れない。テストの時間が延びる。
- **C. 今のまま:** 決まった入力のテストだけ。依存も nightly も増えない。

### 推奨

A。ランタイムは外から来るファイル (`.ugoe`・`.ugoemod`・画像) を読むので、カバレッジを見る fuzz の得が大きい。ワークスペースの外に
置けば `rust-toolchain.toml` (stable) と `just ci` は変わらない。依存 (`libfuzzer-sys` と同梱の libFuzzer) のライセンスは
`just licenses` の対象の外になるので、確かめて ADR か README に書く (配布物には入らない)。

## 受け入れ条件

- [x] 利用者に案を示して方針を決め、決めたことと理由を docs/testing.md (と、新しい依存・ツールチェインなら ADR) に書いた
- [x] 決めた方針で、少なくともバンドルの読み込みとモジュールの読み込みを入力にする fuzz (か性質のテスト) を足し、手元で回した時間と
      見つかったもの (直したなら、その入力を `broken_input.rs` に足したこと) をメモに書いた (C ならこの項目は不要)
- [x] CI で回すなら、ジョブ (定期か手動) を足し、push した後の CI で確かめることをメモに書いた
- [x] `just licenses` と `just ci` が通る

## 参考

- T0184 のメモ (テスト・起票の候補 (c))、docs/portability.md#パニックしない の「検査の方法」
- crates/ugoe-format/tests/broken_input.rs、crates/ugoe/tests/broken_input.rs、docs/testing.md

## メモ

- T0066 で起票した (元の候補: T0184 (nightly の cargo-fuzz で壊れた入力のテストを続けて回すかの判断))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- 同じ `ugoe-format` の開発時の依存とテストの feature を触る T0186 の後にした。
- 判断 (2026-10-07): 利用者が案 A (cargo-fuzz をワークスペースの外に置き、定期の CI ジョブと just fuzz で回す) を選んだ。
- やったこと (2026-10-07):
  - 方針を ADR 0032 (`docs/adr/0032-fuzzing-with-cargo-fuzz.md`) と docs/testing.md#fuzz・docs/portability.md の「検査の方法」に書いた。
  - `fuzz/` (ugoe-fuzz、MIT、`publish = false`、空の `[workspace]` と独自の Cargo.lock) にターゲット `bundle`・`module`・`image`・`json` を置いた。
    種は `assets/compat`・`spec/examples`・`fuzz/seeds/image` (小さな PNG 6 つと KTX2 11 個。どれも正しく読めることを確かめた)。
  - `fuzz/run.mjs` と `just fuzz <target> [秒]`・`just fuzz-check` (ライセンス・整形・clippy は stable、ビルドは nightly)。
  - `scripts/check-licenses.mjs --fuzz` を足した (godot の分岐と同じ関数にまとめた)。`libfuzzer-sys` は `(MIT OR Apache-2.0) AND NCSA`
    (同梱の libFuzzer が NCSA。ソースの見出しは Apache-2.0 WITH LLVM-exception) なので、このクレートにだけ NCSA を認めた。ほかは寛容なライセンス。
  - CI: `.github/workflows/fuzz.yml` (週に 1 度・手動、ターゲットごとに既定 300 秒、corpus を actions/cache で持ち越し、見つかった入力を成果物に上げる)。
  - 範囲外の小さな変更: `AGENTS.md` (構成とコマンドの表)・`justfile`・`biome.json` (fuzz/ を検査に入れた)・`.gitignore`・
    `commitlint.config.mjs` (scope `fuzz`)・`scripts/check-licenses.mjs`。
- 回した時間と見つかったもの (手元、nightly 2026-10-06、1 プロセス):
  - 1 回目 60 秒ずつ: `image` が 1 分以内に `png` 0.18.1 のパニックを見つけた (PLTE の長さが 3 の倍数でないと、パレットの展開で
    範囲外の添字)。`png` の最新 (0.18.1) でも直っていないので、`ugoe_format::image::decode_png` で `read_info` の後に断るようにした
    (`ImageError::Decode`)。最小にした入力を `crates/ugoe-format/tests/broken_input.rs` の `found_by_fuzzing` に足した (直す前に失敗を確認)。
  - 直した後: `image` 120 秒、さらに 4 つとも 180 秒ずつ (bundle 約 160 万回・module 約 230 万回・image 約 690 万回・json 約 280 万回)。何も見つからなかった。
- push した後の CI で確かめること (push していないので未確認):
  - fuzz.yml を手動 (workflow_dispatch、秒数を小さく) で流し、4 つのジョブが通ること。特に: `taiki-e/install-action` の `cargo-fuzz`
    (一覧に無いので cargo-binstall に落ちる) が入ること、`actions/cache/restore@v6`・`save@v6` の版と corpus の持ち越し、
    `Swatinem/rust-cache` の `workspaces: fuzz -> target`、bundle のジョブの `just fuzz-check` (stable の clippy で libFuzzer の C++ をビルドする)。
  - 見つかったときに `fuzz-artifacts-<target>` が上がること。
- 起票の候補:
  - `png` の上流に PLTE の長さの不具合を報告する (直ったら `decode_png` の検査は残してもよい)。
  - PR ごとの CI の `rust-checks` などで `cargo check --manifest-path fuzz/Cargo.toml` だけでも流し、API の変更でターゲットが壊れたことに週を待たずに気づく。
  - `ugoe-ffi` の C ABI の入口 (std の無い形はパニックで abort) を直接入力にするターゲット。
