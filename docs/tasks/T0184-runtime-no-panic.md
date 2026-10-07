---
id: T0184
title: ランタイム層でパニックを起こさない (lint と壊れた入力のテスト)
status: in-progress
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

- [ ] ランタイム層のクレートのライブラリのコード (テストを除く) で、`clippy::expect_used`・`clippy::panic`・`clippy::unreachable`・
      `clippy::indexing_slicing` (多すぎるなら外から来る値を添字にする箇所を監査する) などを有効にし、出た箇所を直した。
      本当に起こらないと示せる箇所だけ、理由を書いて `#[expect(...)]` にした
- [ ] 壊れた入力のテストを足した: 切り詰めた・ビットを反転したバンドルと JSON の読み込み、範囲外の番号、巨大な数、NaN と無限大の
      パラメータ・`dt`・トラッキング。パニックせずにエラーか定めた扱い (docs/portability.md#ホストから来る数) になることを確かめる
      (`proptest` などの依存を足すなら PR に理由を書く。stable で動かせる形にする)
- [ ] 算術の桁あふれ (debug ビルドのパニック) を、外から来る大きさ・個数の箇所で監査した (`checked_*`・`saturating_*`)
- [ ] docs/portability.md#ランタイム層が守る制約-移植性のため に「パニックしない」と検査の方法を書いた
- [ ] `just ci` が通る

## 参考

- [ADR 0031](../adr/0031-no-std-runtime-and-host-neutral-c-abi.md)、docs/workflow/code-review.md#m9-で繰り返し見つかった誤り、docs/portability.md#ホストから来る数

## メモ

- 2026-10-06 起票。no_std のタスク (T0179〜T0183) と同じクレートを触るので、並行するならオーケストレーターが順番を決める。
