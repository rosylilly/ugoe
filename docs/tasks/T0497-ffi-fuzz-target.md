---
id: T0497
title: "fuzz: C ABI (ugoe-ffi) の入口に直接入力を渡す fuzz のターゲットを足す (判断が要る)"
status: todo
milestone: M11
crates: [fuzz, ugoe-ffi, .github/workflows, scripts, justfile, docs, AGENTS.md]
depends_on: []
assignee:
---

# T0497: fuzz: C ABI (ugoe-ffi) の入口に直接入力を渡す fuzz のターゲットを足す (判断が要る)

## 目的

T0188 の fuzz のターゲット (`bundle`・`module`・`image`・`json`) は、Rust の API (`ugoe`・`ugoe-format`) に入力を渡す。C ABI (`ugoe-ffi`) は
読み込み (バイト列と長さ)・パラメータの名前 (C の文字列)・番号・数を受け取り、自分で確かめてから Rust の API に渡す。std の無い形では
パニックは `abort` になる (T0491・T0494) ので、C ABI の入口の確かめの漏れを fuzz で探す。

**判断が要る** (`unsafe` の扱い)。C ABI の関数は `unsafe extern "C" fn` で、呼ぶには `unsafe` が要る。`fuzz/Cargo.toml` は
`unsafe_code = "deny"` で、AGENTS.md は `unsafe` を `ugoe-ffi` と仮想カメラのクレートにだけ許している。作業者は下の案から勝手に選ばず、
利用者に確かめてから進める。

### 案

- **A. fuzz のターゲットで `unsafe` を許す:** `fuzz/` (ワークスペースの外・配布しない) を `unsafe` の例外に足し、ターゲットから C ABI の関数を
  直接呼ぶ (C のホストと同じ呼び方)。AGENTS.md と ADR 0032 に例外を書く。
- **B. `ugoe-ffi` に fuzz 用の安全な入口を置く:** `ugoe-ffi` に feature (例: `fuzzing`。ヘッダには出さない。T0494 の `panic-test` と同じ扱い) で
  安全な Rust の関数 (入力のバイト列を読み込み・問い合わせ・解放の列に分けて C ABI の関数を呼ぶ) を足し、ターゲットはそれを呼ぶ。
  `unsafe` は `ugoe-ffi` の中に留まる。
- **C. 足さない:** C ABI の入口は単体テストと適合テストに任せる。

### 推奨

B。`unsafe` の例外を増やさずに済み、入口の列 (読み込み → 問い合わせ → `update` → 解放) を `ugoe-ffi` の中で C のホストと同じ形で回せる。

## 受け入れ条件

- [ ] 利用者に案を示して方針を決め、決めたことと理由をメモと docs/testing.md#fuzz (A なら AGENTS.md と ADR 0032 も) に書いた
- [ ] 決めた方針で、C ABI の読み込み (`.ugoe` のバイト列。モジュールも) と、読めたものへの問い合わせ・パラメータの設定 (番号・名前・NaN と無限大) を
      入力から組み立てるターゲットを足した。std がある形のパニック (`UGOE_ERROR_PANIC` に変わる) も fuzz が見つけられること
      (libfuzzer-sys のパニックのフックが `catch_unwind` より先に `abort` するか) を確かめ、メモに書いた (C ならこの項目は不要)
- [ ] fuzz.yml の matrix・`just fuzz all`・`scripts/check-licenses.mjs --fuzz` (依存が増えたら) に新しいターゲットを入れ、手元で回した時間と
      見つかったもの (直したなら、その入力を `ugoe-ffi` のテストに足したこと) をメモに書いた
- [ ] `just fuzz-check` と `just ci` が通る

## 参考

- T0188 のメモ (起票の候補)、docs/adr/0032-fuzzing-with-cargo-fuzz.md、fuzz/README.md、fuzz/Cargo.toml (`unsafe_code = "deny"`)
- crates/ugoe-ffi/README.md、docs/portability.md#パニックしない

## メモ

- T0068 で起票した (元の候補: T0188 (`ugoe-ffi` の C ABI の入口に直接入力を渡す fuzz のターゲット))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
