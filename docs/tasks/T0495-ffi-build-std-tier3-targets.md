---
id: T0495
title: "C ABI: -Zbuild-std が要るターゲット (tier 3 のゲーム機向けなど) で std の無い静的ライブラリを作る手順を確かめる"
status: done
milestone: M11
crates: [ugoe-ffi, docs]
depends_on: []
assignee: claude-worker-T0495
---

# T0495: C ABI: -Zbuild-std が要るターゲット (tier 3 のゲーム機向けなど) で std の無い静的ライブラリを作る手順を確かめる

## 目的

T0491 の `just nostd-check` は `aarch64-unknown-none` (tier 2。`core`・`alloc` が rustup で配られる) で `std` の無い `ugoe-ffi` の
静的ライブラリを作ってリンクを確かめる。ゲーム機などのターゲットは tier 3 のことが多く、`core`・`alloc` が配られないので、
nightly の `-Zbuild-std=core,alloc` で作る必要がある。その手順が通るか (依存のクレートが tier 3 でもビルドできるか、
`panic = "abort"` のプロファイルとの組み合わせ) を確かめ、README に書く。

## 受け入れ条件

- [x] SDK が無くても rustc が知っている tier 3 のターゲットを 1 つ以上選び (例: `aarch64-nintendo-switch-freestanding`・
      `armv7a-none-eabihf` など。選んだ理由をメモに書く)、nightly の `cargo +nightly build -p ugoe-ffi --no-default-features
      --profile nostd --target <ターゲット> -Zbuild-std=core,alloc` が通ることを確かめた。通らなければ、原因 (依存のクレート・
      ターゲットの機能) と回避をメモに書いた
- [x] 作った静的ライブラリの未解決のシンボルが `just nostd-check` と同じ (`abort`・`aligned_alloc`・`calloc`・`free`・`malloc`・`realloc`) か、
      ほかに要るもの (`memcpy` などのコンパイラの組み込み) があるかを `nm` などで確かめ、メモに書いた
- [x] 手順 (nightly の入れ方・`rust-src`・`-Zbuild-std` の引数・`panic_immediate_abort` を使うか) を crates/ugoe-ffi/README.md#std-の無い環境 に書いた。
      docs/portability.md のターゲット別の表に tier 3 の扱いを書いた
- [x] 実機・ゲーム機の SDK で確かめていないことをメモに書いた
- [x] `just ci` が通る

## 参考

- T0491 のメモ (確かめていないこと・起票の候補)、scripts/nostd-check.mjs (リンクの検査)、crates/ugoe-ffi/README.md#std-の無い環境
- docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md、https://doc.rust-lang.org/cargo/reference/unstable.html#build-std

## メモ

- T0066 で起票した (元の候補: T0491 (`-Zbuild-std` が要るゲーム機向けのターゲットでのビルド手順の確認))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- 検査を `just nostd-check` に入れるかは、nightly が要るので入れない前提 (入れたくなったら起票の候補にする)。

### やったこと (T0495)

- nightly を `rustup update nightly` で 1.101.0-nightly (2026-10-06) に上げ、`rust-src` を入れた (手元の nightly 1.95 はルートの
  `rust-version = "1.98"` で拒まれる)。rust-toolchain.toml は変えていない。
- **選んだターゲットと理由:** `aarch64-nintendo-switch-freestanding` (tier 3。今のゲーム機と同じ 64 ビットの AArch64 で、SDK 無しで
  rustc が知っている。`os = "horizon"`)。32 ビットの例として `armv6k-nintendo-3ds`・`armv7-sony-vita-newlibeabihf`・`mipsel-sony-psp`
  (いずれも tier 3) も試した。`armv7a-none-eabihf` は今の Rust では tier 2 (rustup で配られる) なので選ばなかった。
- `cargo +nightly build -p ugoe-ffi --no-default-features --profile nostd --target aarch64-nintendo-switch-freestanding -Zbuild-std=core,alloc`
  は通る (cdylib のリンクで `cannot find entry symbol _start` の警告が出るだけ)。README には `cargo rustc --crate-type staticlib` の形を書いた。
