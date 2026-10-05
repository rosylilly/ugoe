---
id: T0938
title: WebGL2 の復元が届かないときの扱いと、諦めた後に作り直すボタン (エディタ・Player。判断が要る)
status: todo
milestone: M9
crates: [packages/player, apps/editor, bindings/web, docs]
depends_on: [T0936]
assignee:
---

# T0938: WebGL2 の復元が届かないときの扱いと、諦めた後に作り直すボタン (エディタ・Player。判断が要る)

## 目的

T0935 で、エディタと Player も WebGL2 のコンテキストの喪失で復元を待ち、続けて失敗したら諦めるようにした。次が残っている (T0935 の候補)。

1. **判断が要る:** 復元 (`webglcontextrestored`) が届かないまま長く待つときの扱い (時間切れで作り直しを試す・「GPU の復元を待っています」と出す)。
   `<ugoe-avatar>` (bindings/web) と同じ扱いにするので一緒に決める。
2. 諦めた後に、起動し直さずに作り直せるボタン (Player・エディタのビューポート。`<ugoe-avatar>` の `restart()` に当たるもの)。

`PuppetSurface` を触る T0936 の後に着手する。

## 受け入れ条件

- [ ] 復元を待つ間の表示と時間切れの扱いの案と推奨をメモに書き、利用者の判断を得た。決めた扱いをエディタ・Player・`<ugoe-avatar>` で揃え、docs に書いた
- [ ] 諦めた後の「作り直す」ボタンがエディタのビューポートと Player にあり、押すと読み込み直す (モデルと着せ替えを保つ)。部品のテストがある
- [ ] 文言は `@ugoe/i18n` のカタログ (日英) にある
- [ ] `just ci` が通る

## 参考

- T0935 のメモ (起票の候補)
- docs/ui.md、bindings/web/README.md

## メモ

- T0032 で起票した (元の候補: T0935 (2 つ))。
