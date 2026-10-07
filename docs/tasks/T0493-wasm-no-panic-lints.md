---
id: T0493
title: "ugoe-wasm: T0184 のパニックの lint を有効にするかを決め、有効にする (判断が要る)"
status: todo
milestone: M11
crates: [ugoe-wasm, docs]
depends_on: []
assignee:
---

# T0493: ugoe-wasm: T0184 のパニックの lint を有効にするかを決め、有効にする (判断が要る)

## 目的

T0184 はランタイム層の 7 つのクレートで、T0491 は `ugoe-ffi` で、clippy の lint (`expect_used`・`indexing_slicing`・`panic`・
`string_slice`・`unimplemented`・`unreachable`) を有効にした。`ugoe-wasm` は対象外のまま (docs/portability.md#パニックしない に
「ブラウザ向けで `std` があり、パニックは wasm の trap として JavaScript の例外で届く」と書いてある)。
ただ、trap の後の `WebPuppet` は Rust の状態が途中のまま残り、続けて呼ぶと食い違った結果やもう一度の trap になりうる
(`<ugoe-avatar>` やエディタ・ビューアが作り直すまで使えない)。wasm の入口もパニックしないようにするかを決め、するなら lint を有効にする。

**判断が要る** (docs/portability.md に書いた「`ugoe-wasm` は対象外」の方針を変える)。作業者は下の案から勝手に選ばず、利用者に確かめてから進める。

### 案

- **A. 同じ lint を有効にする:** `crates/ugoe-wasm/src/lib.rs` で 6 つを有効にし、出た所を `get`・`?`・JS の例外を返す形に直す
  (テストは `clippy.toml` で除く。ほかのクレートと同じ)。docs/portability.md の「対象外」を書き換える。
- **B. 対象外のまま:** 理由 (trap は JS の例外で届く・ホストが作り直す) を残し、trap の後の `WebPuppet` の扱い (作り直す) を
  bindings/web/README.md に書く。

### 推奨

A。入口はホスト (JS) から来る番号・文字列・バイト列を受けるので、`ugoe-ffi` と同じ規則にそろえるほうが分かりやすく、trap の後の
作り直しに頼らずに済む。出る数は少ないと見ている (今の `ugoe-wasm` は多くを `get` と `?` で書いている。`tracking.rs` の
`index_of(id).unwrap()` などは確かめる)。

## 受け入れ条件

- [ ] 利用者に案を示して方針を決め、決めたことと理由を docs/portability.md#パニックしない と crates/ugoe-wasm/README.md に書いた
- [ ] A なら: 6 つの lint を `ugoe-wasm` で有効にし、出た所を直した (残す `#[expect]` には理由を書いた)。範囲の外の番号や
      知らない ID で JS の例外か既定の値になり、trap しないことを wasm のテスト (`wasm-bindgen-test` か `just conformance --only queries` など) で確かめた
- [ ] `just wasm-check`・`just web-check`・`just conformance` と `just ci` が通る

## 参考

- T0184 のメモ (やらなかったこと・起票の候補 (a))、T0491 のメモ (申し送り・`ugoe-ffi` の lint)
- docs/portability.md#パニックしない、crates/ugoe-wasm/README.md

## メモ

- T0066 で起票した (元の候補: T0184 (`ugoe-wasm` にも T0184 のパニックの lint を有効にする。`ugoe-ffi` は T0491 で済み))。
- M11 の後続 (docs/roadmap.md の M11。M11 の完了の条件は `ugoe-ffi` までで、`ugoe-wasm` は範囲の外)。完了の条件には含めない。
- 添字を取る問い合わせの API を `Option` にするかを決める T0187 は、`ugoe-wasm` の呼び出し側を触るのでこのタスクの後にした。
- 判断 (2026-10-07): 利用者が案 A (同じ lint を有効にする) を選んだ。
