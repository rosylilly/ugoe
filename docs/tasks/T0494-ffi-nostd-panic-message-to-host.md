---
id: T0494
title: "C ABI: std の無い形で、パニックのメッセージをホストに渡す口を作るかを決める (判断が要る)"
status: in-progress
milestone: M11
crates: [ugoe-ffi, bindings/unity, docs]
depends_on: [T0495]
assignee: claude-worker-T0494
---

# T0494: C ABI: std の無い形で、パニックのメッセージをホストに渡す口を作るかを決める (判断が要る)

## 目的

T0491 で作った `std` の無い `ugoe-ffi` の静的ライブラリ (`src/nostd.rs`) は、パニックの処理で `abort` を呼ぶだけで、どこで何が
起きたかをホストに知らせない (std がある形は `catch_unwind` で `UGOE_ERROR_PANIC` を返す)。ゲーム機・組み込み機器ではデバッガを
つなげないことも多いので、パニックのメッセージと位置をホストのログに渡す口を作るかを決める。lint (T0184・T0491) でパニックは
起きない前提だが、後ろの守りとして調べる手がかりを残す。

**判断が要る** (C ABI の公開の関数を足す。グローバルな状態を持つかどうか)。作業者は下の案から勝手に選ばず、利用者に確かめてから進める。

### 案

- **A. ホストが登録する関数:** `ugoe_set_panic_handler(void (*fn)(const char *message, size_t len, void *user), void *user)` を足す。
  パニックの処理は、登録された関数があればメッセージ (と `file:line`) を固定の大きさのスタックのバッファ (確保しない) に
  `core::fmt::Write` で書いて呼び、その後に `abort` する。登録は `AtomicPtr` に置く (T0489 でエラーのグローバルな状態を無くしたが、
  パニックの処理はもともとプロセスに 1 つ)。std がある形では何もしないか、std のパニックのフックにもつなぐかを決める。`ABI_VERSION` を上げる。
- **B. ホストがリンク時に定義するシンボル:** `ugoe_host_panic(const char *, size_t)` を `extern "C"` で宣言し、ホストが定義する
  (`abort` と同じ扱い)。グローバルな状態は無いが、定義しないとリンクできない (弱いシンボルは Rust の stable に無い)。
- **C. 今のまま (`abort` だけ):** 理由と調べ方 (デバッガ・`panic` の lint) を README に書く。

### 推奨

A。リンク時の要求を増やさず (B は使わないホストにも定義を強いる)、std がある形と同じヘッダで使える。バッファの大きさ
(例: 256 バイト) と切り詰めの扱い、パニックの処理の中でもう一度パニックしないこと (書式の失敗を無視する) を決める。

## 受け入れ条件

- [ ] 利用者に案を示して方針を決め、決めたことと理由を crates/ugoe-ffi/README.md#std-の無い環境 と docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md の追記に書いた
- [ ] A・B なら: 関数 (かシンボル) をヘッダ (cbindgen の生成物と `just refs` の宣言の検査)・Unity の宣言・README に足し、
      `just ffi-check` の std の無い見本 (examples/c/nostd_main.c) で、わざと起こしたパニックのメッセージがホストの関数に届いてから
      abort することを確かめた (見本のテスト用の入口か、テストだけの feature で)
- [ ] `just nostd-check`・`just ffi-check`・`just unity-check`・`just conformance` と `just ci` が通る

## 参考

- T0491 のメモ (起票の候補)、crates/ugoe-ffi/src/nostd.rs、crates/ugoe-ffi/README.md#std-の無い環境
- docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md、docs/portability.md#パニックしない

## メモ

- T0066 で起票した (元の候補: T0491 (std 無しの C ABI でパニックのメッセージをホストに渡す口。今は abort だけ))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- 同じ crates/ugoe-ffi/README.md#std-の無い環境 を書き足す T0495 (判断を待たずに進められる) の後にした。
- 判断 (2026-10-07): 利用者が案 A (ホストが登録する関数 ugoe_set_panic_handler。ABI_VERSION を上げる) を選んだ。
