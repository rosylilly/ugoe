# 0031: ランタイム層を no_std + alloc で使えるようにし、C ABI をスレッドと std に依存しない形にする

- 状態: Proposed
- 日付: 2026-10-06 (提案)
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

### ランタイム層ではパニックを起こさない

- 外から来るデータ (ファイル・ホストの引数) のどんな値でも、ランタイム層はパニックしない。今もこれは目標だった
  ([portability.md#ホストから来る数](../portability.md#ホストから来る数)) が、`std` の無い環境では `catch_unwind` が使えず、
  パニックがアプリ全体を止めるので、守るべき性質として扱う。
- 検査の方法: ランタイム層のクレートのライブラリのコードで `clippy::expect_used`・`clippy::panic`・`clippy::unreachable` などを有効にする
  (`unwrap_used` はワークスペースで有効)。添字の範囲外と算術の桁あふれは lint で網羅できないので、壊れた入力・巨大な値・NaN を与えるテストで確かめる。
- C ABI の `catch_unwind` は `std` がある形では残す (後ろの守り)。`std` が無い形では、パニックしたらホストの `abort` を呼ぶ。

### C ABI のエラーはスレッドローカルに置かない

- 失敗の中身 (メッセージ・種類・補足の値) を、呼び出し側が渡す不透明なオブジェクト `UgoeError` に書く関数を足す。
  `UgoeError` は `ugoe_error_create` / `ugoe_error_destroy` で作って壊し、`ugoe_error_message(e)`・`ugoe_error_kind(e)`・
  `ugoe_error_field(e, key)`・`ugoe_error_field_count(e)` などで読む。読む関数の名前と意味は今の `ugoe_last_error*` と揃える。
- 今の関数は残し、スレッドローカルにも今と同じく記録する (足すだけなので ABI の版は上げない)。新しい関数の形 (今の関数に
  `UgoeError*` を足した別名を全部の関数に作るか、`ugoe_error_take_last` のように最後の失敗を移す形にするか) は T0489 で決める。
  決める基準は、Go の goroutine のように **呼び出しの間で OS のスレッドが変わっても** 正しい失敗を読めることと、`std` の無い形で
  スレッドローカルを使わずに済むこと。
- `std` の無い形の C ABI は、スレッドローカルの `ugoe_last_error*` を持たない (ヘッダの宣言も feature で分ける)。

### 1 フレーム分の描画の状態をまとめて渡す

- 1 回の呼び出しで、すべての描画要素の状態 (表示・不透明度・色・合成モード・マスク・描画順と変更のフラグ) と、
  頂点の位置をまとめて渡す関数を足す (借用の形とホストのバッファへのコピーの形)。描画要素ごとの関数は残す。
- 頂点の位置は描画要素ごとの範囲 (先頭と数) を持つ 1 つの連続した配列にし、変更の無い描画要素を飛ばせるよう、変更のフラグも一緒に渡す。

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

## 結果

- ゲーム機・組み込み機器のように `std` の無い環境で、`ugoe-ffi` を静的ライブラリとして組み込める道ができる。Unity・Unreal の
  ゲーム機向けのビルドと、cgo などから使う新しいホストが同じものを使う。
- 数学関数を `libm` に寄せることで、OS を問わず評価の結果がビット単位で揃う。参照の出力の作り直しが一度要るかもしれない。
- ランタイム層のクレートは `alloc::` と `core::` を明示して書く必要があり、`HashMap` が使えない。
- 依存を足すときに no_std で使えるかを確かめる手間が増える (`just nostd-check` が検出する)。
- 見直すきっかけ: `rmp-serde` などの依存が no_std で使えず、置き換えの手間が大きいと分かったとき。ゲーム機の実機で、
  `aarch64-unknown-none` の検査では見つからない問題 (TLS・アトミック・浮動小数点の設定など) が見つかったとき。
