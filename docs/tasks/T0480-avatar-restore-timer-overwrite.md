---
id: T0480
title: "<ugoe-avatar>: 作り直しが CreateError で失敗すると、復元の時間切れのタイマーを消さずに上書きする"
status: in-progress
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

- [ ] 先に失敗するテスト (avatar の単体テスト) で、作り直しの失敗の後に時間切れのタイマーが 1 つだけであることを確かめる
- [ ] タイマーを置き直す前に前のものを消す。作り直しの回数の数え方 (T0938) は変えない
- [ ] `just conformance` (avatar のシナリオ) と `just ci` が通る

## 参考

- T0479・T0941 のメモ (起票の候補)、bindings/web/src/avatar.ts、tests/conformance/web/avatar.js

## メモ

- オーケストレーターが直接起票した (小さなバグなので候補の一覧を待たずに起票)。
