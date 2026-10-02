---
id: T0422
title: 独立の実装: 描画の例の view のケースで、参照の描画に view を渡して比べる
status: in-progress
milestone: M7
crates: [tools/spec-check]
depends_on: [T0414]
assignee: claude-worker-T0422
---

# T0422: 黒箱の比較で view を渡す

## 目的

T0416 から分けた。tools/spec-check は公開仕様 (spec/) だけを読んで書いた独立の実装なので、crates/ と docs/spec/ を読まない作業者が担当する
([ADR 0021](../adr/0021-public-specification.md))。

## 受け入れ条件

- [ ] `tools/spec-check/src/render-reference.ts` で、描画の例のうち view を持つケースに、参照の描画 (`ugoe render`) の view の指定を渡して比べる
      (`ugoe render --help` の出力と spec/ だけを手がかりにする)
- [ ] view を持つ例がすべて比べられ、期待値と一致する
- [ ] `just spec`・`just ci` が通る

## 参考

- spec/ の描画の節、`tools/spec-check/README.md`

## メモ
