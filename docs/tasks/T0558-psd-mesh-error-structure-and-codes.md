---
id: T0558
title: PSD の読み込みと自動メッシュのエラーに構造とコードを付け、エディタで訳す (判断が要る)
status: done
milestone: M9
crates: [ugoe-psd, ugoe-mesher, ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0558
---

# T0558: PSD の読み込みと自動メッシュのエラーに構造とコードを付け、エディタで訳す (判断が要る)

## 目的

- `ugoe-psd` の `PsdError` には上限の超過の変種が無く、editor-core は文で見分けている (`crates/ugoe-editor-core/src/import.rs` の `PsdImportError::from_psd`)。
  `ugoe-format` の `ReadLimitExceeded` と同じ形 (`LimitExceeded { limit, size, max }` など) を足せば、チャンネルの実際の大きさも引数に出せる (T0554 の候補)。
- `ugoe-mesher` の `MeshError` と `PsdError` にコードが無く、`edit.automesh_failed`・`psd.automesh_failed`・`psd.*` の `detail` が英語の文のまま (T0163 の候補)。

**判断が要る:** `PsdError` に変種を足すのは公開 API の変更 (`#[non_exhaustive]` が無ければ破壊的)。T0554 で決めた `MeshError` の扱いに揃えるかを確かめる。

## 受け入れ条件

- [x] `PsdError` に上限の超過の変種があり、editor-core が文でなく変種で見分ける。テストがある
- [x] `PsdError`・`MeshError` に `code()` があり (T0163 の形)、editor-core のメッセージの `cause`・`detail` がコードと引数で訳される。`en.json`・`ja.json` に訳がある
- [x] ugoe-psd・ugoe-mesher の README と docs/editor.md#エラーと警告のコード を更新した
- [x] `just ci` が通る

## 参考

- T0554・T0163 のメモ (起票の候補)、T0533

## メモ

- T0026 で起票した (元の候補: T0554 の `PsdError` の上限の変種、T0163 の `MeshError`・`PsdError` のコード)。

### 判断 (claude-worker-T0558)

- **`PsdError` に変種を足し、`#[non_exhaustive]` を付けた (T0554 の `MeshError` と同じ扱い)。** 根拠:
  `ugoe-psd`・`ugoe-mesher` は `scripts/crates.mjs` の `PUBLISHED` に無く (`publish.workspace = true` だが公開の検査の対象外。
  版は `[workspace.package]` の 0.0.0)、crates.io に出ていないので、変種を足して壊れる外の利用者はいない。リポジトリの中の
  `PsdError` の `match` (editor-core・テスト) は直した。今後も足せるよう `#[non_exhaustive]` にした (`ImportLimit` も)。
  公開済みの API を壊す選択ではないので blocked にしなかった。

### やったこと (T0558)

- **ugoe-psd:** `PsdError::LimitExceeded { limit: ImportLimit, size, max }` (`ugoe-format` の `ReadLimitExceeded` と同じ形。`ImportLimit` は
  `ChannelDecompressed`・`TotalDecompressed`、`as_str`)。`Budget::take` が `Unsupported` の文の代わりにこれを返す。`size` は
  チャンネル 1 枚ならその平面、合計ならそれまでの合計に足したもの (超えたところでやめる)。`PsdError::code()` (`invalid`・
  `unsupported`・`limit_exceeded`)、`Clone`・`PartialEq`・`Eq` を足した。テスト: 既存の上限のテストを変種で確かめ、
  `limit_exceeded_tells_which_limit_and_the_size` で種類・大きさ・値・コードを確かめる。
- **ugoe-mesher:** `MeshError::code()` (`empty`・`bad_image`・`invalid_options`・`too_complex`・`too_large`) と `Clone`・`PartialEq`・`Eq`
  (`EditError` が `PartialEq` なので、`MeshError` をそのまま持つため)。テスト `error_codes_are_the_variant_names`。
- **editor-core:**
  - `PsdImportError::from_psd` (文で見分けていた) を消し、`From<PsdError>` が変種で `LimitExceeded { limit, size, max }` に分ける
    (未知の上限は `Psd` のまま = `psd.read_failed`)。`psd.channel_limit_exceeded`・`psd.total_limit_exceeded` に引数 `size` を足した。
  - `EditError::Mesher` のフィールドを `message: String` から `error: MeshError` に変えた (英語の文は同じ)。
  - `Coded for PsdError` (`psd_read.*`) と `Coded for MeshError` (`automesh.*`) を `sourced` で組み立て、`psd.read_failed`・
    `psd.automesh_failed`・`edit.automesh_failed` の `cause` に載せる。画面 (`errors.ts`) は `detail` を持つコードなので原因の訳を
    `detail` に差し込む (画面のコードは変えていない)。`sourced_codes` のテストに 2 つのクレートを足し、見本 (`psd_errors`・`mesh_errors`) を足した。
  - テスト: `limit_exceeded_tells_which_limit` (`size` と原因 `psd_read.invalid`)、`regenerate_mesh_*` (原因 `automesh.invalid_options`)、
    `psd_import_mesh_error_carries_the_mesh_error_as_the_cause`。
- **翻訳:** `en.json`・`ja.json` に `psd_read.*`・`automesh.*` を足し、上限の 2 つの文に `{{size}}` を入れた。`messageCodes.ts` (`just gen`)。
- **docs:** `docs/editor.md` (読み込みの上限・元のエラーのコード・まだ英語のもの・コードの表)、ugoe-psd の README (エラーの節・展開の上限)、
  ugoe-mesher の README (`code()`)。
- **範囲外の小さな変更:** `crates/ugoe-mcp/src/autorig.rs` のテストが上限の文 (editor-core の `PsdImportError` の文) を完全一致で見ていたので、
  `size` の入った新しい文に直した (1 行)。
- `just ci` が通った (main に rebase した後)。

### やらなかったこと

- `psd_read.invalid`・`psd_read.unsupported` の `detail` (壊れた理由・対応しない機能の名前) は英語の文のまま。`ugoe-psd` の
  `Invalid(String)`・`Unsupported(String)` は 30 か所ほどで文を組むので、構造にするのは別の作業にした。
- `automesh.invalid_options` の `reason` (`MeshOptions::validate` の文) も英語のまま。
- エディタのバックエンドの `psd.read_file_failed` (AI の提案の PSD を読む `read_tree`。`EditorError::PsdRead { path, detail }`) は
  原因を持たないので英語の `detail` のまま (T0567 が `apps/editor` を並行して触っているので、`EditorError` の形は変えなかった)。
- 実機のエディタの画面での表示は確かめていない (Rust のテスト・翻訳のカタログの Vitest まで)。

### 起票の候補

- `ugoe-psd` の `Invalid`・`Unsupported` の理由を種類と値の構造にし (`UnsupportedDepth { depth }`・`UnsupportedColorMode { mode }`・
  `GroupsTooDeep { max }` など)、`psd_read.*` の `detail` を訳す。
- `EditorError::PsdRead` に原因 (`PsdError` の `Message`) を持たせ、`psd.read_file_failed` の `detail` も `psd_read.*` で訳す。
- `MeshError::InvalidOptions` の理由をフィールドの名前と値にし、`automesh.invalid_options`・`psd.invalid_mesh_options` の `reason` を訳す。

