---
id: T0188
title: "壊れた入力のテストを、cargo-fuzz などで続けて回す fuzzing に広げるかを決める (判断が要る)"
status: todo
milestone: M11
crates: [ugoe-format, ugoe, .github/workflows, docs]
depends_on: [T0186]
assignee:
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

- [ ] 利用者に案を示して方針を決め、決めたことと理由を docs/testing.md (と、新しい依存・ツールチェインなら ADR) に書いた
- [ ] 決めた方針で、少なくともバンドルの読み込みとモジュールの読み込みを入力にする fuzz (か性質のテスト) を足し、手元で回した時間と
      見つかったもの (直したなら、その入力を `broken_input.rs` に足したこと) をメモに書いた (C ならこの項目は不要)
- [ ] CI で回すなら、ジョブ (定期か手動) を足し、push した後の CI で確かめることをメモに書いた
- [ ] `just licenses` と `just ci` が通る

## 参考

- T0184 のメモ (テスト・起票の候補 (c))、docs/portability.md#パニックしない の「検査の方法」
- crates/ugoe-format/tests/broken_input.rs、crates/ugoe/tests/broken_input.rs、docs/testing.md

## メモ

- T0066 で起票した (元の候補: T0184 (nightly の cargo-fuzz で壊れた入力のテストを続けて回すかの判断))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- 同じ `ugoe-format` の開発時の依存とテストの feature を触る T0186 の後にした。
