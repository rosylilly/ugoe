# 0031: ランタイム層を no_std + alloc で使えるようにし、C ABI をスレッドと std に依存しない形にする

- 状態: Accepted
- 日付: 2026-10-06 (提案と決定)
- 関連: T0179〜T0184、T0489〜T0491、ADR 0017 (1 つのコアからの配布)、ADR 0030 (KTX2 のデコード)、
  [portability.md](../portability.md)、[portability.md#c-abi-ugoe-ffi-の規約](../portability.md#c-abi-ugoe-ffi-の規約)

## 背景

ランタイムは段階 B (頂点を渡し、描画はエンジンに任せる) を第一級に扱い、`ugoe-ffi` の C ABI から Unity・Unreal・Godot などに組み込む
([portability.md](../portability.md))。今後の組み込み先には、次のような環境がある。

- **Rust の `std` が無い環境。** ゲーム機 (コンシューマー機) の SDK は Rust の `std` を公式には持たない。持ち込めるのは `core` と `alloc` と、
  SDK の C ライブラリ (`malloc`・`free`・`abort` など) くらい。組み込み機器も同じ。Unity・Unreal でゲーム機向けにビルドするときも、
  `ugoe-ffi` の静的ライブラリがその環境で動く必要がある。
- **呼び出しのたびに OS のスレッドが変わりうるホスト。** Go (cgo・purego) の goroutine は、呼び出しと呼び出しの間に別の OS のスレッドへ移る。
  C# の async の続きや、スレッドプールから呼ぶホストも同じ。

今の実装には、これらの環境を妨げるものがある。

1. ランタイム層のクレートは `std` に依存している。多くは `core`・`alloc` にあるもの (`fmt`・`Arc`・`mem`・`BTreeSet`) だが、
   `HashMap`・`HashSet`・`Mutex` (`ugoe-motion` のトラッキング)・`std::io::Read` (`ugoe-format` のバンドルと画像)・`f32` の数学関数
   (`sin`・`powf` など。`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe`) は `std` にしかない。
   [portability.md](../portability.md) は no_std を「未決 (需要があれば対応する)」としていた。
2. C ABI のエラーはスレッドローカル (`ugoe_last_error()`・`ugoe_last_error_kind()`・`ugoe_last_error_field*`) にある。
   失敗した関数と `ugoe_last_error()` が別のスレッドで動くと、違うスレッドの値 (または NULL) を読む。スレッドローカルは `std` が無い環境でも使えない。
3. C ABI はパニックを `catch_unwind` で捕まえて `UGOE_ERROR_PANIC` にしている。`std` の無い環境では `panic=abort` になるので、
   パニックはアプリ全体を止める。
4. 描画の状態は描画要素ごとの関数 (`ugoe_puppet_drawable_state`・`ugoe_puppet_positions` など) で取る。境界を越える 1 回の呼び出しが重いホスト
   (cgo は 1 回あたり数十 ns、マネージドなホストはマーシャリングがある) では、描画要素の数に比例して呼び出しが増える。

## 決定

### ランタイム層を no_std + alloc でビルドできるようにする

- 対象は `ugoe-core`・`ugoe-model`・`ugoe-runtime`・`ugoe-physics`・`ugoe-motion`・`ugoe-format`・`ugoe` (SDK、`render` を外した形)・`ugoe-ffi`。
  `ugoe-render`・`ugoe-wasm` は対象にしない。
- 各クレートは `#![no_std]` と `extern crate alloc;` で書き、既定で有効な feature `std` を持つ。`std` を外しても公開 API の形は変えない
  (エラー型は `core::error::Error` を実装すれば足りる。`std` があるときだけの便利な関数は `std` の feature で足す)。
  既定の feature は今と同じなので、今の利用者 (crates.io・Unity・Godot・wasm) には影響しない。
- `HashMap`・`HashSet` は `BTreeMap`・`BTreeSet` か、決まったハッシュ関数を使う `hashbrown` に置き換える (どちらを使うかは各タスクで、
  決定性と速さで選ぶ。`std` の `RandomState` は no_std に無く、評価の決定性にも要らない)。
- 外部の依存は `default-features = false` で使い、`std` の feature から各依存の `std` を有効にする。ワークスペースの依存
  (`[workspace.dependencies]`) で既定の feature を切らないと、メンバーの `default-features = false` が効かないので、ルートで切って、
  ツール層のクレートが必要な feature を書く。
- 依存の扱い (確かめた範囲。実際に使えるかは各タスクで検査する):
  - `glam`・`serde`・`serde_json` (`alloc`)・`thiserror` (2.x)・`ruzstd`: no_std で使える。
  - `rmp-serde`: no_std で使えるか分からない。使えなければ、MessagePack の読み書きを `rmp` (低い層) で書くか、別のクレートに替える
    (形式は変えない。[file-formats.md](../spec/file-formats.md) のバイト列が同じであることをテストで確かめる)。
  - `png`・`bcdec_rs`: もともと optional。`std` の feature の側に置く (テクスチャのデコードはもともとホストの役目。
    [crates/ugoe-ffi/README.md](../../crates/ugoe-ffi/README.md))。
- **検査:** `aarch64-unknown-none` (`std` が無く、`core` と `alloc` がビルド済みで配られている tier 2 のターゲット) で、
  対象のクレートを `--no-default-features` でビルドする `just nostd-check` を作り、CI で回す。このターゲットには `std` そのものが無いので、
  どこかで `std` に依存すると (feature の統合の漏れを含めて) ビルドが失敗する。

### 数学関数は `libm` に寄せる

- `f32` の `sin`・`cos`・`powf`・`exp`・`atan2` などは `std` にしかない (`sqrt`・`abs`・`floor` などの一部は `core` でも使える版が増えているが、
  そろってはいない)。ランタイム層は数学関数を `ugoe-core` の小さなモジュール (`ugoe_core::math`) から呼び、その中身は `std` の有無に
  かかわらず **常に** `libm` (純粋な Rust の実装) を使う。`glam` も `libm` の feature で同じにする。
- 常に `libm` にするのは、`std` の有無でも OS (Windows・macOS・Linux の C の数学ライブラリ) でも、同じ入力から同じビット列の出力を得るため。
  今は OS の数学ライブラリの違いを適合テストの許容誤差で吸収している。
- 出力が今と少し変わるので、参照の出力 (適合テスト・spec-check の期待値・スナップショット) が許容誤差の中に収まるかを確かめ、
  収まらなければ作り直す。速さも測り、目に見えて遅くなる関数があればその関数だけ対策を考える。
- 追記 (T0193): 平方根 (`ugoe_core::math::sqrt`) は、`std` があれば `std` の `f32::sqrt` (CPU の命令) を使う。安定版の Rust の `libm` は
  wasm32 で `f32.sqrt` の命令を使えずソフトウェアで反復し、ブラウザで物理の段が約 2 倍遅かった。平方根は IEEE で正しく丸めると決まっているので、
  `std` の有無でも OS でも結果のビット列は変わらず、この決定の目的 (同じ入力から同じビット列) は保たれる。

### ランタイム層ではパニックを起こさない

- 外から来るデータ (ファイル・ホストの引数) のどんな値でも、ランタイム層はパニックしない。今もこれは目標だった
  ([portability.md#ホストから来る数](../portability.md#ホストから来る数)) が、`std` の無い環境では `catch_unwind` が使えず、
  パニックがアプリ全体を止めるので、守るべき性質として扱う。
- 検査の方法: ランタイム層のクレートのライブラリのコードで `clippy::expect_used`・`clippy::panic`・`clippy::unreachable` などを有効にする
  (`unwrap_used` はワークスペースで有効)。添字の範囲外と算術の桁あふれは lint で網羅できないので、壊れた入力・巨大な値・NaN を与えるテストで確かめる。
- C ABI の `catch_unwind` は `std` がある形では残す (後ろの守り)。`std` が無い形では、パニックしたらホストの `abort` を呼ぶ。

### C ABI のエラーはスレッドローカルに置かない

- 失敗の中身 (メッセージ・種類・補足の値) は、呼び出し側が渡す不透明なオブジェクト `UgoeError` に書く。
  `UgoeError` は `ugoe_error_create` / `ugoe_error_destroy` で作って壊し、`ugoe_error_message(e)`・`ugoe_error_kind(e)`・
  `ugoe_error_field(e, key)`・`ugoe_error_field_count(e)`・`ugoe_error_field_key_at(e, i)`・`ugoe_error_field_value_at(e, i)` で読む。
  読める値 (メッセージ・種類・補足の値) は今の `ugoe_last_error*` と同じで、wasm と同じ `code` を使う。
- **スレッドローカルの `ugoe_last_error*` は廃止する。** 失敗の中身を受け取る手段を 1 つにする (2 つあると、どちらを使うべきか・
  両方に同じ値が入るかで混乱する)。C ABI はグローバルな状態もスレッドローカルな状態も持たず、オブジェクトだけで完結させる。
  - 1.0 より前で、C ABI の利用者はリポジトリの中 (C の見本・適合テスト・Unity のバインディング) だけなので、ABI を壊してよい。
    `ABI_VERSION` を上げ、リポジトリの中の利用者は同じ PR で移す。
- 失敗しうる関数は、結果コード (`UgoeResult`) を返すのは今のままで、`UgoeError*` を引数に取る (NULL なら中身を書かない。
  結果コードだけで足りるホストのため)。引数の位置と名前はすべての関数で揃える (細かい形は T0489 で決める)。
- スレッドについての約束:
  - `UgoeError` はどのスレッドで作り、書かせ、読んでもよい (スレッドに結び付かない)。ただし 1 つの `UgoeError` を同時に複数のスレッドから
    使ってはならない (呼び出しごとに 1 つ持つか、スレッドごとに 1 つ持つ)。
  - 呼び出しの間で OS のスレッドが変わるホスト (Go の goroutine・C# の async の続き) でも、失敗した呼び出しに渡した `UgoeError` から正しい中身を読める。
  - `UgoePuppet` などのオブジェクトの約束 (同時に複数のスレッドから使わない) は今と同じ。
- 読み出しの関数が返す文字列は、その `UgoeError` を次に書かせるか壊すまで有効とする。

### 毎フレームの描画の状態はまとめて渡す

描画の情報を、読む目的 (時機) で 2 つに分け、どちらも読み方を 1 つにする。

| 目的 | 読むもの | 形 |
| --- | --- | --- |
| モデルの形を知る (読み込みと着せ替えの後に 1 回) | 描画要素の数と情報 (`ugoe_puppet_drawable_count`・`_info`)・マスクの参照 (`_mask`)・UV・インデックス・テクスチャと法線マップ・ライティングを受けるか (`_lit`・`ugoe_puppet_uses_lighting`) | 描画要素ごと (今のまま) |
| このフレームの状態を知る (`update` の後に毎回) | 頂点の位置・表示・不透明度・色・合成モード・マスクの効き具合・描画順・変更のフラグ | 1 回の呼び出しでまとめて受け取る形だけ |

- 毎フレームの状態を描画要素ごとに読む関数 (`ugoe_puppet_drawable_state`・`ugoe_puppet_positions`・`ugoe_puppet_copy_positions`・
  `ugoe_puppet_render_order`・`ugoe_puppet_render_order_changed`) は、まとめて受け取る形に置き換えて廃止する。
  同じ情報を読む手段を 1 つにするため。1.0 より前なので ABI を壊し (`ABI_VERSION` を上げる)、リポジトリの中の利用者は同じ PR で移す。
- まとめて受け取る形は、借用 (次の `update`・着せ替え・破棄まで有効) と、ホストのバッファへのコピー (足りなければ
  `UGOE_ERROR_BUFFER_TOO_SMALL` で要る数を返す) の両方を持つ。借用はコピーを省きたいホスト、コピーは借用の寿命を扱いにくいホスト
  (マネージドな言語) のためで、置き場所の違いであって読む情報は同じ。
- 頂点の位置は、描画要素ごとの範囲 (先頭と数) を持つ 1 つの連続した配列にする。変更のフラグも一緒に渡し、変わっていない描画要素の
  位置をホストが写さずに済むようにする。
- これは呼び出しの回数とホストの書きやすさのための変更で、描き方は変えない。ホストは今と同じく、描画順に従って各描画要素を描く
  (描画のまとめ方 ([ADR 0016](0016-draw-batching.md)) はレンダラ・エンジンの側の話)。
- パーツの不透明度 (`ugoe_puppet_part_opacity` など) は描画の状態ではなく、パーツについての問い合わせなので、ここでは扱わない。

### std の無い環境への C ABI

- `ugoe-ffi` に `std` を外した形 (staticlib) を作る。アロケータ (`#[global_allocator]`) は、ホストの C ライブラリの
  `malloc`・`free` (と、整列が要るときの `aligned_alloc` か `posix_memalign`) を呼ぶ。パニックの処理 (`#[panic_handler]`) は `abort` を呼ぶ。
  どちらもホストのリンクで解決するので、SDK ごとの知識はクレートに持たない。
- `std` の無い形は、`aarch64-unknown-none` で staticlib をビルドし、C の見本とリンクできる (未解決のシンボルが `malloc`・`free`・`abort`
  などホストの C ライブラリのものだけである) ことを CI で確かめる。実機での確認は、組み込み先 (ゲーム機の SDK を持つ人) に任せる。

## 検討した代替案

| 案 | 利点 | 欠点 |
| --- | --- | --- |
| no_std に対応せず、組み込み先ごとにランタイムを別の言語で書き直す (段階 C) | Rust を持ち込めない環境にも届く | 3.8 万行ほどを言語ごとに二重に保守する。仕様の変更のたびに追いかける。救えるのはその言語のホストだけ |
| `std` のまま、ゲーム機の SDK 向けに `std` が使う libc の関数を差し替える (Go の Hitsumabushi と同じ発想) | 今のコードを変えずに済む | 差し替える関数の集合が Rust の版で変わる。スレッドローカルと unwind が要る。SDK ごとに作業が要る |
| 数学関数は `std` があれば `std`、無ければ `libm` を使う | 今の出力が変わらない | `std` の有無で出力が変わり、ゲーム機と PC で結果が揃わない。適合テストで両方を見る必要がある |
| エラーの読み出しを `runtime.LockOSThread` などホストの側で解決してもらう | C ABI を変えずに済む | ホストごとに注意が要り、誤りやすい。`std` の無い形のスレッドローカルの問題は残る |
| 毎フレームの状態をまとめて受け取る形を足し、描画要素ごとの関数も残す | 今の利用者の書き換えが要らない。一部の描画要素だけ読むホストに便利 | 同じ情報を読む手段が 2 つになる。毎フレームの読み方としては、描画要素ごとの呼び出しが要る理由が無い (どのホストも全部を描画順に読む) |
| `UgoeError` を足し、`ugoe_last_error*` も残す (足すだけで ABI を壊さない) | 今の利用者の書き換えが要らない | 同じ目的の手段が 2 つになり、どちらを使うか・両方に同じ値が入るかで混乱する。`std` の無い形だけ関数が欠けるのでヘッダが 2 通りになる。1.0 より前で利用者はリポジトリの中だけなので、壊さない利点が小さい |

## 結果

- C ABI を壊す (`ABI_VERSION` を上げる)。C の見本・適合テスト・Unity のバインディングを、`UgoeError` と毎フレームの状態をまとめて受け取る形に
  移す必要がある。代わりに、C ABI はスレッドに結び付く状態を持たなくなり、`std` の有無でヘッダが変わらず、同じ情報を読む手段が 1 つになる。
- ゲーム機・組み込み機器のように `std` の無い環境で、`ugoe-ffi` を静的ライブラリとして組み込める道ができる。Unity・Unreal の
  ゲーム機向けのビルドと、cgo などから使う新しいホストが同じものを使う。
- 数学関数を `libm` に寄せることで、OS を問わず評価の結果がビット単位で揃う。参照の出力の作り直しが一度要るかもしれない。
- ランタイム層のクレートは `alloc::` と `core::` を明示して書く必要があり、`HashMap` が使えない。
- 依存を足すときに no_std で使えるかを確かめる手間が増える (`just nostd-check` が検出する)。
- 見直すきっかけ: `rmp-serde` などの依存が no_std で使えず、置き換えの手間が大きいと分かったとき。ゲーム機の実機で、
  `aarch64-unknown-none` の検査では見つからない問題 (TLS・アトミック・浮動小数点の設定など) が見つかったとき。

## 追記: ugoe-format の MessagePack (T0182)

`rmp-serde` 1.3 は `std::io` を前提に書かれ、`std` を外す feature を持たない (中の `rmp` は `std` を外せる)。そこで、
`ugoe-format` の中に MessagePack の serde の実装 (`msgpack` モジュール。テストを含めて 1,300 行ほど) を書き、
`rmp-serde` を置き換えた。新しい依存は足していない (`rmp` も使わない。`rmp` は自分の `std` の feature の有無で公開 API の形が変わり、
ワークスペースでは開発時の依存の `rmp-serde` が `std` を有効にするので、両方の形でビルドできるように書く手間のほうが大きい)。

- 書き出すバイト列と読み込みの結果 (値か誤りか・入れ子の深さの上限の数え方・`Option` と enum の判別) は `rmp-serde` の
  `to_vec_named` と `Deserializer::from_read_ref` と同じにした。`rmp-serde` を開発時の依存に残し、整数・文字列・配列・マップの長さの境目、
  serde の enum と構造体の形、壊したバイト列、`assets/samples` と `assets/compat` のすべてのチャンクで、両者の結果が同じことを
  単体テストで比べる。`assets/` のすべてのファイルを読み直して書き出したバイト列 (無圧縮と zstd) も、置き換えの前後で同じだった。
- 書かないもの: `i128`・`u128` と MessagePack の拡張型 (モデルの型は使わない。拡張型は読むことはできる)。
- `ruzstd` は `std` 無しでも使える (`ruzstd::io::Read` が `std` の有無で `std::io::Read` か自前の同じ形の trait になる)。
- PNG (`png`) と BCn (`bcdec_rs`) のデコードは feature `png` (`std` を含む) の側に置いた。`std` 無しで読める画像は、
  圧縮していない 8bit の KTX2 (超圧縮は無しか zstd) だけ。
- 置き換えの手間は小さく、この ADR の決定は変えない。

## 追記: std の無い形の C ABI (T0491)

`ugoe-ffi` に既定で有効な feature `std` を足し、外すと `#![no_std]` の静的ライブラリになるようにした
([crates/ugoe-ffi/README.md](../../crates/ugoe-ffi/README.md#std-の無い環境))。決定を具体にしたところと、その理由:

- **アロケータ:** `malloc`・`free` と `aligned_alloc` に加え、`calloc` (`alloc_zeroed`) と `realloc` も呼ぶ。LLVM は確保と 0 埋めを
  `calloc` に置き換えるので、呼ばなくてもシンボルは要る。それなら明示して、0 埋めと伸長を C ライブラリに任せる (`realloc` は
  `Vec` の伸長で写しを省ける)。`malloc` の整列は std の `System` と同じく 64 ビットのターゲットで 16 バイト、ほかで 8 バイトとみなし、
  それより大きな整列 (と、大きさより大きな整列) だけ `aligned_alloc` にする。`aligned_alloc` は C11 にあり、POSIX の
  `posix_memalign` よりゲーム機の SDK の C ライブラリが持っている見込みが高いので選んだ (大きさは整列の倍数に切り上げる)。
- **パニックの処理:** `abort` を呼ぶだけで、メッセージは出さない (出す先がホストごとに違い、書式化のコードも要るため)。
  確保の失敗 (`handle_alloc_error`) も同じパニックの処理に来る。
- **プロファイル:** ワークスペースの dev・release を変えず、`[profile.nostd]` (release を継ぎ、`panic = "abort"`) を足した。
  `aarch64-unknown-none` は既定で `panic = "abort"` だが、ゲーム機のターゲットはそうとは限らないため。
- **置き場所:** アロケータとパニックの処理は `ugoe-ffi` の中の `#[cfg(all(not(feature = "std"), not(test)))]` のモジュールに置いた。
  別のクレートに分けると、組み込む人が 2 つをリンクする必要があり、std の形と取り違えやすい。代わりに、std を外した `ugoe-ffi` は
  ほかの Rust のクレートから使えず、結合テストも動かない (単体テストは std の prelude で動く)。
- **検査:** `just nostd-check` が `aarch64-unknown-none` の静的ライブラリをフリースタンディングの C の見本と rust-lld でリンクし、
  未解決のシンボルが `abort`・`aligned_alloc`・`calloc`・`free`・`malloc`・`realloc` だけであることを確かめる (`memcpy` などは
  このターゲットの `compiler_builtins` が持つ)。`just ffi-check` はホスト (Linux・macOS) 向けの std の無い形をホストの C ライブラリと
  リンクして見本のモデルで動かす。ホスト向けに配られる `core` は巻き戻しありでビルドされているので `rust_eh_personality` が残り、
  見本の側で形だけ定義した。実機では確かめていない。

## 追記: パニックのメッセージをホストに渡す (T0494)

上の T0491 の追記では、std の無い形のパニックの処理は `abort` を呼ぶだけでメッセージを出さないとした。ゲーム機や組み込み機器では
デバッガをつなげないことも多く、どこで何が起きたかの手がかりが残らないので、ホストが登録する関数にメッセージを渡す口を足した
([crates/ugoe-ffi/README.md](../../crates/ugoe-ffi/README.md#パニックのメッセージをホストに渡す-t0494))。

- **決定:** `ugoe_set_panic_handler(void (*fn)(const char *message, size_t len, void *user), void *user)` を足した。パニックの処理は、
  登録された関数があれば `panicked at <file>:<line>:<column>: <message>` を固定の大きさ (255 バイトと NUL) のスタックのバッファに
  `core::fmt::Write` で書いて呼び、その後に `abort` する。確保しないので確保の失敗のパニックでも呼べる。入りきらない分は文字の境界で捨て、
  書式化の失敗は無視し、パニックの処理の中でもう一度パニックしたら関数を呼ばずに `abort` する。`ABI_VERSION` を 3 に上げた。
- **登録はグローバルな状態:** C ABI はグローバルな状態を持たない (T0489) が、パニックの処理はもともとプロセスに 1 つなので、登録だけは
  `AtomicPtr` に置く。ポインタの幅のアトミックの読み書きだけを使うので、64 ビットのアトミック操作の無いターゲットでも使える
  (`armv6k-nintendo-3ds` で、未解決のシンボルが増えないことを確かめた)。
- **`std` がある形:** 関数は同じヘッダにあるが、登録した関数は呼ばない。パニックは `catch_unwind` で `UGOE_ERROR_PANIC` になり、
  メッセージは `UgoeError` に書くので足りる。std のパニックのフック (`std::panic::set_hook`) にはつながない (プロセスの既定の出力を
  変えてしまい、std の形ではアプリを止めないので「渡してから `abort`」と意味が揃わないため)。
- **選ばなかった案:**
  - ホストがリンク時に定義するシンボル (`ugoe_host_panic`) を呼ぶ。グローバルな状態は要らないが、使わないホストにも定義を強いる
    (Rust の stable には弱いシンボルが無い)。
  - 今のまま `abort` だけにする。調べる手がかりがデバッガに限られる。
