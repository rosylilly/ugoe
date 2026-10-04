---
id: T0554
title: PSD の取り込みで、上限の超過と大きすぎるレイヤーを分かりやすく扱う
status: done
milestone: M9
crates: [ugoe-editor-core, ugoe-mcp, ugoe-mesher, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0554
---

# T0554: PSD の取り込みで、上限の超過と大きすぎるレイヤーを分かりやすく扱う

## 目的

- **上限の超過 (T0531):** PSD のデコードの上限を超えると、エディタと MCP サーバは `Unsupported` のメッセージのまま出す。
  何の上限を超えたか (大きさ・合計) が分かるコードと文言にする。上限を変える設定を持たせるかも決める。
- **大きすぎるレイヤー (T0533):** `ugoe-editor-core` の PSD の取り込み (`import.rs`) は、自動メッシュの `MeshError::TooLarge` を受けると
  取り込み全体を失敗させる。`TooComplex` と同じく警告を出してそのレイヤーだけ飛ばす。
- **`MeshError` の `#[non_exhaustive]` (T0533):** 変種を足すたびに公開 API の変更になるのを避けるため付けるか決める
  (**判断が要る:** `ugoe-mesher` の公開 API の変更)。

## 受け入れ条件

- [x] 上限の超過が、エディタ (翻訳つき) と MCP で、何の上限かが分かるコード (ADR 0029) で返る。テストがある
- [x] `TooLarge` のレイヤーは警告 (コードと翻訳のキーつき) を出して飛ばし、残りのレイヤーは取り込まれるテスト
- [x] `MeshError` に `#[non_exhaustive]` を付けるか決めた (付けたら呼び出し側の `match` を直した)
- [x] 上限を変える設定を持たせるか決め、`docs/editor.md` (と ai-integration.md の MCP の節) に書いた
- [x] `just ci` が通る

## 参考

- T0531・T0533 のメモ (起票の候補)
- crates/ugoe-editor-core/src/import.rs・automesh.rs、crates/ugoe-mesher/src/lib.rs (`MeshError`)

## メモ

- T0019 で起票した (元の候補: T0531 のエラーの見せ方と上限の設定、T0533 の `TooLarge` と `#[non_exhaustive]`)。

### やったこと (T0554)

- **上限の超過:** editor-core に `import::read_layer_tree(bytes, limits)` と `PsdImportError::LimitExceeded { limit: PsdLimit, max }` を足し、
  コード `psd.channel_limit_exceeded` / `psd.total_limit_exceeded` (引数 `max` バイト) で返す。`import_psd` はこれで読む。
  `ugoe-psd` は上限の超過を `PsdError::Unsupported` の文 ("import limit"、合計は "in total" で終わる) で返すので、
  `PsdImportError::from_psd` が文で見分ける (`ugoe-psd` は範囲の外で、T0531 が variant を足さないと決めたので変えていない)。
  文が変わると editor-core・エディタ・MCP の上限のテスト (`layers.psd`・bustup を小さな上限で読む) が失敗する。
  - エディタ: 取り込み (`import_psd`・新しいプロジェクト) は `?` でそのコードになる。AI の提案の PSD を読む `read_tree` も、
    上限の超過はそのコードで、ほかは今までどおり `psd.read_file_failed` (パス付き)。日英の翻訳と `messageCodes.ts` (`just gen`) を足した。
  - MCP: `PsdSource::load` が `read_layer_tree` で読み、失敗の文の末尾に `(code: psd.channel_limit_exceeded)` の形でコードを付ける
    (MCP のツールのエラーは文だけなので、ADR 0029 の「MCP は英語の文」を保ったまま見分けられるようにした)。
- **上限を変える設定:** 持たせないと決めた (`docs/editor.md#読み込みの上限`・`docs/ai-integration.md` の PSD 取り込みと
  エージェントに任せる自動リギング)。既定値は 30000² のキャンバスの 1 チャンネルが収まり、上げる設定はメモリを使い果たす入力を
  通す手段になるため。ライブラリとしては `read_layer_tree` に別の `ImportLimits` を渡せる。
- **`TooLarge`:** `Builder` が `TooComplex` と同じく警告 `psd.layer_too_large` (`width`・`height` = 格子のセル数、`max` = `MAX_GRID_CELLS`) を
  出してそのレイヤーだけ飛ばす。本物の `TooLarge` は 1 GB 超の画像が要るので、自動メッシュの関数を差し替える
  `layer_tree_command_with` (crate 内) を足し、特定の幅で `TooLarge` を返すスタブで「警告が出て前後のレイヤーは取り込まれる」ことを確かめた。
  カタログの見本 (`message_catalog.rs` の inline 警告) と `docs/editor.md` のコード表・取り込みの規則の表も直した。
- **`MeshError` の `#[non_exhaustive]`:** `ugoe-mesher` は `scripts/crates.mjs` の `PUBLISHED` に無い (crates.io に出さない) ので付けた。
  リポジトリの中の `match` (editor-core の `import.rs`・`automesh.rs`) はもともと `_`/`Err(source)` の腕があり、直す所はなかった。
  README の「保証」に 1 行。
- `just ci` が通った (main に rebase した後)。

### やらなかったこと

- CLI (`ugoe import-psd`) は `import_psd` を通るので上限の超過は新しい文になるが、CLI 自体は変えていない (範囲の外。anyhow の文)。
- `ugoe-psd` に構造のある変種 (上限の種類と値) は足していない。

### 起票の候補

- `ugoe-psd` の `PsdError` に上限の超過の変種 (`LimitExceeded { limit, size, max }` など。`ugoe-format` の `ReadLimitExceeded` と同じ形) を足し、
  editor-core の文での見分け (`PsdImportError::from_psd`) をやめる。チャンネルの実際の大きさも引数に出せる。
- MCP のツールのエラーを文だけでなく構造 (`structuredContent` の `code`・`args`) でも返すか (いまは文の末尾の `(code: ...)`)。

- **判断 (2026-10-05、利用者の判断):** エディタに PSD の読み込みの上限を変える設定は出さない (今のまま)。SDK (ugoe-psd の ImportLimits) では変えられる。
