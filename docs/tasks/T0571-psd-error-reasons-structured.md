---
id: T0571
title: "ugoe-psd の Invalid・Unsupported の理由を構造にして psd_read.* の detail を訳し、EditorError::PsdRead に原因を持たせる"
status: done
milestone: M9
crates: [ugoe-psd, ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0571
---

# T0571: ugoe-psd の Invalid・Unsupported の理由を構造にして psd_read.* の detail を訳し、EditorError::PsdRead に原因を持たせる

## 目的

T0558 で、PSD の読み込みの失敗は `psd_read.*` のコードと原因を持つようになったが、次は英語の文のまま出る (T0558 の候補 2 つ):

- `ugoe-psd` の `PsdError::Invalid(String)`・`Unsupported(String)` の理由 (30 か所ほどで文を組む)。`psd_read.invalid`・`psd_read.unsupported` の `detail` が英語。
- エディタのバックエンドの `psd.read_file_failed` (AI の提案の PSD を読む `read_tree`。`EditorError::PsdRead { path, detail }`) は原因を持たず、`detail` が英語。

## 受け入れ条件

- [x] `PsdError` の `Invalid`・`Unsupported` の理由を種類と値の構造 (例: `UnsupportedDepth { depth }`・`UnsupportedColorMode { mode }`・`GroupsTooDeep { max }`) にした
      (`#[non_exhaustive]` を保つ。ugoe-psd は crates.io に出していないので互換の別名は要らない)。テストで種類と値を確かめる
- [x] editor-core の `Coded for PsdError` が理由の種類ごとのコードと引数を出し、エディタの日英のカタログで訳す (`sourced_codes` のテストと見本を直した)
- [x] `EditorError::PsdRead` が原因 (`PsdError` の `Message`) を持ち、`psd.read_file_failed` の `detail` も `psd_read.*` で訳される。テストがある
- [x] crates/ugoe-psd/README.md・docs/editor.md のエラーの節を直した
- [x] `just ci` が通る

## 参考

- T0558 のメモ (判断・やらなかったこと・起票の候補)
- crates/ugoe-psd/src (`PsdError`)、crates/ugoe-editor-core (`Coded`・`sourced`)、apps/editor/src-tauri (`EditorError::PsdRead`)

## メモ

- T0038 で起票した (元の候補: T0558 (`Invalid`・`Unsupported` の理由と `EditorError::PsdRead`))。同じエラーの組み立てを触る T0572 はこのタスクの後に着手する。

### 判断 (claude-worker-T0571)

- **`Invalid(String)`・`Unsupported(String)` を、`PsdError` の平らな変種 (19 個) に置き換えた。** `Invalid(理由の enum)` のように
  2 段にする案もあったが、`code()` を変種ごとの値にすると `sourced("psd_read", code)` と `sourced_codes` のテスト (ソースの `code()` の
  文字列を読む) がそのまま使え、ほかのクレート (`MeshError`・`ValidationError`) とも同じ形になるので平らにした。壊れた入力か
  対応しない機能かの区別は `is_invalid()`・`is_unsupported()` で引ける。`Display` の文は前と同じ (`invalid psd: …`・
  `unsupported psd feature: …`)。ZIP の展開の失敗の状態 (`TINFLStatus` の名前) は文から落とした (`BadZipData`)。
- `Truncated { section }` の区画の名前 (`"layer info"` など 40 ほど) と、`UnsupportedColorMode` の `name` (`color_mode_name`。`cmyk` など)
  は PSD の形式の用語として英語のまま引数に渡し、訳さない。区画の名前まで種類にすると読み取りの関数の引数がすべて変わる割に、
  利用者に意味が増えないため。

### やったこと (T0571)

- **ugoe-psd:** `PsdError` の変種 (`NotPsd`・`UnknownVersion`・`Truncated`・`InvalidChannelCount`・`InvalidImageSize`・`LayerBoundsOutOfRange`・
  `TooManyLayerChannels`・`BadBlendModeSignature`・`BadZipData`・`ZipLengthMismatch`・`RleRowTooShort`・`RleRowWrongLength`・
  `TooFewCompositeChannels`・`UnbalancedGroups`・`UnsupportedPsb`・`UnsupportedDepth`・`UnsupportedColorMode`・
  `UnsupportedCompositeCompression`・`GroupsTooDeep`・`LimitExceeded`)、`code()` (変種の名前)、`is_invalid()`・`is_unsupported()`、
  `color_mode_name(mode)`。テスト: 既存のテストを変種と値で確かめるように直し、`error_reasons_carry_their_kind_and_values`
  (種類・値・コード・文) を足した。
- **editor-core:** `Coded for PsdError` が理由ごとの引数を出す (`version`・`section`・`channels`・`width`・`height`・`mode`・`name` など)。
  見本 (`psd_errors`) をすべての変種にし、`source_error_codes_follow_the_crate_codes` の数を直した。`import_tests` で原因のコード
  (`psd_read.not_psd`) と値 (`psd_read.unsupported_color_mode` の `mode`・`name`) を確かめる。
- **apps/editor:** `EditorError::PsdRead { path, error: PsdError }` にし、`cause()` で `psd_read.*` を載せる (`detail` は英語の文のまま)。
  テスト: `error.rs` の `psd_read_carries_the_psd_error_as_the_cause`、`proposals_tests.rs` の `read_tree_carries_the_psd_error_as_the_cause`。
  `en.json`・`ja.json` の `psd_read.invalid`・`unsupported` を理由ごとの 19 の文に置き換え、`messageCodes.ts` を作り直した (`just gen`)。
- **docs:** ugoe-psd の README (エラーの節を変種の表に、PackBits・ZIP の節の変種の名前)、`docs/editor.md` (読み込みの上限の節・
  まだ英語のもの・コードの表)。`docs/editor.md` に残っていた古い記述 (上限の超過を `Unsupported` の文で見分ける。T0558 で変わった) を消した。
- `just ci` が通った (main に rebase した後)。

### やらなかったこと

- 実機のエディタの画面での表示は確かめていない (Rust のテスト・翻訳のカタログの Vitest まで)。
- `automesh.invalid_options` の `reason` (T0558 の候補の 3 つ目) は範囲の外 (T0572 など別のタスク)。

### 起票の候補

- なし (T0558 の候補の残り `MeshError::InvalidOptions` は既に起票済みと思われる。無ければ起票する)。
