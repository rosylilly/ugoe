---
id: T0152
title: SDK とバインディングから読み込みの上限を指定できるようにする (C ABI・wasm・@ugoe/runtime)
status: done
milestone: M9
crates: [ugoe, ugoe-ffi, ugoe-wasm, bindings/web, docs]
depends_on: [T0150]
assignee: claude-worker-T0152
---

# T0152: SDK とバインディングから読み込みの上限を指定できるようにする (C ABI・wasm・@ugoe/runtime)

## 目的

T0150 で `ugoe-format` に入れる読み込みの上限 (`ReadLimits`) を、組み込む側が指定できるようにする。
上限は読む側の方針なので (T0150 の「方針」)、ゲームエンジン・ブラウザ・配信アプリなど、置かれた環境ごとに利用者が決められる必要がある。
指定しなければ既定値で読み、超えたら読み込みはエラーになる。

## 受け入れ条件

- [x] SDK (`ugoe` クレート): モデルとモジュールの読み込みで上限を指定できる (既定値で読む今の関数はそのまま残す)
- [x] C ABI: 上限を渡して読み込む関数を足し、ヘッダと README に書いた。上限を超えたときのエラーコードがある
- [x] wasm と `@ugoe/runtime`: 読み込みの関数のオプションで上限を指定できる。超えたときのエラーにコードがある (T0434 の読み込みのエラーのコードと揃える)
- [x] 各入口で、小さな上限を指定して超えるファイルを読むとエラーになるテストがある
- [ ] Unity と Godot に同じ口を出すかを決め、出さないなら後続のタスクにした
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0150 (ugoe-format の `ReadLimits` と、仕様の書き直し)
- T0434 (wasm の読み込みのエラーのコード)
- docs/portability.md

## メモ

