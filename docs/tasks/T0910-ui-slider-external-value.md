---
id: T0910
title: UI: Slider と NumberField が外から渡した値を寄せるときに onchange を呼ばないようにする
status: done
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

- [x] Slider と NumberField (と Slider の中の数値の欄) は、利用者の操作 (ドラッグ・キー・入力・リセット) のときだけ onchange を呼ぶ。外から渡した値の寄せでは呼ばない
- [x] そのことを確かめる Vitest (happy-dom)
- [x] スタジオの操作ウィンドウの回避 (T0802 で入れた、半刻み以下の変化を送らない処理) を外し、部品の修正だけで呼吸などが上書きされないことを確かめる
- [x] docs/ui.md の Slider の節に、外からの値と onchange の約束を書く
- [x] `just ci` が通る

## 参考

- docs/tasks/T0802-studio-scenes-and-profiles.md のメモ

## メモ

### やったこと (2026-10-02)

- `Slider`: Bits UI の `Slider` は、外から渡した値が刻みの格子に乗らないと、`watch` の中で格子に寄せて `onValueChange` を呼ぶ。
  つまみの操作の間 (`pointerdown` の capture から document の `pointerup`・`pointercancel`・`pointerleave` まで、キーは root の
  `keydown` の capture から bubble まで) だけ `onValueChange` を通すようにした。`onValueCommit` は Bits UI が操作のときにしか呼ばないので、そのまま。
  props と見た目は変えていない。
- `NumberField`: 書き換えたか (`dirty`。`input` で立て、確定・取り消しで下ろす) を持ち、書き換えていなければフォーカスを外しても Enter でも確定しない
  (前は、丸めた表示を確定して `onchange` を呼んでいた)。書き換えていない間はフォーカスがあっても外からの値に追従し (前は止まっていたので、外すと古い表示を
  書き込んだ)、書き換えの途中は上書きしない。Esc は書き換えていれば戻し、そうでなければ外側へ渡す (前と同じ)。
- テスト (`packages/ui/src/components.test.ts` の `external values`、`src/test/ExternalValueHarness.svelte`): 格子に乗らない値を何度渡しても
  `onchange`・`oncommit` を呼ばない・キーと欄とリセットとドラッグでは呼ぶ・ドラッグの後に渡した値では呼ばない・欄は丸めや古い表示で呼ばない・
  書き換えの途中の文字は外からの値で消えず確定できる。直す前の部品では 5 つのうち 4 つが落ちることを確かめた。
- スタジオ (`apps/studio/src/Control.svelte`): T0802 の回避 (半刻み以下の変化を送らない) を外し、`setParameter(id, value)` に戻した。
- `docs/ui.md` に「数値の部品と onchange」の節、`packages/ui/README.md` に 1 行。

### 確かめたこと

- `just ci`、`@ugoe/ui` と `@ugoe/studio` の svelte-check、ビューアの Vitest。
- ヘッドレスの Chrome と使い捨てのブリッジ (T0802 のものに、通知の値をフレームの値 (呼吸の後) にする変更を足した。scratchpad/t0910/bridge・check.mjs) で、
  `dress-up.ugoe` を開いた操作ウィンドウ: 何もしない 4 秒で `set_parameter` は 0 回、その間 `Breath`・`AngleX/Y/Z`・`BodyAngleX` は呼吸で動き続ける。
  矢印キー 1 回で 1 回、ドラッグで 6 回送り、その後の 3 秒は 0 回。直す前の部品で同じことをすると、何もしない 4 秒で 28 回送り、`Breath` が 1 に張り付き
  `BodyAngleX` が 10 まで流れた (上書きの再現)。スクリーンショットは scratchpad/t0910/shots/。

### 確かめていないこと

- 実際の Tauri のウィンドウ (WebView2・WKWebView) でのドラッグ。タッチ (pointer の種類が違う) とペン。
- 移行中のエディタ (T0663) で Slider を使う箇所の画面の確認 (Vitest は just ci で通る)。

### 範囲外の変更

- なし。

