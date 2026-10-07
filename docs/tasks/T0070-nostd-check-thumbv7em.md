---
id: T0070
title: "just nostd-check: 64 ビットのアトミック操作の無い tier 2 のターゲット (thumbv7em-none-eabihf) でもビルドを確かめる"
status: done
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

- [x] `just nostd-check` (scripts/nostd-check.mjs) が、NOSTD_CRATES と `ugoe-ffi` (静的ライブラリ) を `thumbv7em-none-eabihf` 向けにもビルドする
      (clippy まで流すか、ビルドだけにするかを決めてコメントに書く。C の見本とのリンクは `aarch64-unknown-none` だけでよい)
- [x] `node scripts/nostd-check.mjs targets` が新しいターゲットも出し、CI の rust (runtime) の `rustup target add` で入る (ターゲットが無いときの手元の案内も)
- [x] T0495 の直し (`next_instance_id` の cfg) を一時的に戻すと `just nostd-check` が失敗することを確かめ、メモに書いた (戻した変更はコミットしない)
- [x] scripts/nostd-check.test.mjs を直し、docs/portability.md・crates/ugoe-ffi/README.md#std-の無い環境・docs/testing.md の検査の説明に足した
- [x] `just ci` が通る

## 参考

- T0495 のメモ (32 ビットのターゲットで失敗した・起票の候補)、crates/ugoe-runtime/src/instance.rs (`next_instance_id`)
- scripts/nostd-check.mjs (`NOSTD_TARGET`・`targets`)、docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md

## メモ

- T0068 で起票した (元の候補: T0495 (`just nostd-check` に `thumbv7em-none-eabihf` を足し、`AtomicU64` のような退行を stable の CI で見つける))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- CAS の無いターゲット (`thumbv6m-none-eabi`) は `alloc::sync::Arc` が無くてビルドできない (T0495)。このタスクの範囲の外。

### やったこと (T0070)

- scripts/nostd-check.mjs: `ATOMIC32_TARGET = "thumbv7em-none-eabihf"` と `NOSTD_TARGETS` (`aarch64-unknown-none` と合わせた 2 つ) を足した。
  `run` は NOSTD_CRATES をクレートごとに両方のターゲットで clippy にかけ (ビルドだけでなく clippy にしたのは、
  `cfg(not(target_has_atomic = "64"))` の枝も lint にかけるため。型検査までなので時間はほぼ同じ。理由はコメントに書いた)、
  ugoe-ffi は `thumbv7em-none-eabihf` の静的ライブラリを作る (リンクはしない)。C の見本とのリンクは従来どおり `aarch64-unknown-none` だけ。
  入っていないターゲットはまとめて `rustup target add <...>` の案内を出す (`missingTargets`)。`targets` は 2 行を出す。
- CI の rust (runtime) は `node scripts/nostd-check.mjs targets | xargs rustup target add` のままで新しいターゲットも入るので、
  ステップのコメントだけを直した (.github/workflows/ci.yml。push はしていない。**CI で確かめること:** rust (runtime) の no_std の
  ステップで `thumbv7em-none-eabihf` が入り、clippy と静的ライブラリが通ること)。
- scripts/nostd-check.test.mjs にターゲットの一覧・引数・`missingTargets` のテストを足した。docs/portability.md・
  crates/ugoe-ffi/README.md#std-の無い環境・docs/testing.md (rust (runtime) の行)・justfile のレシピのコメントに書き足した。
- 手元の `just nostd-check` は約 50 秒 (mbx のキャッシュあり)。
- **効き目:** `next_instance_id` の cfg を外して `AtomicU64` だけにすると、`just nostd-check` は
  `ugoe-runtime (thumbv7em-none-eabihf)` の clippy で `error[E0432]: unresolved import core::sync::atomic::AtomicU64` を出して
  失敗した (`aarch64-unknown-none` は通る)。戻した変更はコミットしていない。
- `just ci` は通った。

### やらなかったこと

- `thumbv7em-none-eabihf` の静的ライブラリを C の見本とリンクする検査 (受け入れ条件どおり aarch64 だけ)。ADR 0031 の本文は変えていない
  (検査の追加は README・portability.md に書いた)。
- GitHub Actions の上で流してはいない (上の「CI で確かめること」)。

### 起票の候補

- なし
