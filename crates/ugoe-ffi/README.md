# ugoe-ffi

**層:** L5 バインディング (依存してよいクレートは `scripts/check-layers.mjs` を参照)

C ABI。Unity・Unreal・Godot などから使う。規約は [docs/portability.md](../../docs/portability.md#c-abi-ugoe-ffi-の規約)。

## 守ること

- `unsafe` はこのクレートでだけ許可する
- パニックを FFI の境界の外へ漏らさない (公開する関数はすべて `error::guard` を通す)。そのうえで、ランタイム層と同じく
  パニックそのものを起こさないように書く (`clippy::indexing_slicing` などを有効にしている。[docs/portability.md#パニックしない](../../docs/portability.md#パニックしない))。
  `std` の無い形では、パニックはアプリ全体を止めるため ([std の無い環境](#std-の無い環境))
- `std` に依存しない (`core` と `alloc` で書く)。`std` の有無で関数の集合とヘッダを変えない ([std の無い環境](#std-の無い環境))
- 数学関数は `ugoe::core::math` を使う (std の `f32::sqrt` などは clippy.toml で禁じている。ADR 0031)
- ABI を壊したら `ABI_VERSION` を上げる ([ABI の版](#abi-の版))
- 公開する関数・型を変えたら `just ffi-header` でヘッダを作り直してコミットする
- ホストから来る数 (NaN・無限大・負の `dt`) は、前提を `ugoe::Puppet` と `ugoe-runtime` に任せ、受け付けない値を `UGOE_ERROR_INVALID_ARGUMENT` で
  知らせるだけにする。入口ごとの表は [docs/portability.md#ホストから来る数](../../docs/portability.md#ホストから来る数)

## 使い方

ヘッダは [`include/ugoe.h`](include/ugoe.h)、ライブラリは `cargo build -p ugoe-ffi --release` で
`target/release/` にできる `libugoe_ffi.{a,so,dylib}` (Windows では `ugoe_ffi.{lib,dll}`)。
静的ライブラリをリンクするときは、Linux ではさらに `-lm -lpthread -ldl` が要る。

```c
UgoeError *err = ugoe_error_create();
UgoePuppet *puppet = NULL;
if (ugoe_puppet_create_from_bundle(bytes, len, &puppet, err) != UGOE_OK) {
    fprintf(stderr, "%s\n", ugoe_error_message(err));
}
ugoe_puppet_set_parameter(puppet, index, value, err);
ugoe_puppet_update(puppet, dt, NULL);  /* 結果コードだけで足りるなら NULL */
UgoeFrame frame;
ugoe_puppet_frame(puppet, &frame, err);  /* このフレームの状態をまとめて借りる。次の update まで有効 */
ugoe_puppet_destroy(puppet);
ugoe_error_destroy(err);
```

動く見本は [`examples/c/sample.c`](examples/c/sample.c) (`just ffi-check` でビルドし、`assets/samples/avatar.ugoe`・
法線マップ付きの `avatar-lit.ugoe`・着せ替えの `dress-up/` (本体とモジュール) で実行する)。

- 失敗しうる関数は `UgoeResult` (`UGOE_OK` = 0) を返し、最後の引数 `UgoeError *error` に失敗の中身を書く
  ([失敗の受け取り方](#失敗の受け取り方-t0489))。
- 内部でパニックが起きたら `UGOE_ERROR_PANIC` を返す。そのオブジェクトの状態は不定なので破棄する
  (`std` の無い形ではホストの `abort` を呼ぶ。[std の無い環境](#std-の無い環境))。
- 毎フレームの描画の状態 (頂点位置・表示・不透明度・色・描画順など) は 1 回の呼び出しでまとめて受け取る。内部のバッファの借用
  (`ugoe_puppet_frame`) と、ホストのバッファへのコピー (`ugoe_puppet_copy_frame`) の両方がある ([毎フレームの描画の状態](#毎フレームの描画の状態-t0490))。
- テクスチャはエンコードされたままのバイト列 (PNG・KTX2) を渡す。デコードと乗算済みアルファへの変換はホストが行う
  ([docs/spec/rendering.md](../../docs/spec/rendering.md))。エンジンが読めない形式 (KTX2 の BCn など) は、参照と同じデコーダ
  `ugoe_image_decode` で RGBA8 にできる ([テクスチャのデコード](#テクスチャのデコード-t0466))。
- 1 つの `UgoePuppet` を複数のスレッドから同時に使ってはならない。

## 失敗の受け取り方 (T0489)

失敗の中身 (メッセージ・種類・補足の値) は、呼び出し側が渡す不透明なオブジェクト `UgoeError` に書く
([ADR 0031](../../docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md#c-abi-のエラーはスレッドローカルに置かない))。
C ABI はグローバルな状態もスレッドローカルな状態も持たない。

```c
UgoeError *err = ugoe_error_create();
if (ugoe_puppet_detach_module(puppet, "Hat", err) != UGOE_OK) {
    const char *kind = ugoe_error_kind(err);           /* "not_attached" など。種類を持たない失敗なら NULL */
    printf("%s: %s\n", kind ? kind : "-", ugoe_error_message(err));
    for (size_t i = 0; i < ugoe_error_field_count(err); i++) {
        printf("  %s = %s\n", ugoe_error_field_key_at(err, i), ugoe_error_field_value_at(err, i));
    }
}
ugoe_error_destroy(err);
```

- **引数の形:** `UgoeResult` を返す関数は、すべて **最後の引数** に `UgoeError *error` を取る (名前も `error` で揃える)。
  NULL なら中身を書かない (結果コードだけで足りるとき)。`void` を返す破棄の関数 (`ugoe_puppet_destroy`・`ugoe_module_destroy`・
  `ugoe_image_destroy`) と、失敗しない関数 (`ugoe_abi_version`・`ugoe_version`・`*_default`・`ugoe_error_*`) は取らない。
- **成功したときは中身を変えない。** 前の失敗の中身が残るので、失敗したかどうかは結果コードで判断する (毎フレームの呼び出しで書き込みをしないため)。
- **読み出し:** `ugoe_error_message(e)` (メッセージ。まだ失敗を書いていなければ NULL)・`ugoe_error_kind(e)` (機械が読む種類。wasm と同じ `code`)・
  `ugoe_error_field(e, key)` (補足の値)。補足の値は `ugoe_error_field_count(e)` と `ugoe_error_field_key_at(e, i)`・
  `ugoe_error_field_value_at(e, i)` で、キーを知らなくてもすべてを列挙できる (新しい版でキーが増えても、ホストにキーの一覧を写さずに済む。T0464)。
  読み出しの関数には NULL の `UgoeError` を渡してもよい (NULL か 0 が返る)。
- 返る文字列は、その `UgoeError` に次の失敗を書くか、破棄するまで有効。
- **スレッド:** `UgoeError` はスレッドに結び付かないので、どのスレッドで作り、書かせ、読んでもよい。呼び出しの間で OS のスレッドが変わる
  ホスト (Go の goroutine・C# の async の続き) でも、失敗した呼び出しに渡した `UgoeError` から正しい中身を読める。ただし 1 つの `UgoeError` を
  同時に複数のスレッドから使ってはならない (呼び出しごとか、スレッドごとに 1 つ持つ)。
- 内部でパニックが起きたとき (`UGOE_ERROR_PANIC`) も、パニックのメッセージを `UgoeError` に書く。

## ヘッダの生成

ヘッダは [cbindgen](https://github.com/mozilla/cbindgen) の CLI で生成する (設定は `cbindgen.toml`)。
cbindgen は MPL-2.0 なので、このクレートの依存 (`build-dependencies`・`dev-dependencies` を含む) には入れず、
ワークスペースの外のツールとして使う。版は `scripts/ffi.mjs` の `CBINDGEN_VERSION` と CI (`.github/workflows/ci.yml` の `cbindgen@...`) で揃え、
違う版や未導入なら `just ffi-check` / `just ffi-header` が入れ方を案内して止まる。ヘッダには cbindgen の版を書かない。

`just ffi-check` は cbindgen と C コンパイラ (`cc`。`CC` で変えられる) が要るので `just ci` には含めず、CI の `rust (bindings)` (Linux) と
`rust (macos)` のジョブで別に実行する (Windows では流さない。[docs/testing.md](../../docs/testing.md#windows-と-macos))。

```sh
cargo install cbindgen --version 0.29.4 --locked
just ffi-header   # include/ugoe.h を作り直す
just ffi-check    # ヘッダが最新かの検査と、C の見本のビルド・実行
```

## std の無い環境

ゲーム機や組み込み機器のように Rust の `std` が無い環境には、既定で有効な feature `std` を外した静的ライブラリを組み込む
([ADR 0031](../../docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md#std-の無い環境への-c-abi)。T0491)。
`std` を外すと、このクレートと依存するランタイム層のクレートは `#![no_std]` (`core` と `alloc`) でビルドされる。

```sh
# 巻き戻さないプロファイル nostd (ルートの Cargo.toml。release と同じで panic = "abort") で、staticlib だけを作る
cargo rustc -p ugoe-ffi --lib --crate-type staticlib --no-default-features --profile nostd --target <ターゲット>
# → target/<ターゲット>/nostd/libugoe_ffi.a
```

- **関数の集合とヘッダは変わらない。** 同じ `include/ugoe.h` を使う。失敗の中身は呼び出し側の `UgoeError` に書くので
  ([失敗の受け取り方](#失敗の受け取り方-t0489))、スレッドローカルもグローバルな状態も要らない。
- **アロケータ** (`#[global_allocator]`。`src/nostd.rs`) は、ホストの C ライブラリの `malloc`・`calloc`・`realloc`・`free` と、
  `malloc` の整列 (64 ビットのターゲットで 16 バイト、ほかで 8 バイトとみなす) より大きな整列が要るときの `aligned_alloc` (C11) を呼ぶ。
- **パニック** (`#[panic_handler]`) は、ホストが登録した関数にメッセージを渡してから ([下](#パニックのメッセージをホストに渡す-t0494))、
  ホストの C ライブラリの `abort` を呼ぶ。巻き戻しが無いので `catch_unwind` できず、
  `UGOE_ERROR_PANIC` は返らない。確保の失敗も `abort` になる。ランタイム層と C ABI はパニックを起こさないように書いている
  ([docs/portability.md#パニックしない](../../docs/portability.md#パニックしない)) ので、パニックは ugoe の誤りであり、`abort` はその最後の守り。
- どちらもシンボル (`malloc`・`calloc`・`realloc`・`aligned_alloc`・`free`・`abort`) をホストのリンクで解決するだけで、ゲーム機などの
  SDK ごとの知識はこのクレートに持たない。SDK の C ライブラリがこれらを持たなければ、同じ名前の関数をホストの側で用意する。
  `memcpy` などは Rust の `compiler_builtins` が持つターゲットと、ホストの C ライブラリに任せるターゲットがある。
- **使えない機能:** テクスチャのデコード (`ugoe_image_decode`) は、PNG とブロック圧縮 (BCn) の KTX2 をデコードしない (デコーダが `std` を要る)。
  どちらも `UGOE_ERROR_FORMAT`・種類 `"unsupported_texture"` で失敗するので、ホストのデコーダで読む。圧縮していない 8 ビットの KTX2 は読む。
  ほかの関数は `std` の有無にかかわらず同じに働く (読み込み・評価・物理・モーション・トラッキング・着せ替え・JSON の読み書き)。
- **パニックのプロファイル:** `panic = "abort"` は Cargo のプロファイルで決まるので、ワークスペースの既定のプロファイル (dev・release) は変えず、
  専用のプロファイル `nostd` を使う。Rust のツールチェインが配る `core` が巻き戻しありでビルドされているターゲット (ホストの Linux など) では、
  `rust_eh_personality` が未解決のシンボルとして残る (呼ばれることは無い)。巻き戻しの無いターゲット (`aarch64-unknown-none` など) や、
  `core` を `panic = "abort"` でビルドし直す (`-Zbuild-std`) ときは残らない。残るときはホストで形だけ定義する (`examples/c/nostd_main.c`)。
- **Rust から使わない:** `std` を外した `ugoe-ffi` は、アロケータとパニックの処理を定義するので、ほかの Rust のクレートから rlib として使わない
  (Rust のホストは `ugoe` を使う)。同じ理由で、`std` を外した形の結合テスト (`tests/`) は動かない (単体テストは動く)。

**検査 (実機での確認はしていない):**

- `just nostd-check` (CI の rust (runtime)): `aarch64-unknown-none` (std の無い tier 2 のターゲット) で clippy、ホストで `std` を外した形の単体テスト、
  静的ライブラリを作ってフリースタンディングの C の見本 [`examples/c/nostd.c`](examples/c/nostd.c) と (clang でコンパイルして rust-lld で) リンクし、
  未解決のシンボルが `abort`・`aligned_alloc`・`calloc`・`free`・`malloc`・`realloc` だけであることを確かめる (`scripts/nostd-check.mjs` の
  `HOST_SYMBOLS`)。
- `just ffi-check` (CI の rust (bindings) と rust (macos)): ホスト (Linux・macOS) 向けに `std` を外した静的ライブラリを作り、同じ見本を
  [`examples/c/nostd_main.c`](examples/c/nostd_main.c) からホストの C ライブラリ (アロケータとパニックの処理の相手) とリンクして、
  見本のモデルで実行する。Windows (MSVC の C ライブラリは `aligned_alloc` を持たない) では飛ばす。
- ゲーム機の実機と SDK での確認は、組み込み先 (SDK を持つ人) に任せる。`aarch64-unknown-none` の検査で見つからない問題 (TLS・アトミック・
  浮動小数点の設定など) があれば ADR 0031 を見直す。

### パニックのメッセージをホストに渡す (T0494)

ゲーム機や組み込み機器ではデバッガをつなげないことも多いので、`abort` の前にパニックの位置とメッセージをホストのログに渡せる
([ADR 0031 の追記](../../docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md#追記-パニックのメッセージをホストに渡す-t0494))。

```c
static void on_panic(const char *message, size_t len, void *user) {
    host_log_error("ugoe: %.*s", (int)len, message);   /* 例: "panicked at crates/ugoe/src/x.rs:10:5: ..." */
}

ugoe_set_panic_handler(on_panic, NULL);   /* 初期化のとき、ほかの ugoe の関数より先に 1 回 */
```

- **呼ばれ方:** パニックすると、登録した関数に `panicked at <file>:<line>:<column>: <message>` を UTF-8 で渡し、関数から戻った後で
  `abort` を呼ぶ。関数の中でアプリを止めてもよいが、`longjmp` や例外で抜けてはならない (ugoe の中に戻れない)。登録が無ければ
  今までどおり `abort` だけ。`handler` に NULL を渡すと登録を外す。
- **バッファ:** メッセージは固定の大きさ (`UGOE_PANIC_MESSAGE_MAX` = 255 バイトと NUL) のスタックのバッファに `core::fmt::Write` で書き、
  確保しない (確保の失敗のパニックでも呼べる)。入りきらない分は UTF-8 の文字の境界で切り捨てる (位置を先に書くので、位置は残る)。
  ポインタは呼ばれている間だけ有効。
- **もう一度パニックしない:** 書式化の失敗 (切り詰め) は無視する。登録した関数や書式化の中でもう一度パニックしたら、関数を呼ばずに `abort` する。
  別々のスレッドが同時にパニックしたときも、後のほうは関数を呼ばずに `abort` する (どちらにしてもアプリは止まる)。
- **登録はプロセスに 1 つ:** C ABI はグローバルな状態を持たないが ([失敗の受け取り方](#失敗の受け取り方-t0489))、パニックの処理はもともと
  プロセスに 1 つなので、登録だけはグローバルに置く。ポインタの幅のアトミック (`AtomicPtr` の読み書き) に置くので、64 ビットの
  アトミック操作の無いターゲット (`armv6k-nintendo-3ds`) でも使える。関数と `user` は 2 つのアトミックに分けて置くので、
  ほかのスレッドが ugoe の関数を呼んでいない間 (ふつうは初期化のとき) に登録する。
- **`std` がある形:** 関数はあるが、登録した関数は呼ばない。パニックは `catch_unwind` で `UGOE_ERROR_PANIC` になり、メッセージは
  `UgoeError` に書くので、ホストはそちらで受け取る (std のパニックのフックにはつながない。ホストの Rust のフックと混ざらないように)。
- **検査:** `just ffi-check` が、わざとパニックする関数 `ugoe_test_panic` を足すテスト用の feature `panic-test` (ヘッダには出ない。
  配る静的ライブラリでは有効にしない) で std の無い形を作り、[`examples/c/nostd_main.c`](examples/c/nostd_main.c) の `--panic` で、
  登録した関数にメッセージと位置が届いてから `SIGABRT` で止まることを確かめる。登録と書式化と切り詰めは単体テスト (`src/panic.rs`) で確かめる。

### core と alloc が配られないターゲット (tier 3。T0495)

ゲーム機向けのターゲットの多くは Rust の tier 3 で、`core`・`alloc` がビルド済みで配られない (`rustup target add` できない)。
nightly の `-Zbuild-std` で `core`・`alloc` をソースからビルドする ([build-std](https://doc.rust-lang.org/cargo/reference/unstable.html#build-std))。
ゲーム機の SDK のツールチェインが独自の Rust を配るなら、その手順に従う。

```sh
# nightly と、core・alloc のソース (rust-src)。nightly はルートの Cargo.toml の rust-version 以上であること
# (古い nightly は rust-version で拒まれる)。リポジトリの rust-toolchain.toml (stable) は `+nightly` で上書きする
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

cargo +nightly rustc -p ugoe-ffi --lib --crate-type staticlib --no-default-features --profile nostd \
  --target <ターゲット> -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
# → target/<ターゲット>/nostd/libugoe_ffi.a
```

- **`cargo rustc --crate-type staticlib` で staticlib だけを作る。** `cargo build` でも通るが、`Cargo.toml` の `crate-type` にある
  cdylib も作ろうとして、ターゲットによってはリンカの警告 (`aarch64-nintendo-switch-freestanding` の `cannot find entry symbol _start`)
  が出る。
- **`-Zbuild-std-features=compiler-builtins-mem`:** 付けないと、`memcpy`・`memmove`・`memset`・`memcmp` が未解決のシンボルとして
  残る (ビルド済みの `aarch64-unknown-none` の `core` はこれらを持つが、`-Zbuild-std` の既定では持たない)。付けると、未解決のシンボルは
  `just nostd-check` と同じ `abort`・`aligned_alloc`・`calloc`・`free`・`malloc`・`realloc` だけになる。SDK の C ライブラリの `memcpy` など
  (速いことが多い) を使いたければ付けずに、ホストのリンクで解決する。
- **パニック:** プロファイル `nostd` の `panic = "abort"` で `core` もビルドし直すので、`rust_eh_personality` は残らない。
  パニックの戦略 `immediate-abort` (旧 `panic_immediate_abort`。nightly で `-Zpanic-immediate-abort --config 'profile.nostd.panic="immediate-abort"'`) は
  使わない。パニックがその場のトラップ命令になって `#[panic_handler]` (ホストの `abort`) を通らず、ホストがパニックに気づく
  手がかりを失う割に、ライブラリは 3% ほどしか小さくならない。
- **アトミック操作:** ランタイム層は `alloc::sync::Arc` を使うので、ポインタの幅のアトミックな比較と交換 (CAS) が要る。CAS の無い
  ターゲット (`thumbv6m-none-eabi` など) ではビルドできない。64 ビットのアトミック操作は要らない (`armv6k-nintendo-3ds` などの
  32 ビットのターゲットでもビルドできる)。
- **確かめたこと (nightly 1.101、2026-10):** 次のターゲットで上のコマンドが通り、未解決のシンボルが上の 6 つだけであることを、
  フリースタンディングの C の見本 [`examples/c/nostd.c`](examples/c/nostd.c) と rust-lld でリンクして確かめた。CI では検査しない
  (nightly が要るため。tier 2 の `aarch64-unknown-none` は `just nostd-check` が検査する)。

  | ターゲット | 幅 | 確かめたこと |
  | --- | --- | --- |
  | `aarch64-nintendo-switch-freestanding` | 64 ビット | ビルド・未解決のシンボル・形だけのホストの関数とのリンク |
  | `armv6k-nintendo-3ds` | 32 ビット (64 ビットのアトミック操作が無い) | ビルド・未解決のシンボル・形だけのホストの関数とのリンク |
  | `armv7-sony-vita-newlibeabihf` | 32 ビット | ビルド・未解決のシンボル・形だけのホストの関数とのリンク |
  | `mipsel-sony-psp` | 32 ビット (単精度の浮動小数点) | ビルド・未解決のシンボル (C の見本の浮動小数点の ABI を合わせられず、リンクまでは確かめていない) |

- 実機と SDK のツールチェイン (SDK のリンカ・C ライブラリ・スレッドとメモリの設定) では確かめていない。

## 毎フレームの描画の状態 (T0490)

描画の情報は、読む時機で 2 つに分かれる ([ADR 0031](../../docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md#毎フレームの描画の状態はまとめて渡す))。

- **モデルの形** (読み込みと着せ替えの後に 1 回): 描画可能要素の数と情報 (`ugoe_puppet_drawable_count`・`_info`)・マスクの参照 (`_mask`)・
  UV (`_uvs`)・インデックス (`_indices`)・ライティングを受けるか (`_lit`)・テクスチャ (`ugoe_puppet_texture`・`_normal_map`)。描画要素ごとに読む。
- **このフレームの状態** (`ugoe_puppet_update` の後に毎回): すべての描画可能要素の状態 (`UgoeDrawableState`: 表示・不透明度・色・合成モード・
  マスクの効き具合・描画順の値・変更のフラグ)・描画順・描画順が変わったか・頂点の位置。1 回の呼び出しでまとめて受け取る
  (cgo やマネージドな言語のように、境界を越える呼び出しが重いホストでも、呼び出しの数が描画要素の数によらない)。

```c
UgoeFrame frame;
ugoe_puppet_frame(puppet, &frame, err);
for (uint32_t k = 0; k < frame.drawable_count; k++) {
    uint32_t d = frame.render_order[k];            /* 奥から手前へ */
    const UgoeDrawableState *s = &frame.states[d];
    if (s->changes & UGOE_CHANGED_POSITIONS) {     /* 変わった要素だけ頂点バッファへ写す */
        const float *xy = frame.positions + frame.position_ranges[d].first;
        upload(d, xy, frame.position_ranges[d].count);
    }
    draw(d, s);
}
```

- 頂点の位置は、すべての描画要素の位置 (キャンバス座標の x と y の組) を番号順につないだ 1 つの配列 (`positions`、float が `positions_len` 個)。
  描画要素 `d` の位置は `position_ranges[d]` の `first` から `count` 個 (単位は float。`count` は 2 × 頂点の数)。範囲は読み込みと着せ替えの後は変わらない。
- **借用** (`ugoe_puppet_frame`): ポインタはすべて、次の `ugoe_puppet_update`・`ugoe_puppet_follow`・着せ替えの操作・`ugoe_puppet_destroy` まで有効。
  コピーを省きたいホストのため。
- **コピー** (`ugoe_puppet_copy_frame(puppet, &buffers, changed_positions_only, &out, err)`): ホストのバッファ (`UgoeFrameBuffers`) に写し、
  描画要素の数・位置の float の数・描画順が変わったかを `UgoeFrameCopy` に書く。`drawable_capacity` か `positions_capacity` が足りなければ
  `UGOE_ERROR_BUFFER_TOO_SMALL` を返し、要る数 (`*out`) だけを書く (すべてのポインタを NULL・大きさを 0 にして呼べば要る数が分かる)。
  借用の寿命を扱いにくいホスト (マネージドな言語) のため。`changed_positions_only` が true なら、頂点の位置は変更のフラグ
  (`UGOE_CHANGED_POSITIONS`) が立った描画要素の範囲だけを写す (毎フレーム同じバッファへ写すホストが、変わっていない位置を写さずに済む)。
- 変更のフラグは前の `ugoe_puppet_update` からの変化 (作った直後と最初の `update` の後と、着せ替えの直後はすべて立つ)。`update` の後に
  毎回読まなかったホストは、フラグによらずすべてを読み直す。
- 内部では、頂点の位置をつないだ配列を `ugoe_puppet_frame`・`ugoe_puppet_copy_frame` を呼んだときに作り、同じ評価の結果なら作り直さない。
  前に読んだ評価のちょうど次の `update` なら、位置の変わった描画要素だけを写す (`ugoe-runtime` の `ModelInstance::id`・`update_count`)。
  そのため、この 2 つの関数は `UgoePuppet *` (const でない) を取る。
- Unity は `ugoe_puppet_copy_frame` を `FrameSync.Pull` で 1 フレームに 1 回呼ぶ (bindings/unity/README.md)。Godot は C ABI ではなく SDK を直接使う。

## ABI の版

`ugoe_abi_version()` (ヘッダでは `UGOE_ABI_VERSION`) は、**既存の** 関数・型・定数を壊したときだけ上げる (T0409 で決めた)。

- 上げる: 既存の関数の引数・戻り値の型や意味、構造体のフィールドの並び・大きさ、定数の値を変えたとき、関数や型を消したとき。
- 上げない: 関数・型・定数・エラーコードを **足す** だけのとき。古いヘッダで作ったホストは、足したものを使わないので
  そのまま動く。新しい関数があるかは、ライブラリの版 (`ugoe_version()`) で確かめる。
- そのため、構造体にフィールドを足したくなったら、既存の構造体は変えずに別の関数や構造体を足す
  (例: 描画可能要素がライティングを受けるかは `UgoeDrawableInfo` に足さず、`ugoe_puppet_drawable_lit` にした)。
- 知らないエラーコードが返ることがある (足したもの)。0 以外はすべて失敗として扱い、`ugoe_error_message` を出せばよい。
- 今の版は 3。T0489 (`UgoeError`) で 1 から 2 に上げ、T0490 (毎フレームの描画の状態をまとめて受け取る形。描画要素ごとの
  `ugoe_puppet_drawable_state`・`ugoe_puppet_positions` などを消した) も同じ 2 に含めた。T0494 (`ugoe_set_panic_handler`) で 3 に上げた。
  関数を足すだけだが、std の無い形のパニックの扱い (ホストの関数を呼んでから `abort`) が変わったことを版で確かめられるよう、利用者の判断で上げた。

## 着せ替え (T0409)

```c
UgoeModule *hair = NULL;
ugoe_module_create(bytes, len, &hair, err);            /* .ugoemod を読む (付けはしない)。読めなければ UGOE_ERROR_FORMAT */
if (ugoe_puppet_attach_module(puppet, hair, err) != UGOE_OK) {
    /* UGOE_ERROR_DRESS_UP: 種類と補足の値は wasm の DressUpError と同じ */
    printf("%s (%s)\n", ugoe_error_kind(err), ugoe_error_field(err, "slot"));
}
const uint32_t *changed; size_t n;
ugoe_puppet_changed_textures(puppet, &changed, &n, err); /* 画像を設定し直すテクスチャの番号 */
ugoe_puppet_detach_module(puppet, "LongHair", err);
ugoe_puppet_set_modules(puppet, list, count, err);       /* 付ける組をまとめて決める (合成の順)。0 個で本体だけ */
ugoe_module_destroy(hair);                          /* 付けたモジュールは付いたまま残る */
```

- 規則は `docs/spec/modules.md#puppet-での着せ替え` (`ugoe::Puppet::attach_module` などと同じ)。同じスロットか同じ名前の
  モジュールは差し替える。失敗したときは何も変えない。
- **付け外しに成功すると、モデルが差し替わる。** 描画可能要素の数・並び・ID が変わり、この `UgoePuppet` から借りたポインタ
  (パラメータ・描画可能要素・当たり判定の ID、頂点・UV・インデックス・描画順、付けているモジュールの名前、外したモジュールのテクスチャ)
  は無効になる。描画可能要素の情報は取り直す。
- テクスチャは本体、モジュールの順に並ぶ (`ugoe::Puppet::texture_sources`)。付け外しの後は `ugoe_puppet_changed_textures` の番号の
  画像と法線マップを `ugoe_puppet_texture` / `ugoe_puppet_normal_map` で引き直し、`ugoe_puppet_texture_count` 以上の番号は捨ててよい。
- `UgoeModule` は `Arc` で共有するので、破棄しても付けたモジュール (と画像) は残り、複数の `UgoePuppet` に付けてよい。
- スロットは `ugoe_puppet_slot_count` / `ugoe_puppet_slot_id` (本体のもの)、付けているモジュールは
  `ugoe_puppet_attached_module_count` / `ugoe_puppet_attached_module_name` (合成の順)。

失敗の種類 (`ugoe_error_kind`) は wasm の `DressUpError` の `code` と同じで、同じ補足の値
(`ugoe_error_field(err, "module")`・`"slot"`・`"parameter"`・`"mesh"`・`"attached"` など) を持つ
([crates/ugoe-wasm/README.md](../ugoe-wasm/README.md#着せ替え-t0123))。番号 (`texture`・`required`・`actual`・`errors`) は 10 進の文字列
(wasm では数)、種類の一覧 (`error_codes`) は `, ` でつないだ文字列 (wasm と同じ)。無い値 (本体がアバタープロファイルを持たないときの `actual`) は NULL (T0157。キーの意味は
[crates/ugoe/README.md](../ugoe/README.md#着せ替えの失敗の補足の値))。

| 種類 | 戻り値 | 補足の値 |
| --- | --- | --- |
| `invalid_file` | `UGOE_ERROR_FORMAT` (`ugoe_module_create`) | |
| `read_limit_exceeded` | `UGOE_ERROR_LIMIT_EXCEEDED` (`ugoe_module_create`・`ugoe_module_create_with_limits`。[読み込みの上限](#読み込みの上限-t0152)) | `limit`・`chunk`・`max` |
| `unknown_slot`・`missing_slot` | `UGOE_ERROR_DRESS_UP` | `module`・`slot` |
| `missing_parameter` | 同上 | `module`・`parameter` |
| `parameter_not_exposed` | 同上 | `module`・`slot`・`parameter` |
| `slot_occupied` | 同上 | `slot`・`module`・`attached` |
| `unknown_mask` | 同上 | `module`・`mesh`・`mask` |
| `texture_out_of_range` | 同上 | `module`・`mesh`・`texture` |
| `unknown_collider` | 同上 | `module`・`rig`・`collider` |
| `pose_not_exposed` | 同上 | `module`・`slot`・`part` |
| `avatar_level` | 同上 | `module`・`required`・`actual` (本体のレベル。無ければ NULL) |
| `invalid_module_name`・`duplicate_module`・`not_attached` | 同上 | `module` |
| `invalid_composition` | 同上 | `errors` (検査のエラーの数)・`error_codes` (検査のエラーの種類。例: `"unknown_reference, duplicate_id"`。T0168) |
| `compile` | 同上 | |
| `invalid_physics` | 同上 (物理の設定の差し替え `ugoe_puppet_set_physics_json`・`ugoe_puppet_set_base_physics_json` では `UGOE_ERROR_INVALID_MODEL`。T0469) | `errors`・`error_codes` |

wasm にある `not_loaded`・`no_model`・`texture` は C ABI では起きない (モジュールはポインタで渡し、`UgoePuppet` は常にモデルを持ち、
画像の登録はホストが行う)。種類を持たない失敗では `ugoe_error_kind` は NULL。種類と補足の値の対応は SDK の
`ugoe::DressUpError::code`・`fields` (と `INVALID_FILE`) の 1 か所にあり、wasm も同じものを使う (T0412)。

## 読み込みの上限 (T0152)

```c
UgoeReadLimits limits = ugoe_read_limits_default();       /* 1 チャンク 256 MiB・合計 1 GiB */
limits.max_total_decompressed = 64u * 1024 * 1024;        /* 置かれた環境に合わせて変える */
UgoeResult r = ugoe_puppet_create_from_bundle_with_limits(bytes, len, &limits, &puppet, err);
if (r == UGOE_ERROR_LIMIT_EXCEEDED) {
    /* ugoe_error_kind(err) は "read_limit_exceeded" */
    printf("%s %s > %s\n", ugoe_error_field(err, "chunk"), ugoe_error_field(err, "limit"), ugoe_error_field(err, "max"));
}
ugoe_module_create_with_limits(bytes, len, &limits, &module, err); /* .ugoemod も同じ */
```

- 上限はファイル形式の決まりではなく、組み込む側の方針 (`docs/spec/file-formats.md#読み込みの上限`)。数えるのは圧縮したチャンクを
  展開したバイトだけで、上限を超えそうになったら展開の途中でやめる (先に確保しない)。`limits` が NULL なら既定値。
- 上限を指定しない `ugoe_puppet_create_from_bundle`・`ugoe_module_create` は既定値で読み、超えたら同じく `UGOE_ERROR_LIMIT_EXCEEDED` を返す。
- 種類 (`ugoe_error_kind`) は `"read_limit_exceeded"`、補足の値は `"limit"` (`"chunk_decompressed"` | `"total_decompressed"`)・
  `"chunk"` (展開をやめたチャンクのタグ。`"TEXR"` など)・`"max"` (超えた上限のバイト数、10 進)。値は SDK の
  `ugoe::limits::ReadLimitExceeded` で、wasm の読み込みのエラーの `code` と同じ。
- PNG の大きさの上限 (wasm の `max_image_side`) は読み込みには無い。読み込みは画像をデコードせず、エンコードされたまま渡すので、デコードの上限はホストが決める
  (`ugoe_image_decode` を使うなら `UgoeImageLimits`。[テクスチャのデコード](#テクスチャのデコード-t0466))。
- JSON から作る `ugoe_puppet_create_from_json` は展開をしないので、上限を取らない (大きさは入力の長さで決まる)。
- Unity (`bindings/unity`) は `UgoeReadLimits` と `Puppet.FromBundle(bytes, limits)`・`Module.FromBytes(bytes, limits)` で出す (T0458)。Godot は C ABI ではなく SDK を使う。

## モデルの読み込みの失敗 (T0457)

```c
UgoeResult r = ugoe_puppet_create_from_bundle(bytes, len, &puppet, err);
if (r != UGOE_OK) {
    const char *kind = ugoe_error_kind(err); /* "unsupported_version" など。NULL なら種類なし */
    if (kind && strcmp(kind, "unsupported_version") == 0)
        printf("version %s > %s\n", ugoe_error_field(err, "found"), ugoe_error_field(err, "supported"));
}
```

`ugoe_puppet_create_from_bundle`・`ugoe_puppet_create_from_bundle_with_limits`・`ugoe_puppet_create_from_json` の失敗は、種類
(`ugoe_error_kind`) と補足の値 (`ugoe_error_field`) を持つ。値は wasm の `LoadError` の `code` と同じで
([crates/ugoe-wasm/README.md](../ugoe-wasm/README.md#読み込みの失敗-t0434))、SDK の `ugoe::LoadError` (`code()`・`fields()`) の 1 か所で決める。
補足の値は 10 進の文字列 (wasm では数)。戻り値は T0457 の前と同じ (種類を足しただけなので ABI の版は上げない)。

| 種類 | 戻り値 | 補足の値 | いつ |
| --- | --- | --- | --- |
| `invalid_file` | `UGOE_ERROR_FORMAT` | | ファイルを読めない (ugoe のファイルでない・壊れている・JSON の誤りなど。細かい理由は `ugoe_error_message` にだけある) |
| `unsupported_version` | 同上 | `found` (ファイルの版)・`supported` (読める版) | ファイルの版 (スキーマかコンテナ) が新しすぎる |
| `unsupported_features` | 同上 | `features` (知らない必須の機能の ID を `", "` でつないだもの) | 知らない必須の機能を宣言している |
| `invalid_model` | `UGOE_ERROR_FORMAT` (バンドルを読むときの検査)・`UGOE_ERROR_INVALID_MODEL` (コンパイルの前の検査。JSON はここ) | `errors` (検査のエラーの数)・`error_codes` (検査のエラーの種類を `", "` でつないだもの。着せ替えの `invalid_physics` と同じ。T0469) | モデル・物理・モーション・表情が検査に通らない |
| `compile` | `UGOE_ERROR_INVALID_MODEL` | | 検査は通ったが評価できない構造 |
| `read_limit_exceeded` | `UGOE_ERROR_LIMIT_EXCEEDED` | `limit`・`chunk`・`max` | [読み込みの上限](#読み込みの上限-t0152) を超えた |

- wasm の `image_limit_exceeded`・`invalid_texture`・`texture_too_large`・`texture`・`device_lost` はモデルの読み込みでは起きない (画像をデコードせず、
  レンダラを持たない)。`image_limit_exceeded`・`invalid_texture` は `ugoe_image_decode` の失敗の種類にある ([テクスチャのデコード](#テクスチャのデコード-t0466))。
- 引数の誤り (NULL・UTF-8 でない JSON) は種類を持たない (`ugoe_error_kind` は NULL)。
- `.ugoemod` の読み込み (`ugoe_module_create`) は今までどおり、上限を超えたものだけ `read_limit_exceeded` で、ほかは `invalid_file`
  ([着せ替え](#着せ替え-t0409)。wasm の `load_module` と同じ)。
- テストは `tests/load_errors.rs`。

## テクスチャのデコード (T0466)

```c
UgoeTexture texture;
ugoe_puppet_texture(puppet, index, &texture, err);            /* エンコードされたまま (PNG・KTX2) */
UgoeImage *image = NULL;
UgoeResult r = ugoe_image_decode(texture.format, texture.data, texture.len,
                                 NULL /* 既定の上限 */, true /* 乗算済みにする */, &image, err);
if (r == UGOE_OK) {
    uint32_t w, h; const uint8_t *rgba; size_t len;
    ugoe_image_size(image, &w, &h, err);
    ugoe_image_pixels(image, &rgba, &len, err);               /* w * h * 4 バイト、上の行が先。ugoe_image_destroy まで有効 */
    upload(rgba, w, h);
    ugoe_image_destroy(image);
} else if (strcmp(ugoe_error_kind(err), "unsupported_texture") == 0) {
    /* ETC2・ASTC など。白 (法線マップなら平ら) で代用する */
}
```

- `std` の無い形では PNG と BCn の KTX2 を `unsupported_texture` で断る ([std の無い環境](#std-の無い環境))。
- 参照 (`ugoe render`)・wasm と同じ `ugoe_format::image` のデコーダで、同じ画素になる (`docs/spec/rendering.md#前提`)。
  PNG と、KTX2 の圧縮していない 8 ビットの RGB / RGBA と BC1〜BC5・BC7 を読む ([ADR 0030](../../docs/adr/0030-compressed-ktx2-decoding.md))。
  色はリニア化せず、KTX2 のミップマップは最初の段だけを使う。`premultiply` が true なら乗算済みアルファ (参照と同じ整数の丸め)、
  法線マップは false (アルファを掛けない)。
- バイト列と形式 (`UgoeTexture` の `format` と `data`・`len`) を受け取るだけの関数で、`UgoePuppet` を変えない。法線マップ
  (`ugoe_puppet_normal_map`)・モジュールのテクスチャにも同じに使う。PNG を自分のエンジンで読むホストは、読めない形式にだけ使えばよい。
- 上限は `UgoeImageLimits` (`max_side`、既定は一辺 16384。`ugoe_image_limits_default`)。NULL なら既定値。超える画像は画素を確保する前に失敗にする。
- 結果は不透明な `UgoeImage` で、`ugoe_image_destroy` で解放する (画素のポインタはそこまで有効)。

| 種類 (`ugoe_error_kind`) | 戻り値 | 補足の値 | いつ |
| --- | --- | --- | --- |
| `image_limit_exceeded` | `UGOE_ERROR_LIMIT_EXCEEDED` | `width`・`height`・`max` | 一辺が上限を超える (wasm の同じ種類はさらに `image`・`texture` を持つ) |
| `unsupported_texture` | `UGOE_ERROR_FORMAT` | | 読める KTX2 だが、デコードしない形式 (ETC2・ASTC・BC6H・Basis Universal など)。wasm では読み込みの警告 `texture_ktx2`・`normal_map_ktx2` |
| `invalid_texture` | `UGOE_ERROR_FORMAT` | | 画像が壊れている |

`format` が `UGOE_TEXTURE_PNG`・`UGOE_TEXTURE_KTX2` でなければ `UGOE_ERROR_INVALID_ARGUMENT` (種類なし)。

**形を決めた理由:** ABI の版の規則 ([ABI の版](#abi-の版)) どおり関数と型を足すだけにし、既存の関数 (`ugoe_puppet_texture` などのエンコードされた
バイト列と、読み込み) の意味は変えない。読み込みでデコードまで済ませる形は、使わないホスト (PNG を自分のエンジンで読む) にもデコードの時間と
メモリを払わせ、読み込みの関数に画像の上限の引数を足すことになる (T0152 で、デコードと上限はホストの責務とした)。テクスチャの番号で
デコードする形は、法線マップとモジュールのテクスチャに別の関数が要る。結果の受け渡しは、大きさを先に知る関数が無いので、ホストの
バッファに書く形 (`capacity` で数を聞いて呼び直す) では 2 度デコードすることになるため、`UgoeModule` と同じ不透明なオブジェクトにした。

**共有ライブラリの大きさ:** PNG のデコーダ (`png`) と BCn の展開 (`bcdec_rs`) が入る。Linux x86_64 の release の `libugoe_ffi.so` で
6,302,904 → 6,496,256 バイト (約 189 KiB、3% 増。T0466 で測った)。

テストは `src/image.rs` (PNG・BC1 の KTX2・上限・種類)、C の見本 (`examples/c/sample.c`)、Unity (`bindings/unity/tests/ImageTests.cs`)、
適合テストのテクスチャを持つバンドルのケース (PNG・KTX2。Unity の C# の層の結果の `textures` を参照のデコードと比べる。[tests/conformance/README.md](../../tests/conformance/README.md))。

## ライティング (T0409)

- `ugoe_puppet_uses_lighting` (光源の操作を出すか)、`ugoe_puppet_drawable_lit` (その要素に陰影を掛けるか)、
  `ugoe_puppet_normal_map` (テクスチャの番号ごとの法線マップのバイト列。無ければ `UGOE_TEXTURE_NONE` で、法線 `(0, 0, 1)` として扱う)。
  法線マップの PNG は **アルファを掛けずに** デコードする。
- 光源はホストの状態 (`UgoePuppet` は持たない)。`ugoe_lighting_uniform(ambient, lights, count, &uniform)` が光源の組
  (`UgoeLight`。`ugoe::lighting::Lighting` と同じ中身) を検査し、参照シェーダのユニフォームと同じ配置 (`UgoeLightingUniform`。
  平行光源は光源へ向かう単位ベクトル、点光源は位置と `1 / range`) にする。エンジンのシェーダで陰影の式
  (`docs/spec/rendering.md#ライティング`) を書くときに、そのまま定数バッファに入れられる。
- 向きが 0・有限でない値・`range` が正でない・負の色は `UGOE_ERROR_INVALID_ARGUMENT` (wasm の `set_lighting_json` と同じく先に知らせる)。
  `UGOE_MAX_LIGHTS` (8) を超える光源は先頭だけを使う。

## 当たり判定・トラッキング・そのほか (T0409)

- 当たり判定: `ugoe_puppet_hit_area_count` / `_id`、`ugoe_puppet_hit_areas_at` (手前から順の番号)、`ugoe_puppet_hit_test` (ID)、
  標準の名前 (`"Head"`・`"Body"`) の `ugoe_puppet_standard_hit_area` / `ugoe_puppet_hit_test_standard`。直前の `update` の形で判定する。
- 可変長の結果 (`ugoe_puppet_hit_areas_at`・`ugoe_puppet_tracking_targets`) は、ホストのバッファ (`out`・`capacity`) に書き、数を
  `*out_count` に書く。足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で数だけを書くので、`capacity` 0 で数を聞いてから呼び直せる。
- トラッキング: `ugoe_puppet_set_tracking(puppet, &input)` (NULL で入力をやめる)。`UgoeTrackingInput` は頭の回転、
  体の回転と肩 (NaN で省略)、ARKit の 52 のブレンドシェイプ (添字は `ugoe_blend_shape_name` / `ugoe_blend_shape_index`。
  MediaPipe のカテゴリ名をそのまま引ける)。平滑化などはホストが行う。トラッキング中は `ugoe_puppet_set_stage_enabled` で
  目パチ (`UGOE_STAGE_EYE_BLINK`) を止めるとよい。書き込むパラメータは `ugoe_puppet_tracking_targets`。
- トラッキング中のモーションの優先 (T0722): `ugoe_puppet_set_motion_over_tracking(puppet, true)` の間は、再生中のモーションが動かす
  パラメータではモーションの値がトラッキングの上に乗る (既定は `false` で、トラッキングがモーションの値を置き換える)。
  `ugoe_puppet_set_tracking_kept(puppet, indices, count)` (パラメータの番号) で選んだパラメータは、優先していてもトラッキングのまま
  (目は除く、など)。設定は `ugoe_puppet_motion_over_tracking` で読める。`Puppet::set_motion_over_tracking`・`set_tracking_kept` と同じで、
  wasm の `set_motion_over_tracking`・`set_tracking_kept` と揃えた (`docs/spec/motion.md#トラッキング中のモーションの優先`)。
- `ugoe_puppet_set_parameters_by_id` (ID でまとめて設定。知らない ID は飛ばし、設定した数を返す)、
  `ugoe_puppet_stop_expressions` (表情の解除)。

## 読み込みの警告 (T0409)

`ugoe_puppet_load_warning_count` / `ugoe_puppet_load_warning` (`UgoeWarning { code, message }`) /
`ugoe_puppet_load_warning_field` (補足の値。数も 10 進の文字列)。`.ugoemod` は `ugoe_module_warning_*`。
`code` は wasm の `load_warnings_json` と同じ値を使う ([crates/ugoe-wasm/README.md](../ugoe-wasm/README.md#読み込みの警告-t0125))。

| `code` | 補足の値 | いつ |
| --- | --- | --- |
| `texture_missing` | `texture` | テクスチャの画像が無い (読み込みが数を検査するので通常は起きない) |
| `unknown_optional_feature` | `feature` (機能の ID) | 知らない任意の機能を宣言したファイル。フォールバックで読んだ |
| `unknown_value` | `path` (チャンクと場所) | 知らない列挙の値をフォールバックで読んだ |

`code` と補足の値は SDK の `ugoe::LoadWarning` (`code()`・`fields()`) が決め、wasm の `load_warnings_json` も同じものを使う (T0412)。

- wasm の `texture_ktx2`・`normal_map_ktx2` は C ABI では出さない (画像のデコードはホストが行うので、KTX2 を扱えるかはホストが決める。
  `ugoe_image_decode` では、読めない形式は失敗の種類 `unsupported_texture` になる)。
  `physics_removed`・`module_detached` はモデルの読み直しで起きるもので、C ABI には読み直しが無いので起きない。
- JSON から作ったときは警告を出さない (JSON はもともとテクスチャを持たない)。
- `message` は英語の文で、安定した値として扱わない。知らない `code` は `message` をそのまま出せばよい。

## モーションのイベントと物理の設定 (T0412)

```c
CHECK(ugoe_puppet_update(puppet, dt, err));
uint32_t n = 0;
ugoe_puppet_motion_event_count(puppet, &n, err);          /* 直前の update で発火したイベント */
for (uint32_t i = 0; i < n; i++) {
    UgoeMotionEvent e;
    ugoe_puppet_motion_event(puppet, i, &e, err);          /* e.time (モーションの中の秒)・e.value (次の update まで有効) */
    UgoeMotionEventSource s;
    ugoe_puppet_motion_event_source(puppet, i, &s, err);   /* 発火したモーション: s.motion (名前)・s.number (始めた番号。T0462) */
}

size_t len = 0;
ugoe_puppet_physics_json(puppet, NULL, 0, &len, err);      /* 物理が無ければ len == 0。あれば UGOE_ERROR_BUFFER_TOO_SMALL */
ugoe_puppet_set_physics_json(puppet, json, json_len, err); /* *.physics.json の形で差し替える。NULL で外す */
ugoe_puppet_reset_physics(puppet, err);                    /* 粒子を初期姿勢に戻す */
ugoe_puppet_set_base_physics_json(puppet, json, json_len, err); /* 本体の設定を差し替える。付け外しの後も保つ (T0416) */
ugoe_puppet_base_physics_json(puppet, NULL, 0, &len, err);      /* 本体の設定 (書き方は ugoe_puppet_physics_json と同じ) */
```

- イベントは `ugoe::Puppet::motion_events` と同じ (開始した順のモーションごとに、発火した順)。wasm の `motion_events()` は値だけを返す。
- **イベントの出どころ (T0462):** `ugoe_puppet_motion_event_source` は同じ番号のイベントを発火したモーションの名前と、始めた番号
  (`ugoe::Puppet::motion_event_sources`。この `UgoePuppet` で開始したモーションに 1 から振り、開始しなかった要求では進まない)。
  クロスフェードでフェードアウト中のモーションのイベントはそのモーションのもので、同じ名前を始め直したものは番号で見分ける
  (`docs/spec/motion.md#モーションマネージャ`)。**口の形:** `UgoeMotionEvent` にフィールドを足すと構造体の大きさが変わって ABI を壊すので、
  [ABI の版](#abi-の版) の規則どおり別の構造体と関数を足した (ABI の版は上げない)。wasm は `motion_events_json()`、Unity は
  `MotionEvent.Motion`・`Number`、Godot は `motion_event_fired` のシグナルと `get_motion_events()`。
- `ugoe_puppet_set_physics_json` は `ugoe::Puppet::set_physics` と同じく、モデルに対して検査する。JSON を読めなければ `UGOE_ERROR_FORMAT`、
  検査を通らなければ `UGOE_ERROR_INVALID_MODEL` で、どちらも何も変えない。粒子は次の `update` で静止姿勢から始める。
  検査の失敗の種類 (`ugoe_error_kind`) は `"invalid_physics"`、補足の値は `"errors"` (検査のエラーの数)・`"error_codes"`
  (種類を `, ` でつないだもの) で、着せ替えの `invalid_physics` と同じ (T0469。戻り値は前のまま、種類を足しただけなので ABI の版は上げない)。
- 着せ替えのモジュールを付けている間に差し替えた設定は、次の付け外しで本体の設定から作り直される (wasm の `set_physics_json` と同じ)。
  差し替えた設定を保たないことは T0414 で決めた (合成した設定は本体の分に戻せないため。`docs/spec/modules.md#puppet-での着せ替え`)。
- 付けている間も保つ設定は `ugoe_puppet_set_base_physics_json` で本体の設定として渡す (`ugoe::Puppet::set_base_physics`、T0416)。
  付けているモジュールの組と合成し直し、この後の付け外しでもこの設定から合成する。付けたことがなければ `ugoe_puppet_set_physics_json` と同じ。
  JSON を読めなければ `UGOE_ERROR_FORMAT`、本体のモデルか合成したモデルに対する検査を通らなければ `UGOE_ERROR_INVALID_MODEL`
  (種類と補足の値は `ugoe_puppet_set_physics_json` と同じ `"invalid_physics"`)、
  合成できなければ `UGOE_ERROR_DRESS_UP` (種類は `ugoe_error_kind`) で、何も変えない。本体の物理の有無が変わってスロットの
  衝突体の扱いが変わるとモデルを差し替えるので、そのときは着せ替えの操作と同じく ID のポインタが無効になる。
- `ugoe_puppet_base_physics_json` は合成する前の本体の設定 (付けたことがなければ今の設定)。モデルを読み直すホストが引き継ぐために使う。
- `ugoe_puppet_physics_json` は NUL 終端しない UTF-8 をホストのバッファに書く (可変長の結果の規約)。

## パーツ・ポーズ・効果の設定 (T0414)

```c
uint32_t parts = 0, index = 0;
ugoe_puppet_part_count(puppet, &parts, err);             /* パーツの数 (Model::parts の並び) */
ugoe_puppet_part_index(puppet, "ArmA", &index, err);     /* ID から番号。ugoe_puppet_part_id で逆引き */
UgoePartOpacity o;
ugoe_puppet_part_opacity(puppet, index, &o, err);        /* 直前の update が書いた上書き (o.value, o.weight) */
float evaluated = 0;
ugoe_puppet_part_evaluated_opacity(puppet, index, &evaluated, err); /* 評価後の不透明度 (祖先を掛けた値。T0416) */

bool shown = false;
ugoe_puppet_show_pose(puppet, "ArmB", &shown, err);      /* ポーズの要素 (part の ID) を表示する。要素でなければ false */
uint32_t groups = 0, visible = 0;
ugoe_puppet_pose_group_count(puppet, &groups, err);      /* ポーズのグループの数 (付けたモジュールのグループを含む) */
ugoe_puppet_pose_visible_part(puppet, 0, &visible, err); /* グループ 0 で表示中の要素のパーツの番号 (T0478) */
uint32_t entries = 0, entry = 0;
ugoe_puppet_pose_entry_count(puppet, 0, &entries, err);  /* グループ 0 の要素の数 (T0483) */
ugoe_puppet_pose_entry_part(puppet, 0, 1, &entry, err);  /* グループ 0 の要素 1 のパーツの番号 */
ugoe_puppet_set_effects_json(puppet, json, len, err);    /* 目パチ・呼吸・リップシンクの設定 (モデルの effects の形) */
```

- `ugoe_puppet_part_opacity` はモーションのパーツの不透明度のカーブとポーズが書いた **上書き** (`lerp(o, value, weight)` の `value` と
  `weight`。上書きしなければ `weight` は 0) で、公開仕様の時間の例 (`spec/examples/README.md#timeline-examples`) の `part_opacity` と同じもの。
  キーフォームと親を掛けた評価の後の不透明度ではない。
- `ugoe_puppet_part_evaluated_opacity` は評価の後の不透明度 (上書きを当てた自身の値に祖先の値を掛けたもの。
  `ugoe::runtime::ModelInstance::part_opacity`、`docs/spec/evaluation.md` の 2.、T0416)。そのパーツのメッシュの不透明度に掛かる
  (メッシュの不透明度は `UgoeDrawableState::opacity`)。直前の update (か着せ替えの操作) の結果。wasm は `part_evaluated_opacity(part)`。
- `ugoe_puppet_show_pose` は `PosePlayer::show` と同じ (`docs/spec/motion.md#ポーズ`)。`links` のパーツや、どの要素の `part` でもない
  パーツは何も変えず `false` (エラーにはしない)。不透明度は次の update からフェードし、そのフレームにモーションが同じグループの要素を
  要求すれば要求が勝つ。ポーズの段を止めている間も表示中の要素は変わる。wasm は `show_pose(part)`。
- `ugoe_puppet_pose_visible_part` はグループで表示中の要素の `part` をパーツの番号で返す (`PosePlayer::visible_parts`。T0478)。
  `ugoe_puppet_show_pose` で選んだ要素はフェードを待たずに、モーションのパーツの不透明度のカーブの要求で切り替わった要素は次の update の
  後に入るので、ホストはポーズの欄をモーションに追従させられる。グループの番号は合成したモデルのポーズ (付けたモジュールのグループを
  含む) の並びで、着せ替えの操作で変わる。範囲の外は `UGOE_ERROR_OUT_OF_RANGE`、要素の無いグループは `UGOE_ERROR_NOT_FOUND`。
  wasm は `pose_visible_json()` (`pose_json()` の `groups` の並びで、パーツの ID か `null` の配列)。関数を足しただけなので ABI の版は上げない
  ([ABI の版](#abi-の版))。
- `ugoe_puppet_pose_entry_count`・`ugoe_puppet_pose_entry_part` はポーズの定義 (グループごとの要素の `part` をパーツの番号で。T0483) で、
  ホストがポーズを選ぶ欄を作るために使う。並びはモデルの `entries` の並び (先頭が初めに表示する要素) で、`links` のパーツは返さない。
  グループの番号は `ugoe_puppet_pose_visible_part` と同じ (付けたモジュールのグループを含み、着せ替えの操作で変わる)。要素の無いグループの
  要素の数は 0。グループか要素の番号が範囲の外なら `UGOE_ERROR_OUT_OF_RANGE`。wasm の `pose_json()` の `groups[].entries[].part` に当たる
  (C ABI では JSON でなく番号で返し、ID は `ugoe_puppet_part_id` で引く)。関数を足しただけなので ABI の版は上げない。
- `ugoe_puppet_set_effects_json` は `Puppet::set_effects` と同じ (`docs/spec/motion.md#エフェクト設定の差し替え`)。同じ設定でも目パチと呼吸は
  初めからやり直し、リップシンクの音量は保つ。省略したフィールドは既定値、`seed` は丸めずに読む。読めなければ `UGOE_ERROR_FORMAT`。
  設定はモデルの読み込みと同じに検査する (`Puppet::try_set_effects`。有限でない値 (JSON の `1e39` は `f32` の無限大)・範囲の外の値
  (目パチの `interval` と呼吸の `cycle` が 0 以下、目パチの `closing`・`closed`・`opening` が負)・モデルにないパラメータ)。
  通らなければ `UGOE_ERROR_INVALID_MODEL` (`ugoe_error_kind` は `"invalid_model"`、`ugoe_error_field(err, "errors")` は検査のエラーの数、
  `"error_codes"` は種類を `, ` でつないだもの。読み込みの `invalid_model` と同じ) を返し、何も変えない (T0461・T0469)。
- パーツの ID のポインタは、着せ替えの操作か `UgoePuppet` の破棄まで有効 (当たり判定の ID と同じ)。

## 受け手として進める (T0133・T0134・T0135)

```c
float values[PARAM_COUNT];                          /* ugoe_puppet_parameter_count の数。パラメータの番号の順 */
UgoePartOpacityInput parts[2];                      /* モーションのパーツの不透明度のカーブ (パーツの番号・値・重み) */
/* values を基準値として書き、物理 → リップシンク (音量 level・強さ strength。level が負なら当てない) →
   パーツの不透明度 (parts とポーズ) → 評価の順に dt 秒進める。カーブが無ければ NULL, 0 */
ugoe_puppet_follow(puppet, values, PARAM_COUNT, level, strength, parts, 2, dt, err);
ugoe_puppet_set_base_values(puppet, values, PARAM_COUNT, err); /* 基準値として書くだけ (評価しない) */

/* 指揮役の側 (T0454): 直前の ugoe_puppet_update の、受け手へ配る値を読む */
size_t n = 0;
ugoe_puppet_values_before_physics(conductor, values, PARAM_COUNT, &n, err);   /* 物理の段に入る直前の値 */
ugoe_puppet_lip_sync_applied(conductor, &level, &strength, err);              /* 当てていなければ level = -1 */
ugoe_puppet_part_opacities_applied(conductor, parts, 2, &n, err);             /* 足りなければ UGOE_ERROR_BUFFER_TOO_SMALL と数 */
ugoe_puppet_follow(follower, values, PARAM_COUNT, level, strength, parts, n, dt, err);
```

- `ugoe::Puppet::follow`・`set_base_values` と同じ (`docs/spec/motion.md#受け手として進める`)。スタジオの仮想カメラのような受け手が、指揮役から
  届いた `{dt, values, lip_sync, part_opacities}` を順に適用するために使う (ADR 0028 の 3.)。`values` は指揮役の物理の段に入る直前の値
  (`Puppet::values_before_physics`)、音量と強さは `Puppet::lip_sync_applied`、`UgoePartOpacityInput` の並びは
  `Puppet::part_opacities_applied` (カーブのあるパーツだけ。番号は `ugoe_puppet_part_index` と同じ。モデルに無い番号は無視する)。
  モーション・トラッキング・表情・目パチ・呼吸は通さない。
  リップシンクはこの `UgoePuppet` のモデルの対象に当て、`ugoe_puppet_set_lip_sync_level` の音量は使わない。
- `ugoe_puppet_set_parameter` の値は前のフレームの出力と同じなら基準値にならないが、これらは同じでも基準値にする (範囲に丸める)。
- 数がパラメータの数と違う・NaN を含む (音量と強さ、パーツの不透明度の値と重みも)・(当てるときの) 強さかパーツの不透明度の値と重みが無限大・
  `dt` が負か有限でないときは `UGOE_ERROR_INVALID_ARGUMENT` を返し、何も変えない (無限大は 0 × ∞ で NaN をパラメータに書きうるため)。
  `ugoe_puppet_follow` の後も、モーションのイベントは直前の `ugoe_puppet_update` のまま。wasm は `follow`・`set_base_values` (`Float32Array`)。
- 指揮役の側の口 `ugoe_puppet_values_before_physics`・`ugoe_puppet_lip_sync_applied`・`ugoe_puppet_part_opacities_applied` は
  `ugoe::Puppet::values_before_physics`・`lip_sync_applied`・`part_opacities_applied` と同じ値を、そのまま `ugoe_puppet_follow` に渡せる形で返す
  (T0454)。並びはホストのバッファ (`out`・`capacity`) に写し、数を `*out_count` に書く (`ugoe_puppet_tracking_targets` と同じ。
  足りなければ `UGOE_ERROR_BUFFER_TOO_SMALL` で数だけを書く)。借りたポインタではなく写すのは、指揮役は毎フレーム値を送り出すので
  コピーは避けられず、`update` で無効になるポインタの寿命の約束を増やさないため。リップシンクを当てていなければ音量は -1 (強さは 1)。
  直前が `ugoe_puppet_follow` なら、受け手に渡した値 (値域にクランプしたもの) を返すので、受け手からさらに中継できる。
  wasm は `values_before_physics()`・`lip_sync_applied()`・`part_opacities_applied_parts()` / `part_opacities_applied_values()`、
  Godot は `get_values_before_physics()`・`get_lip_sync_applied()`・`get_part_opacities_applied()`、
  Unity は `Puppet.ValuesBeforePhysics()`・`Puppet.LipSyncApplied()`・`Puppet.PartOpacitiesApplied()`。

## 範囲外 (後続)

- Windows と macOS での C の見本のビルドは確かめていない (CI は Linux だけ)。
- 描画 (`ugoe-render`) は C ABI に出さない。ホストのエンジンで描く (段階 B。`docs/portability.md`)。

## 関連タスク

- T0401 (`docs/tasks/T0401-*.md`)
- T0409 (着せ替え・ライティング・当たり判定・トラッキング・読み込みの警告)
- T0412 (エラーと警告の code を SDK にまとめる・モーションのイベント・物理の設定の差し替え)
- T0414 (パーツの不透明度・ポーズの表示の選択・効果の設定の差し替え)
- T0722 (トラッキング中のモーションの優先)
- T0133 (受け手として進める: 基準値として書く口)
- T0134 (受け手でもリップシンクを物理の後に当てる)
- T0489 (失敗を呼び出し側の `UgoeError` で受け取る)
- T0490 (毎フレームの描画の状態をまとめて受け取る)
- T0454 (指揮役の側の値を読む口)
- T0457 (モデルの読み込みの失敗の種類)
- T0489 (失敗の中身を呼び出し側の `UgoeError` で返す。スレッドローカルの状態を持たない。ADR 0031)
- T0491 (std の無い環境向けの静的ライブラリ。ADR 0031)
- T0494 (std の無い形のパニックのメッセージをホストに渡す `ugoe_set_panic_handler`)
