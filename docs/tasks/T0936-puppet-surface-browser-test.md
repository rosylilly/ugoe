---
id: T0936
title: PuppetSurface を本物の wasm とブラウザで確かめる自動テスト (判断が要る: 置き場所)
status: in-progress
milestone: M9
crates: [packages, tests, docs]
depends_on: [T0935]
assignee: claude-worker-T0936
---

# T0936: PuppetSurface を本物の wasm とブラウザで確かめる自動テスト (判断が要る: 置き場所)

## 目的

T0932 の `PuppetSurface` は、本物の wasm とブラウザでは手で書いたページで確かめただけ (T0932 の候補)。自動のテストにする。

**判断が要る:** 置き場所。packages/ は GPL-3.0-only で、`tests/conformance` は MIT。MIT の場所に GPL のコードを入れない (AGENTS.md) ので、
packages/ の中にブラウザのテストを置くか、別の GPL のテストの場所を作るか。

## 受け入れ条件

- [ ] 置き場所を決めた (ライセンスの理由を書いた。利用者の判断を待つなら blocked にしてメモに案を書く)
- [ ] 本物の wasm とヘッドレスのブラウザで、`PuppetSurface` の作成・描画・作り直し (T0935 の方針) を確かめるテストがあり、`just` のレシピから流せる
- [ ] CI で流すか (どのジョブか) を決め、docs/testing.md に書いた
- [ ] `just ci` が通る

## 参考

- T0932 のメモ (起票の候補)、T0440 (`<ugoe-avatar>` のブラウザのテスト)

## メモ

- T0026 で起票した (元の候補: T0932 の自動テスト)。作り直しの方針を決める T0935 の後に着手する。
