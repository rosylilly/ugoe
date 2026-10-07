---
id: T0184
title: ランタイム層でパニックを起こさない (lint と壊れた入力のテスト)
status: done
milestone: M11
crates: [ugoe-core, ugoe-model, ugoe-runtime, ugoe-physics, ugoe-motion, ugoe-format, ugoe, Cargo.toml, docs]
depends_on: []
assignee: claude-worker-T0184
---

# T0184: ランタイム層でパニックを起こさない (lint と壊れた入力のテスト)

## 目的

`std` の無い環境の C ABI は `panic=abort` になり、パニックはアプリ全体を止める ([ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md#ランタイム層ではパニックを起こさない))。
外から来るデータ (ファイル・ホストの引数) のどんな値でもランタイム層がパニックしないことを、守るべき性質として lint とテストで確かめる。

## 受け入れ条件

- [x] ランタイム層のクレートのライブラリのコード (テストを除く) で、`clippy::expect_used`・`clippy::panic`・`clippy::unreachable`・
      `clippy::indexing_slicing` (多すぎるなら外から来る値を添字にする箇所を監査する) などを有効にし、出た箇所を直した。
      本当に起こらないと示せる箇所だけ、理由を書いて `#[expect(...)]` にした
- [x] 壊れた入力のテストを足した: 切り詰めた・ビットを反転したバンドルと JSON の読み込み、範囲外の番号、巨大な数、NaN と無限大の
      パラメータ・`dt`・トラッキング。パニックせずにエラーか定めた扱い (docs/portability.md#ホストから来る数) になることを確かめる
      (`proptest` などの依存を足すなら PR に理由を書く。stable で動かせる形にする)
- [x] 算術の桁あふれ (debug ビルドのパニック) を、外から来る大きさ・個数の箇所で監査した (`checked_*`・`saturating_*`)
- [x] docs/portability.md#ランタイム層が守る制約-移植性のため に「パニックしない」と検査の方法を書いた
- [x] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、docs/workflow/code-review.md#m9-で繰り返し見つかった誤り、docs/portability.md#ホストから来る数

## メモ

- 2026-10-06 起票。no_std のタスク (T0179〜T0183) と同じクレートを触るので、並行するならオーケストレーターが順番を決める。
- 2026-10-07 完了 (claude-worker-T0184)。
  - **lint:** 7 クレート (`ugoe-core`〜`ugoe-format`・`ugoe`) の `lib.rs` で `clippy::expect_used`・`indexing_slicing`・`panic`・
    `string_slice`・`unimplemented`・`unreachable` を有効にした。テストのコードは `clippy.toml` の `allow-indexing-slicing-in-tests`・
    `allow-panic-in-tests` で除く (`MATH_CRATES` の写しも揃えた)。出た約 180 か所は、ほとんどを `get`・`zip`・`iter().take()` に
    書き換え (正しい入力での結果は変えない。physics の golden と `just conformance` は同じ)、残りは理由を書いた `#[expect]`
    (`# Panics` を約束している添字の問い合わせ、`const fn` の中、コンパイルで作った番号だけを使うグラフの探索、固定のフィクスチャ)。
    `debug_assert!` も外した。
  - **見つかって直したもの:** (1) 衝突体の掃引が、区間が衝突体の半径に比べて極端に長い (1e18 px など) と数十億回回って止まらない
    (刻みの下限 `π / 65536` を足した。`docs/spec/physics.md#衝突体` と英語の `spec/physics.md` の式も直した。`spec/` は `crates:` の外だが、
    仕様の同期のための 1 行)。(2) ホストが `Puppet::instance_mut` でパラメータの並びやインスタンスを別のモデルのものに差し替えると、
    `copy_from_slice` と範囲外の `set_part_opacity_override` でパニックした (写しを `clear` + `extend_from_slice` にし、
    `set_part_opacity_override` は範囲外なら何もしないようにした)。(3) キーフォームの `locate` は NaN の値で `0 - 1` の桁あふれ
    (公開の `keyform::weights` に NaN を渡すとパニック)。(4) 灰色の PNG の RGBA の大きさの積が 32bit で桁あふれしうる。
    (5) `Warp::new`・`Chain::step`/`settle`・`ResolvedBinding` などの公開の型に食い違った長さを渡すとパニックした。
  - **テスト:** `crates/ugoe-format/tests/broken_input.rs` (`assets/compat` のバンドルとモジュールのすべての長さの切り詰めと、
    すべてのバイトのビットの反転を読み、画像をデコードし、読めたモデルを評価。fixtures の JSON の切り詰め・1 文字の置き換え・
    数をすべて `1e39`・`1e999`・`4294967296` などにしたもの。深い入れ子)、`crates/ugoe/tests/broken_input.rs` (壊したバンドルと
    モジュールを `Puppet` で動かす。NaN・無限大・`f32::MAX`・`1e18` などのパラメータ・`dt`・トラッキング・リップシンク・受け手の入力・
    当たり判定の点。範囲外の番号と知らない ID。検査していない物理の設定・モーション・表情・エフェクト。評価の状態の差し替え)、
    `ugoe-format` の `image` の単体テスト (壊した KTX2 と PNG)。乱数の fuzz や `proptest` は使わず、依存は足していない
    (決まった入力で stable で動く。2 つの結合テストは debug で 25 秒と 11 秒ほど)。
  - **算術の監査:** `clippy::arithmetic_side_effects` で出た箇所 (浮動小数点を含めて 160 ほど) を見て、外から来る大きさ・個数の箇所を
    確かめた。バンドルの表・チャンクの範囲・展開の上限・KTX2 の大きさ・キーフォームの格子の積 (検証で `checked_mul`) はすでに
    `checked_*`・`saturating_*`。直したのは上の (4) と、掃引の回数 (1)。lint 自体は浮動小数点まで数えて多すぎるので有効にしていない。
  - **文書:** `docs/portability.md#パニックしない` (書き方・パニックしてよい Rust の API の約束・検査の方法)、制約の一覧に 1 行、
    `ホストから来る数` の表の `set_part_opacity_override` の行、`ugoe-format`・`ugoe` の README。
  - **検査:** `just ci`・`just nostd-check`・`just wasm-check`・`just conformance` (c・wasm・wasm-webgpu・avatar・queries・example・
    unity・godot すべて ok)・`just cross-check` が通った。
  - **やらなかったこと:** `ugoe-ffi`・`ugoe-wasm` は `crates:` の外なので lint を有効にしていない。添字を取る問い合わせの API
    (`ParameterValues::get` など) は `# Panics` の約束のまま (値を返せないため。上の層が範囲を確かめる)。
  - **起票の候補:** (a) `ugoe-ffi` (と `ugoe-wasm`) に同じ lint を有効にする (`std` の無い C ABI ではパニックが abort になる。
    T0491 と合わせるとよい)。(b) 添字で引く問い合わせの API (`ParameterValues::get`・`ModelInstance::part_opacity` など) を
    `Option` を返す形にするかの判断 (公開 API の変更)。(c) nightly の `cargo-fuzz` で、壊れた入力のテストを継続的な fuzz に広げるかの判断。

