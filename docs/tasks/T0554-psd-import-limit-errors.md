---
id: T0554
title: PSD の取り込みで、上限の超過と大きすぎるレイヤーを分かりやすく扱う
status: todo
milestone: M9
crates: [ugoe-editor-core, ugoe-mcp, ugoe-mesher, apps/editor, docs]
depends_on: []
assignee:
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

- [ ] 上限の超過が、エディタ (翻訳つき) と MCP で、何の上限かが分かるコード (ADR 0029) で返る。テストがある
- [ ] `TooLarge` のレイヤーは警告 (コードと翻訳のキーつき) を出して飛ばし、残りのレイヤーは取り込まれるテスト
- [ ] `MeshError` に `#[non_exhaustive]` を付けるか決めた (付けたら呼び出し側の `match` を直した)
- [ ] 上限を変える設定を持たせるか決め、`docs/editor.md` (と ai-integration.md の MCP の節) に書いた
- [ ] `just ci` が通る

## 参考

- T0531・T0533 のメモ (起票の候補)
- crates/ugoe-editor-core/src/import.rs・automesh.rs、crates/ugoe-mesher/src/lib.rs (`MeshError`)

## メモ

- T0019 で起票した (元の候補: T0531 のエラーの見せ方と上限の設定、T0533 の `TooLarge` と `#[non_exhaustive]`)。
