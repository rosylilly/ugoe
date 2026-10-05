---
id: T0558
title: PSD の読み込みと自動メッシュのエラーに構造とコードを付け、エディタで訳す (判断が要る)
status: in-progress
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

- [ ] `PsdError` に上限の超過の変種があり、editor-core が文でなく変種で見分ける。テストがある
- [ ] `PsdError`・`MeshError` に `code()` があり (T0163 の形)、editor-core のメッセージの `cause`・`detail` がコードと引数で訳される。`en.json`・`ja.json` に訳がある
- [ ] ugoe-psd・ugoe-mesher の README と docs/editor.md#エラーと警告のコード を更新した
- [ ] `just ci` が通る

## 参考

- T0554・T0163 のメモ (起票の候補)、T0533

## メモ

- T0026 で起票した (元の候補: T0554 の `PsdError` の上限の変種、T0163 の `MeshError`・`PsdError` のコード)。
