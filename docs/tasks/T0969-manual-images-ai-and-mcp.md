---
id: T0969
title: "マニュアル: AI と MCP の画像 (ai-proposal・ai-revision・ai-consent・ai-settings など・mcp-*) を偽の AI のプロバイダで撮り直す"
status: in-progress
milestone: M10
crates: [tests, docs]
depends_on: [T0050, T0968]
assignee: claude-worker-T0969
---

# T0969: マニュアル: AI と MCP の画像 (ai-proposal・ai-revision・ai-consent・ai-settings など・mcp-*) を偽の AI のプロバイダで撮り直す

## 目的

AI のタブの画像は T0676・T1643 より前のまま (提案のカードに「Adopting changes:」の行も今のボタンの並びも無い。T1643)。
ai-settings.png は T0919 で撮り直したが、鍵の保管庫が無くて API キーの欄が写っていない。MCP の画像も撮っていない (T0919)。
`just e2e --manual` で、偽の AI のプロバイダと鍵の保管庫 (T0050) を使って撮り直す。

## 受け入れ条件

- [ ] `just e2e --manual` の筋書きが、ネットワークに出ない偽の AI のプロバイダ (決まった提案・改訂・法線マップを返す) でエディタを動かす。
      偽物をアプリに入れる口が要るなら、e2e のビルドだけで有効になる形にし、その理由をメモに書いた
- [ ] `ai-proposal`・`ai-consent`・`ai-instructions`・`ai-revision`・`ai-normal-maps`・`ai-settings` (API キーの欄が写る) を撮った
- [ ] `mcp-*` の画像を撮った (MCP サーバと承認の流れ。撮れないものは理由と撮り方をメモと docs/manual-check.md に書いた)
- [ ] docs/manual/ai.md・mcp.md の文と画像の説明が合っている
- [ ] tests/e2e/README.md#マニュアルの画像 の一覧を直した
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0919 のメモ (やらなかったこと・起票の候補)、T1643 のメモ (ai-revision.png)、T0050 (鍵の保管庫)
- tests/e2e/README.md#マニュアルの画像、docs/manual/ai.md、docs/manual/mcp.md

## メモ

- T0046 で起票した (元の候補: T0919 (AI と MCP の画像)・T1643 (ai-revision.png の撮り直し))。
- 同じ `editor.manual.mjs` を触るので T0968 の後にした。
