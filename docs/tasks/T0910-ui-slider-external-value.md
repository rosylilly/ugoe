---
id: T0910
title: UI: Slider と NumberField が外から渡した値を寄せるときに onchange を呼ばないようにする
status: in-progress
milestone: M8
crates: [packages/ui, apps/studio, docs]
depends_on: [T0902]
assignee: claude-worker-T0910
---

# T0910: 外からの値で onchange を呼ばない

## 目的

@ugoe/ui の Slider は、外から渡した値が刻み (step) の格子に乗らないと、寄せた値で onchange を呼ぶ。スタジオでは指揮役の値を表示するだけで
set_parameter が毎秒何度も送られ、呼吸などの自動の動きが手の値で上書きされた (T0802 のメモ。スタジオ側で半刻み以下の変化を送らない回避を入れた)。

## 受け入れ条件

- [ ] Slider と NumberField (と Slider の中の数値の欄) は、利用者の操作 (ドラッグ・キー・入力・リセット) のときだけ onchange を呼ぶ。外から渡した値の寄せでは呼ばない
- [ ] そのことを確かめる Vitest (happy-dom)
- [ ] スタジオの操作ウィンドウの回避 (T0802 で入れた、半刻み以下の変化を送らない処理) を外し、部品の修正だけで呼吸などが上書きされないことを確かめる
- [ ] docs/ui.md の Slider の節に、外からの値と onchange の約束を書く
- [ ] `just ci` が通る

## 参考

- docs/tasks/T0802-studio-scenes-and-profiles.md のメモ

## メモ
