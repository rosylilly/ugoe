---
id: T0480
title: "<ugoe-avatar>: 作り直しが CreateError で失敗すると、復元の時間切れのタイマーを消さずに上書きする"
status: done
milestone: M9
crates: [bindings/web, docs]
depends_on: []
assignee: claude-worker-T0480
---

# T0480: <ugoe-avatar>: 作り直しが CreateError で失敗すると、復元の時間切れのタイマーを消さずに上書きする

## 目的

T0479 の適合テストを書く途中で、`<ugoe-avatar>` (bindings/web) の作り直しが `CreateError` で失敗すると、`#restoreTimer` に
時間切れのタイマーが 2 つ入ることが分かった。前のタイマーを `clearTimeout` せずに上書きしている (T0479・T0941 の候補)。
今は先に走った方が作り直しを始め、もう片方は消されるか何もせずに終わるので二重には数えていないが、上書きの前に消すべき。
`@ugoe/player` の `PuppetSurface` では起きないことを T0941 で確かめた。

## 受け入れ条件

- [x] 先に失敗するテスト (avatar の単体テスト) で、作り直しの失敗の後に時間切れのタイマーが 1 つだけであることを確かめる
- [x] タイマーを置き直す前に前のものを消す。作り直しの回数の数え方 (T0938) は変えない
- [x] `just conformance` (avatar のシナリオ) と `just ci` が通る

## 参考

- T0479・T0941 のメモ (起票の候補)、bindings/web/src/avatar.ts、tests/conformance/web/avatar.js

## メモ

- オーケストレーターが直接起票した (小さなバグなので候補の一覧を待たずに起票)。
- やったこと: `#setRestoreTimer` を足し、`#start` の失敗と `webglcontextlost` の両方の時間切れをここで置く (前のタイマーを `clearTimeout` してから置く)。
  `#onLost` (作り直しの回数の数え方) は変えていない。
- テスト: avatar.test.ts の偽の `createPuppet` に、失敗する前に新しい canvas の `webglcontextlost` を送る口 (`loseOnFailure`) を足し、
  作り直しの失敗の後に `vi.getTimerCount()` が 1 であること、残った時間切れで作り直せることを確かめた。直す前は 2 で落ちることを確かめた。
- 確かめたこと: `just conformance --only avatar` (7 of 7)・`just ci`。
- 起票の候補: なし。
