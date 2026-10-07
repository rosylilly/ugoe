---
id: T0072
title: "CI: PR ごとに fuzz のターゲットが stable でコンパイルできるかを確かめる"
status: todo
milestone: M11
crates: [.github/workflows, fuzz, justfile, docs]
depends_on: []
assignee:
---

# T0072: CI: PR ごとに fuzz のターゲットが stable でコンパイルできるかを確かめる

## 目的

T0188 で、ワークスペースの外の `fuzz/` (ugoe-fuzz、独自の Cargo.lock) に cargo-fuzz のターゲットを置き、週に 1 度の fuzz.yml で回すようにした。
`fuzz/` はルートのワークスペースの外なので、`ugoe`・`ugoe-format` の公開 API を変えてターゲットが壊れても、PR の CI では気づかず、
次の週の fuzz.yml で初めて落ちる。PR ごとの CI で、stable で `cargo check --manifest-path fuzz/Cargo.toml` (か clippy) を流し、早く気づく。

## 受け入れ条件

- [ ] PR ごとに流れる CI のジョブで、fuzz のクレートを stable でコンパイルする手順を足した (どのジョブに置くか (rust-checks はビルドの
      キャッシュを持たない・rust-group の bindings など) を、時間とキャッシュから決めて理由をコメントに書く。`libfuzzer-sys` の
      ビルドスクリプトが C++ の libFuzzer をコンパイルすることと、`fuzz/Cargo.lock` の `--locked` も考える)
- [ ] 入口を `just` のレシピにするか (`just fuzz-check` の一部だけ、など) を決め、justfile と fuzz/README.md を直した
- [ ] ランタイムの公開 API を一時的に変えると、その手順が失敗することを手元で確かめ、メモに書いた (戻した変更はコミットしない)
- [ ] docs/testing.md#fuzz と CI のジョブの表を直した。push した後の CI で確かめること (時間の延び・キャッシュ) をメモに書いた
- [ ] `just ci` が通る

## 参考

- T0188 のメモ (起票の候補)、docs/adr/0032-fuzzing-with-cargo-fuzz.md、fuzz/run.mjs (`--check`)、.github/workflows/fuzz.yml
- docs/testing.md#fuzz、docs/testing.md#ci-のジョブ

## メモ

- T0068 で起票した (元の候補: T0188 (PR ごとの CI に `cargo check --manifest-path fuzz/Cargo.toml` を足す))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- ci.yml は T0069 (rust (windows)) も触るが、別のジョブなので依存は付けていない。fuzz/ にターゲットを足す T0497 とも依存は付けていない
  (どちらが先でも、後のタスクが新しいターゲットを検査に含める)。
