# ugoe-ffi

**層:** L5 バインディング (依存してよいクレートは `scripts/check-layers.mjs` を参照)

C ABI。Unity・Unreal・Godot などから使う。規約は [docs/portability.md](../../docs/portability.md#c-abi-ugoe-ffi-の規約)。

## 守ること

- `unsafe` はこのクレートでだけ許可する
- パニックを FFI の境界の外へ漏らさない (公開する関数はすべて `error::guard` を通す)
- ABI を壊したら `ABI_VERSION` を上げる
- 公開する関数・型を変えたら `just ffi-header` でヘッダを作り直してコミットする

## 使い方

ヘッダは [`include/ugoe.h`](include/ugoe.h)、ライブラリは `cargo build -p ugoe-ffi --release` で
`target/release/` にできる `libugoe_ffi.{a,so,dylib}` (Windows では `ugoe_ffi.{lib,dll}`)。
静的ライブラリをリンクするときは、Linux ではさらに `-lm -lpthread -ldl` が要る。

```c
UgoePuppet *puppet = NULL;
if (ugoe_puppet_create_from_bundle(bytes, len, &puppet) != UGOE_OK) {
    fprintf(stderr, "%s\n", ugoe_last_error());
}
ugoe_puppet_set_parameter(puppet, index, value);
ugoe_puppet_update(puppet, dt);
const float *xy; size_t n;
ugoe_puppet_positions(puppet, drawable, &xy, &n);  /* 次の update まで有効 */
ugoe_puppet_destroy(puppet);
```

動く見本は [`examples/c/sample.c`](examples/c/sample.c) (`just ffi-check` でビルドして実行する)。

- 関数は `UgoeResult` (`UGOE_OK` = 0) を返し、失敗したら同じスレッドの `ugoe_last_error()` でメッセージを得る。
- 内部でパニックが起きたら `UGOE_ERROR_PANIC` を返す。そのオブジェクトの状態は不定なので破棄する。
- 頂点位置はホストのバッファへのコピー (`ugoe_puppet_copy_positions`) と、内部のバッファの借用
  (`ugoe_puppet_positions`。次の `ugoe_puppet_update` か破棄まで有効) の両方がある。
- テクスチャはエンコードされたままのバイト列 (PNG・KTX2) を渡す。デコードと乗算済みアルファへの変換はホストが行う
  ([docs/spec/rendering.md](../../docs/spec/rendering.md))。
- 1 つの `UgoePuppet` を複数のスレッドから同時に使ってはならない。

## ヘッダの生成

ヘッダは [cbindgen](https://github.com/mozilla/cbindgen) の CLI で生成する (設定は `cbindgen.toml`)。
cbindgen は MPL-2.0 なので、このクレートの依存 (`build-dependencies`・`dev-dependencies` を含む) には入れず、
ワークスペースの外のツールとして使う。版は `scripts/ffi.mjs` の `CBINDGEN_VERSION` と CI (`.github/workflows/ci.yml` の `cbindgen@...`) で揃え、
違う版や未導入なら `just ffi-check` / `just ffi-header` が入れ方を案内して止まる。ヘッダには cbindgen の版を書かない。

`just ffi-check` は cbindgen と C コンパイラ (`cc`。`CC` で変えられる) が要るので `just ci` には含めず、CI の `rust` ジョブで別に実行する。

```sh
cargo install cbindgen --version 0.29.4 --locked
just ffi-header   # include/ugoe.h を作り直す
just ffi-check    # ヘッダが最新かの検査と、C の見本のビルド・実行
```

## 範囲外 (後続)

着せ替え (モジュールの付け外し)・ライティング・当たり判定・トラッキング・読み込みの警告は、まだ C ABI に出していない
(T0401 のメモ)。

## 関連タスク

- T0401 (`docs/tasks/T0401-*.md`)
