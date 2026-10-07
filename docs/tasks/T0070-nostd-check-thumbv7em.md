---
id: T0070
title: "just nostd-check: 64 ビットのアトミック操作の無い tier 2 のターゲット (thumbv7em-none-eabihf) でもビルドを確かめる"
status: in-progress
milestone: M11
crates: [scripts, .github/workflows, docs, crates/ugoe-ffi/README.md]
depends_on: []
assignee: claude-worker-T0070
---

# T0070: just nostd-check: 64 ビットのアトミック操作の無い tier 2 のターゲット (thumbv7em-none-eabihf) でもビルドを確かめる

## 目的

T0495 で、32 ビットのターゲット (`armv6k-nintendo-3ds`・`mipsel-sony-psp`、tier 2 の `thumbv7em-none-eabihf`) では `ugoe-runtime` の
`ModelInstance` の番号 (`AtomicU64`) でビルドが失敗することが分かり、`target_has_atomic = "64"` が無ければ `AtomicUsize` で数えるように直した。
`just nostd-check` は `aarch64-unknown-none` (64 ビットのアトミック操作がある) だけを見るので、同じ退行が入っても気づけない。
stable で rustup が配る `thumbv7em-none-eabihf` (32 ビット、CAS はあるが 64 ビットのアトミック操作が無い) でも、std の無い形のビルドを確かめる。

## 受け入れ条件

- [ ] `just nostd-check` (scripts/nostd-check.mjs) が、NOSTD_CRATES と `ugoe-ffi` (静的ライブラリ) を `thumbv7em-none-eabihf` 向けにもビルドする
      (clippy まで流すか、ビルドだけにするかを決めてコメントに書く。C の見本とのリンクは `aarch64-unknown-none` だけでよい)
- [ ] `node scripts/nostd-check.mjs targets` が新しいターゲットも出し、CI の rust (runtime) の `rustup target add` で入る (ターゲットが無いときの手元の案内も)
- [ ] T0495 の直し (`next_instance_id` の cfg) を一時的に戻すと `just nostd-check` が失敗することを確かめ、メモに書いた (戻した変更はコミットしない)
- [ ] scripts/nostd-check.test.mjs を直し、docs/portability.md・crates/ugoe-ffi/README.md#std-の無い環境・docs/testing.md の検査の説明に足した
- [ ] `just ci` が通る

## 参考

- T0495 のメモ (32 ビットのターゲットで失敗した・起票の候補)、crates/ugoe-runtime/src/instance.rs (`next_instance_id`)
- scripts/nostd-check.mjs (`NOSTD_TARGET`・`targets`)、docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md

## メモ

- T0068 で起票した (元の候補: T0495 (`just nostd-check` に `thumbv7em-none-eabihf` を足し、`AtomicU64` のような退行を stable の CI で見つける))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- CAS の無いターゲット (`thumbv6m-none-eabi`) は `alloc::sync::Arc` が無くてビルドできない (T0495)。このタスクの範囲の外。
