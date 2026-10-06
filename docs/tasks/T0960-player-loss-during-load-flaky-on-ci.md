---
id: T0960
title: "Player: 適合テストの player-loss-during-load が CI で失敗する (コンテキストを失った後の load でモデルが開かれる)"
status: in-progress
milestone: M9
crates: [packages/player, docs]
depends_on: []
assignee: claude-worker-T0960
---

# T0960: Player: 適合テストの player-loss-during-load が CI で失敗する (コンテキストを失った後の load でモデルが開かれる)

## 目的

main の CI (https://github.com/rosylilly/ugoe/actions/runs/37429758787 、`conformance` の「PuppetSurface in the browser」= `just player-browser`) で、T0955 が足したシナリオが次で失敗した (20 of 21)。

```
FAIL player-loss-during-load: model avatar.ugoe
```

`packages/player/browser-test/scenarios.js` の `player-loss-during-load` で、`loseContext` → `waitingForGpu` を待った後の `player.load()` が、
コンテキストを失っている間なのにモデルを開いてしまった (`check(player.modelName !== "avatar.ugoe", ...)` が失敗)。
CI のブラウザ (ソフトウェアの描画) では、失ったことを wasm の `load_bundle` が `device_lost` で返さない時機があるか、Player が `waitingForGpu` の間の load を
保留していないかのどちらか。手元で通るかは要確認。

## 受け入れ条件

- [ ] 原因を確かめ、メモに書いた (Player の扱いの誤りか、シナリオの前提の誤りか)
- [ ] Player の誤りなら Player を直す (失っている間の load は保留して作り直した後に開く: T0955 の仕様)。シナリオの誤りならシナリオを直す
- [ ] `just player-browser` を何度か (例 5 回) 流して揺れないことを確かめた
- [ ] `just ci` が通る

## 参考

- T0955・T0486、packages/player、tests/conformance/web/avatar.js (`context-loss-during-load`)
- CI のページのログ: 失敗した run の成果物 `conformance` の `player-browser/page.log`

## メモ

- CI の失敗から起票した (2026-10-06)。