- **32 ビットのターゲットで失敗した (範囲外の修正):** `armv6k-nintendo-3ds`・`mipsel-sony-psp` は 64 ビットのアトミック操作が無く、
  `ugoe-runtime` の `ModelInstance` の番号 (`AtomicU64`) でビルドが失敗した。tier 2 の `thumbv7em-none-eabihf` でも同じく失敗する
  (stable で確かめた)。`ugoe-runtime/src/instance.rs` に `next_instance_id` を作り、`target_has_atomic = "64"` が無ければ
  `AtomicUsize` で数えるようにした (公開 API の型は u64 のまま。番号は 2^32 回で一巡すると doc に書いた)。直した後は 3 つの 32 ビットの
  tier 3 と `thumbv7em-none-eabihf` (stable) で通る。`crates:` の外 (ugoe-runtime) だが、数行の cfg なので行ってここに書いた。
- CAS の無いターゲット (`thumbv6m-none-eabi`) は `alloc::sync::Arc` が無くてビルドできない (直していない。README に「要る」と書いた)。
- **未解決のシンボル (C の見本 `examples/c/nostd.c` と rust-lld でリンク、と `llvm-nm`):**
  - `-Zbuild-std=core,alloc` だけ: `abort`・`aligned_alloc`・`calloc`・`free`・`malloc`・`realloc` に加えて `memcpy`・`memmove`・`memset`・`memcmp`
    (と、nm では `strlen` も。リンクでは `--gc-sections` で消える)。`-Zbuild-std` の `compiler_builtins` は既定で mem の関数を持たないため。
  - `-Zbuild-std-features=compiler-builtins-mem` を足すと、`just nostd-check` と同じ 6 つだけ (Switch・3DS・Vita・PSP)。形だけのホストの
    関数 (`scripts/nostd-check.mjs` の `hostStubSource`) とのリンクも Switch・3DS・Vita で通った。PSP は C の見本の浮動小数点の ABI
    (`-msingle-float`) を clang の引数で合わせられず、リンクまでは確かめていない (ライブラリの側の問題ではない)。
  - `rust_eh_personality` は残らない (`panic = "abort"` で core をビルドし直すため)。
- **panic_immediate_abort:** 今の nightly では `-Zbuild-std-features=panic_immediate_abort` はエラーになり、パニックの戦略
  `immediate-abort` (`-Zpanic-immediate-abort --config 'profile.nostd.panic="immediate-abort"'`) に変わった。これで通り、未解決のシンボルから
  `abort` が消え、静的ライブラリは 3% ほど小さくなる。`#[panic_handler]` (ホストの `abort`) を通らなくなるので、README では使わないと書いた。
- crates/ugoe-ffi/README.md#std-の無い環境 に小見出し「core と alloc が配られないターゲット (tier 3。T0495)」を足し、docs/portability.md の
  ターゲット別の表のゲーム機の行に tier 3 の扱いを書いた。
- `just ci`・`just wasm-check`・`just nostd-check` が通る。

### 確かめていないこと

- ゲーム機の実機と SDK のツールチェイン (SDK のリンカ・C ライブラリ・スタートアップ・TLS・スレッド・メモリの設定・浮動小数点の設定)。
  確かめたのは nightly でのビルドと、rust-lld でのリンク (形だけのホストの関数) までで、動かしてはいない。
- SDK が独自に配る Rust のツールチェインや、`aarch64-nintendo-switch-freestanding` 以外の実在のゲーム機のターゲット (SDK と NDA の向こうにあるもの)。
- PSP の C の見本とのリンク (上記)。

### 起票の候補

- `just nostd-check` に 32 ビット・64 ビットのアトミック操作の無い tier 2 のターゲット (`thumbv7em-none-eabihf`) を足し、今回の
  `AtomicU64` のような退行を stable の CI で見つける (scripts/nostd-check.mjs と CI の `rustup target add`)。
- (任意) nightly の `-Zbuild-std` の検査を、定期実行の CI (nightly が壊れても PR を止めない形) で流す。