- 2026-10-04 (claude-worker-T0152): 実装した。
  - **扱う範囲 (決めたこと)**
    - 展開の上限 (`ReadLimits`、T0150) は SDK・C ABI・wasm・`@ugoe/runtime` のすべてから指定できるようにした。
    - PNG の一辺の上限 (`ImageLimits`、T0009) は **wasm と `@ugoe/runtime` だけ** (`max_image_side`)。C ABI は画像をデコードせず
      エンコードされたまま渡す (デコードと上限はホストの責務) ので出さない。SDK (`ugoe`) も画像をデコードしないので出さない
      (使う側は `ugoe_format::image::decode_png(bytes, ImageLimits)` を直接呼ぶ。`ugoe` は `ugoe-format` の feature `png` を有効にしていない)。
    - JSON のモデル (`load_json`・`ugoe_puppet_create_from_json`) は展開も画像のデコードもしないので、上限を取らない。
  - **やったこと**
    - SDK: 読み込みは今までどおり `ugoe::format::read_bundle_with` / `module_file::read_module_with` (T0150) で上限を指定する (既定値の関数は変えていない)。
      超えたときの種類と補足の値を wasm と C ABI で揃えるため、`ugoe::limits` を足した: `ReadLimits`・`ReadLimit` の再エクスポートと
      `ReadLimitExceeded { chunk, limit, max }` (`CODE` = `"read_limit_exceeded"`、`from_error`、`fields()` = `limit`・`chunk`・`max`)。
    - C ABI: `UgoeReadLimits` (`max_chunk_decompressed`・`max_total_decompressed`、`uint64_t`)・`ugoe_read_limits_default()`・
      `ugoe_puppet_create_from_bundle_with_limits`・`ugoe_module_create_with_limits` (`limits` が NULL なら既定値) と、
      エラーコード `UGOE_ERROR_LIMIT_EXCEEDED` (10。種類 `read_limit_exceeded`、`ugoe_last_error_field` の `limit`・`chunk`・`max`) を足した。
      既存の `ugoe_puppet_create_from_bundle`・`ugoe_module_create` は `NULL` の上限で新しい関数を呼ぶだけにし、既定の上限を超えたときも
      `UGOE_ERROR_LIMIT_EXCEEDED` を返す (今までは `UGOE_ERROR_FORMAT`。README の「知らないエラーコードが返ることがある」の範囲で、ABI の版は上げない)。
      ヘッダを作り直し、README に節を足した。将来上限を足すときは `UgoeReadLimits` を変えず別の構造体と関数にする (ABI の版の規則)。
    - wasm: `load_bundle(bytes, limits_json?)`・`load_module(bytes, limits_json?)` (JSON `{"max_chunk_decompressed"?, "max_total_decompressed"?, "max_image_side"?}`、
      書かなかった項目は既定値、知らない項目・負の値・小数は `invalid limits` の例外)。JS の第 2 引数は省けるので、今の呼び出し側はそのまま動く。
      超えたら `load_bundle` は `name` が `"LoadError"`、`load_module` は今までどおり `"DressUpError"` の `Error` で、`code` は
      `read_limit_exceeded` (補足の値は SDK・C ABI と同じ) か `image_limit_exceeded` (`image`・`texture`・`width`・`height`・`max`)。
      補足の値は読み込みの警告 (`load_warnings_json`) と同じく、番号と大きさは数にした (C ABI は文字列)。そのため `ErrorInfo` の補足の値を
      `serde_json::Value` にし、`load.rs` に移した (`dress_up` から再エクスポート)。`LoadError` に `TextureTooLarge`・`NormalMapTooLarge` と `info()` を足した。
    - `@ugoe/runtime`: `LoadLimits` (`maxChunkDecompressed`・`maxTotalDecompressed`・`maxImageSide`)・`limitsJson`・`isLimitError`・`UgoeLimitError*` の型
      (`src/limits.ts`)、`loadModel(puppet, data, { limits })`・`fetchModel(puppet, url, init, { limits })`、`<ugoe-avatar>` の `loadLimits` プロパティ
      (属性は無い。次の読み込みから効く)。`UgoeDressUpErrorCode` に 2 つのコードを足した。スタブの型と README を直した。
    - 文書: `crates/ugoe-wasm/README.md`・`crates/ugoe-ffi/README.md`・`crates/ugoe/README.md`・`bindings/web/README.md`・
      `docs/spec/file-formats.md#読み込みの上限`・`docs/portability.md` の C ABI の規約。
    - テスト: SDK (`limits.rs`: バンドルとモジュールを小さな上限で読むと種類と補足の値が出る)、C ABI (`tests/limits.rs`: バンドルとモジュールが
      `UGOE_ERROR_LIMIT_EXCEEDED` になり種類と補足の値を持つ・NULL と既定値は読める・上限の外の失敗は今までどおり)、wasm のネイティブのテスト
      (`load.rs`: 上限の JSON・展開の上限・PNG の一辺の上限 (テクスチャと法線マップ)。`dress_up/tests.rs`: モジュールは `invalid_file` でなく上限の種類)、
      `@ugoe/runtime` (`limits.test.ts`、`avatar.test.ts` の `loadLimits` が次の読み込みに渡る)。
  - **T0434 との関係**: T0434 (読み込みの失敗すべてにコードを付ける) はまだ todo なので、ここでは上限を超えた失敗だけに `name` = `"LoadError"` と
    `code` を付け、ほかの `load_bundle` の失敗は今までどおりコードの無い `Error` のまま。T0434 では `LoadError::info()` を全種類に広げ、
    `name` を `"LoadError"` にそろえれば、ここで決めた 2 つのコードと補足の値をそのまま使える (`invalid_file` などと並ぶ)。
    T0434 の例の `texture_too_large` (デバイスの上限、`check_texture_sizes`) とは別のコードにした (読む側の上限は `image_limit_exceeded`)。
  - **範囲外の変更 (小さいので行った)**: `scripts/check-ffi-bindings.mjs` の Unity の OMIT に新しい 3 つの関数を理由付きで足した (`just refs` のため)。
  - **Unity と Godot**: どちらにも今回は出さない (既定値で読む)。
    - Unity: C ABI の新しい関数の P/Invoke と、`Puppet`・モジュールの読み込みの引数に上限を足す作業が要る。`UgoeResult` の enum に
      `LimitExceeded = 10` も足す (今は知らない値として `UgoeException` になるはず。確かめていない)。
    - Godot: C ABI ではなく SDK を直接使うので、`read_bundle_with` に上限を渡し、PNG のデコードに `ImageLimits` を渡す口 (リソースの読み込みの設定) を足す作業。
  - **やらなかったこと・確かめていないこと**
    - ブラウザで wasm の `load_bundle(bytes, limitsJson)` を実際に呼ぶテストは書いていない (wasm-bindgen の境界は `just wasm-check` のビルドだけ。
      中身は `LoadLimits::from_json`・`from_bundle_with`・`LoadError::info` のネイティブのテストで確かめた)。
    - エディタ・ビューア・スタジオ・`@ugoe/player` は上限を渡していない (既定値で読む。スタブの型も第 2 引数なしのままで型は合う)。
      ビューアの読み込みの失敗の訳は T0434 で `code` から行う。
  - **確かめたこと**: main に rebase した後で `just ci` と `just wasm-check` が通った。`just ffi-header` のヘッダは最新、`just refs` も通る。
  - **受け入れ条件の「Unity と Godot」**: 出さないと決めたが、新しいタスクは起票していない (ワーカーは起票しない決まり)。下の起票の候補の 2 つを起票すれば満たす。
  - **起票の候補**
    - Unity の UPM パッケージから読み込みの上限を指定する (`UgoeReadLimits`・`*_with_limits`・`UgoeResult.LimitExceeded`。`scripts/check-ffi-bindings.mjs` の OMIT を外す)。
    - Godot の GDExtension から読み込みの上限 (展開と PNG の一辺) を指定する。
    - (T0434 の中で) `@ugoe/player` などアプリの読み込みで `read_limit_exceeded`・`image_limit_exceeded` を訳す。

