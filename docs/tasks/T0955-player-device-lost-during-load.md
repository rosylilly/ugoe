---
id: T0955
title: "Player: 読み込みの途中に GPU を失ったら、エラーの文を出さず、開こうとしたファイルを作り直した後に開く"
status: todo
milestone: M9
crates: [packages/player, apps/viewer, apps/editor, docs]
depends_on: [T0486]
assignee:
---

# T0955: Player: 読み込みの途中に GPU を失ったら、エラーの文を出さず、開こうとしたファイルを作り直した後に開く

## 目的

T0486 で利用者が `<ugoe-avatar>` の扱いを案 A に決めた (2026-10-06): 読み込みで `device_lost` を受けたら `error` を送らず、モデルを取っておいて
作り直した後に読む。`error` は作り直しを諦めたときだけ。

`@ugoe/player` の Player は今、読み込みで `device_lost` を受けるとエラーの文 (「GPU が止まりました」) を出し、開こうとしたファイルを捨て、
作り直した後は前のモデルを読み直す (T0486 の調べ)。前のモデルが無ければエラーの文が残る。`<ugoe-avatar>` と同じ扱いに揃える。

## 受け入れ条件

- [ ] Player は読み込み (開く・着せ替えを含むなら同じく) で `device_lost` を受けてもエラーの文を出さず、開こうとしたファイルを作り直した後に開く
- [ ] 作り直しを諦めたときだけエラーの文を出す (T0946 の理由の文)
- [ ] `player.test.ts` に偽物の順序のテスト、browser-test に読み込み中の喪失のシナリオがある
- [ ] エディタのビューポート (`ViewportController`) も同じ扱いか確かめ、違えば揃えた
- [ ] docs/ui.md#webpuppet-の寿命・packages/player/README.md を直した。`just player-browser`・`just ci` が通る

## 参考

- T0486 のメモ (調べたことと判断)、T0938・T0941・T0946

## メモ

- オーケストレーターが利用者の判断を受けて直接起票した (T0486 の候補)。
