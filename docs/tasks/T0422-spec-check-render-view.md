---
id: T0422
title: 独立の実装: 描画の例の view のケースで、参照の描画に view を渡して比べる
status: done
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

- [x] `tools/spec-check/src/render-reference.ts` で、描画の例のうち view を持つケースに、参照の描画 (`ugoe render`) の view の指定を渡して比べる
      (`ugoe render --help` の出力と spec/ だけを手がかりにする)
- [x] view を持つ例がすべて比べられ、期待値と一致する
- [x] `just spec`・`just ci` が通る

## 参考

- spec/ の描画の節、`tools/spec-check/README.md`

## メモ

- `ugoe render --help` に `--view <A,B,C,D,E,F>` があり、説明 (キャンバスの `(x, y)` を `(A*x + C*y + E, B*x + D*y + F)` に写す) が
  spec/examples/README.md#rendering-examples の `view` と同じ式・同じ順だったので、例の 6 数をそのまま `--view=…` で渡すようにした
  (負の数で始まる値をオプションと取り違えられないよう `=` でつなぐ)。`view` の無いケースは従来どおり `--fit stretch`。仕様と CLI の食い違いは無かった。
- 結果: `view` のある 6 ケース (`render-culling` の 2〜4、`render-lighting` の 6〜8) を含むすべての描画の例で、比べられる画素は 0 件の食い違い。
  `render-lighting` (T0127 の時点では `build` の問題で描けなかった) も描けて一致した。
- 渡した `view` が効いていることを、使い捨ての改変で確かめた: `e` を 3 画素ずらすと `render-lighting` の 3 ケースで、上下を反転させると
  `render-culling` の 3 ケースで食い違いが出る (改変はコミットしていない)。
- README (`tools/spec-check/README.md`) の「参照実装の描画との比較」を更新した。
- 確かめていないこと: 比較は手で実行するもの (CI に入っていない) で、WSL の dzn (Vulkan) の上でだけ走らせた。
