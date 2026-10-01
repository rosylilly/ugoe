---
id: T0662
title: MCP にモジュールの自動リギングのツール・見本の髪を肩の衝突体に当てる
status: todo
milestone: M4
crates: [ugoe-ai, ugoe-mcp, ugoe-cli, assets, docs]
depends_on: [T0660]
assignee:
---

# T0662: モジュールの自動リギング (MCP) と見本の肩

## 目的

T0660 の後続の候補のうち、ツール層と見本に閉じるものをまとめる。

## 受け入れ条件

- [ ] ugoe-mcp にモジュールの自動リギングのツールを足し、`set_layer_labels` で受けた `variant_sets` をポーズに使う (今は分類に残るだけ)
- [ ] 見本のモジュール `LongHair` の揺れを、本体がスロット Hair で公開する肩の衝突体 `Shoulders` に当てる (`.ugoemod` と指紋が変わる。ビューアの覚えた組への影響をメモに書く)
- [ ] ツールの説明・ugoe-mcp の README・`docs/manual/mcp.md` を揃える
- [ ] `just ci`・`just conformance --only c,wasm` が通る

## 参考

- `docs/tasks/T0660-editor-pose-and-sample-leftovers.md` のメモ

## メモ
