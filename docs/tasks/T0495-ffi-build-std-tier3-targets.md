---
id: T0495
title: "C ABI: -Zbuild-std が要るターゲット (tier 3 のゲーム機向けなど) で std の無い静的ライブラリを作る手順を確かめる"
status: in-progress
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

- [ ] SDK が無くても rustc が知っている tier 3 のターゲットを 1 つ以上選び (例: `aarch64-nintendo-switch-freestanding`・
      `armv7a-none-eabihf` など。選んだ理由をメモに書く)、nightly の `cargo +nightly build -p ugoe-ffi --no-default-features
      --profile nostd --target <ターゲット> -Zbuild-std=core,alloc` が通ることを確かめた。通らなければ、原因 (依存のクレート・
      ターゲットの機能) と回避をメモに書いた
- [ ] 作った静的ライブラリの未解決のシンボルが `just nostd-check` と同じ (`abort`・`aligned_alloc`・`calloc`・`free`・`malloc`・`realloc`) か、
      ほかに要るもの (`memcpy` などのコンパイラの組み込み) があるかを `nm` などで確かめ、メモに書いた
- [ ] 手順 (nightly の入れ方・`rust-src`・`-Zbuild-std` の引数・`panic_immediate_abort` を使うか) を crates/ugoe-ffi/README.md#std-の無い環境 に書いた。
      docs/portability.md のターゲット別の表に tier 3 の扱いを書いた
- [ ] 実機・ゲーム機の SDK で確かめていないことをメモに書いた
- [ ] `just ci` が通る

## 参考

- T0491 のメモ (確かめていないこと・起票の候補)、scripts/nostd-check.mjs (リンクの検査)、crates/ugoe-ffi/README.md#std-の無い環境
- docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md、https://doc.rust-lang.org/cargo/reference/unstable.html#build-std

## メモ

- T0066 で起票した (元の候補: T0491 (`-Zbuild-std` が要るゲーム機向けのターゲットでのビルド手順の確認))。
- M11 の後続 (docs/roadmap.md の M11)。M11 の完了の条件には含めない。
- 検査を `just nostd-check` に入れるかは、nightly が要るので入れない前提 (入れたくなったら起票の候補にする)。
