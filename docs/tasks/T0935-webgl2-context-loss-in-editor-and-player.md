---
id: T0935
title: エディタと Player でも WebGL2 の喪失で復元を待つか、続けて失敗したときの諦め方を決める (判断が要る)
status: todo
milestone: M9
crates: [packages/player, apps/editor, docs]
depends_on: []
assignee:
---

# T0935: エディタと Player でも WebGL2 の喪失で復元を待つか、続けて失敗したときの諦め方を決める (判断が要る)

## 目的

T0932 で WebPuppet の作り直しを共有したが、WebGL2 のコンテキストを失ったときの扱いが `<ugoe-avatar>` (復元を待つ) と、エディタ・Player で違う。
作り直しが続けて失敗したときの諦め方 (回数・知らせ方) も決まっていない (T0932 の候補)。

**判断が要る:** エディタと Player を `<ugoe-avatar>` に揃えるか。諦めるまでの回数と、諦めたときの画面の知らせ。

## 受け入れ条件

- [ ] 方針を決め、docs (docs/ui.md か packages/player の README) に書いた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [ ] 方針どおりに直し、喪失・復元・続けての失敗をテストする (偽の WebGL2 のコンテキストで)
- [ ] `just ci` が通る

## 参考

- T0932 のメモ (起票の候補)、T0697

## メモ

- T0026 で起票した (元の候補: T0932 の WebGL2 の喪失)。
