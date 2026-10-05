---
id: T0479
title: "適合テスト: <ugoe-avatar> で復元が届かないときの作り直しのシナリオ"
status: in-progress
milestone: M9
crates: [tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0479
---

# T0479: 適合テスト: <ugoe-avatar> で復元が届かないときの作り直しのシナリオ

## 目的

T0938 で、`<ugoe-avatar>` は WebGL2 のコンテキストを失って 5 秒のうちに復元が届かなければ新しい canvas で作り直す (作れなければ試し直し、
上限で諦める) ようになった。これは偽の wasm の `avatar.test.ts` でだけ確かめていて、本物の wasm と Chrome で動かす
tests/conformance の `avatar` のシナリオには時間切れが無い (T0938 の候補)。

## 受け入れ条件

- [ ] tests/conformance の `avatar` に、`WEBGL_lose_context` で失わせて復元させないシナリオを足し、`gpulost` (`waitingForGpu`)・時間切れの後の作り直し・
      作り直した後に描けること (画素か `ready`) を確かめる
- [ ] 作り直しを続けて失敗させたときに上限で諦めること (`error`) も確かめるか、確かめないなら理由をメモに書いた
- [ ] 時間切れの待ちで適合テストが大きく遅くならない (かかった時間をメモに書く)
- [ ] tests/conformance/README.md を直した
- [ ] `just conformance`・`just ci` が通る

## 参考

- T0938 のメモ (やらなかったこと・起票の候補)
- bindings/web/src/avatar.ts、tests/conformance/README.md、packages/player/browser-test (PuppetSurface の同じシナリオ `context-loss-timeout`)

## メモ

- T0038 で起票した (元の候補: T0938 (適合テストの avatar))。
