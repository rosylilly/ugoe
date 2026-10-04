---
id: T0434
title: wasm の読み込みのエラーにコードを付ける
status: done
milestone: M9
crates: [ugoe-wasm, packages, bindings/web, docs]
depends_on: [T0424]
assignee: claude-worker-T0434
---

# T0434: wasm の読み込みのエラーにコードを付ける

## 目的

`WebPuppet::load_json` / `load_bundle` の失敗は `JsError` (英語の文だけ) で、`code` が無い。ビューア (`packages/player/src/player.svelte.ts` の `load`) は
その英文をそのまま利用者に見せている (ADR 0029 の「利用者に見せる文言はコードから訳す」に反する)。着せ替えの `DressUpError` (`name` と `code`) と
同じ形で、読み込みの失敗にも機械が読むコードを付け、ビューアがコードから訳せるようにする。T0424 (ugoe-wasm のレビュー) で見つけた。

## 受け入れ条件

- [x] `load_json` / `load_bundle` の失敗が、`name` と `code` を持つ `Error` になる (例: `invalid_file`・`unsupported_version`・`compile`・`texture`・`texture_too_large`・`device_lost`。C ABI の `ugoe_last_error_kind` と揃えられるものは揃える)
- [x] コードの一覧を `crates/ugoe-wasm/README.md` に書き、ネイティブのテストで `LoadError` → コードの対応を確かめる
- [x] ビューア (`@ugoe/player`) が読み込みの失敗をコードから訳して出す (知らないコードは英文のまま)
- [x] `bindings/web` の型の写し (`stub/ugoe_wasm.d.ts`) と README を直す (変わる場合)
- [x] `just ci` が通る

## 参考

- `crates/ugoe-wasm/src/web.rs` (`load_json`・`load_bundle`・`load`)、`crates/ugoe-wasm/src/web/dress_up.rs` (`dress_up_error`)
- docs/adr/0029-shared-ui-and-i18n.md

## メモ

- T0019: T0729 の起票の候補 (ugoe-wasm の読み込みのエラーをコードと引数で返し、`@ugoe/player` が訳す) はこのタスクに含まれる。
  ビューアとスタジオは `format.*` の訳 (T0729 で足した) を持つので、`@ugoe/player` が同じコードで訳せばそのまま効く。
- T0434 (やったこと):
  - `LoadError::info()` をすべての種類に広げた (`Option` を外した)。`load_json` / `load_bundle` の失敗は、すべて `name` が `"LoadError"` で
    `code` と補足の値を持つ `Error`。コードは `invalid_file`・`unsupported_version` (`found`・`supported`)・`unsupported_features` (`features`)・
    `invalid_model` (`errors`)・`compile`・`read_limit_exceeded`・`image_limit_exceeded` (T0152 のまま)・`invalid_texture` (`image`・`texture`)・
    `texture_too_large` (`image`・`texture`・`width`・`height`・`max`)・`texture`・`device_lost`。`invalid_file`・`compile`・`read_limit_exceeded` は
    C ABI の `ugoe_last_error_kind` (着せ替えの `invalid_file`・`compile`) と同じ値。`load()` の失敗 (デバイスを失った・デバイスの上限・登録・
    コンパイル) も `LoadError` の種類 (`Render`・`Register`・`DeviceLost`) にして同じ口から返す。
  - 補足の値は T0157 の `FieldValue` と同じ形: 番号と大きさは数、ほかは文字列 (`field_json(FieldValue::…)` で作る)。展開の上限の `max` (u64 の
    バイト数) だけは wasm32 の `usize` に収まらないことがあるので、`Index` を通さずに数にした。
  - `.ugoemod` の読み込みの失敗 (`DressUpError`) は今までどおり: 上限を超えたものだけ上の 2 つのコードで、ほかは `invalid_file` (C ABI と同じ)。
    `LoadError::exceeds_limit()` で分ける。
  - 表は `crates/ugoe-wasm/README.md#読み込みの失敗-t0434`。ネイティブのテスト `load::tests::every_load_error_has_a_code` で対応を確かめる。
  - `@ugoe/runtime` に `UgoeLoadError`・`UgoeLoadErrorCode`・`UgoeLoadErrorFields`・`isLoadError` を足した (`bindings/web/src/limits.ts`)。
    `stub/ugoe_wasm.d.ts` は型の上で変わらない (例外の型は d.ts に出ない) ので直していない。
  - **`@ugoe/player` から読み込みのエラーをどう渡すか:** 訳すのは `@ugoe/player` (`packages/player/src/loadError.ts` の `describeLoadError`、
    カタログ `player` の `loadError.*`) にし、`Player.error` に今の言語の文を入れる (着せ替えの `describeDressUpError` と同じ形)。ビューアと
    スタジオはどちらも `Player.error` を出すだけなので、アプリにはコードを渡さず、訳も持たない。`load_module` が上限を超えた `DressUpError`
    (`read_limit_exceeded`・`image_limit_exceeded`) も同じ文にした (`describeLimitError`)。知らないコードは英語の文のまま。
  - crates: の外 (小さな変更): ビューアのカタログの `errors.format.chunk_limit_exceeded`・`format.total_limit_exceeded` (T0729 で wasm の
    コードを待って置いたもの) は、上の方針で使われなくなるので消し、`apps/viewer/README.md` と `errors.test.ts` (数の書式の例を `file.too_large`
    に) を直した。ビューアの wasm のエラーは `format.*` (エディタのコード) ではなく wasm の平らなコードで、`@ugoe/player` が訳す。スタジオは変更なし。
  - `docs/portability.md` に wasm の読み込みのエラーのコードを 1 項目足した。
- やらなかったこと:
  - 実際のブラウザでの確認 (wasm の `coded_error` の形は着せ替えと同じ口を使うので、ネイティブのテストと wasm32 の clippy (`just wasm-check`) まで)。
  - 着せ替えの付け外しの `with_wardrobe!` の「the GPU device was lost」(コード無しの `JsError`) と、`invalid limits` の例外は変えていない。
- 起票の候補:
  - C ABI のモデルの読み込み (`ugoe_puppet_create_from_*`) の失敗にも `ugoe_last_error_kind` (`invalid_file`・`unsupported_version`・`invalid_model`・
    `compile` など) を付け、wasm の `LoadError` と揃える (いまは上限を超えたときだけ種類を持つ)。種類の決定を SDK (`ugoe` クレート) に移すとよい。
  - wasm の着せ替えの付け外しで GPU を失ったときの例外に `DressUpError` の `device_lost` を付ける。
  - エディタのプレビュー (`apps/editor`) の `load_json` の失敗も `LoadError` の `code` から訳す (いまは英語の文を出す)。

