---
id: T0546
title: AI: 名前の規則と AI の分類の食い違いを利用者に示す
status: in-progress
milestone: M9
crates: [ugoe-ai, ugoe-mcp, apps/editor, docs]
depends_on: [T0528]
assignee: claude-worker-T0546
---

# T0546: AI: 名前の規則と AI の分類の食い違いを利用者に示す

## 目的

T0528 のレビューで見つけた。`classify_layers_guided` は、モデルが答えたレイヤーのラベルと左右を答えでそのまま置き換え、名前の規則
(`classify_by_name`) の結果は答えのないレイヤーの補いにだけ使う。名前の規則が確からしさ 0.9 で当たっていても (`eyewhite-r` など)、
AI が別のラベルや逆の左右を答えれば黙って上書きされ、確認に回るか (`LayerLabel::needs_review`) は AI の確からしさだけで決まる。
左右の取り違えは目の開閉のパラメータを入れ替えるので、食い違いは利用者に見せたい。

## 受け入れ条件

- [ ] 名前の規則が当たった (確からしさが高い) レイヤーで、AI のラベルか左右が違うものを結果から分かるようにする (確からしさを下げる・
      食い違いの印を `LayerLabel` に足す、などから選び、公開 API の変更なら `docs/ai-integration.md` を同じ PR で直す)
- [ ] エディタの分類の確認と MCP の `get_psd_layers` で、食い違いを示す (文言は i18n)
- [ ] 食い違いのあるとき・ないときのテスト (`FakeProvider`)
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-ai/src/classify.rs` (`classify_layers_guided`・`merge_answer`)
- `docs/ai-integration.md#レイヤーの意味分類`
- `docs/tasks/T0528-review-ai-autorig.md` のメモ

## メモ
