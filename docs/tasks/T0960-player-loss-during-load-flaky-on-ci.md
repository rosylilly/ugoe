---
id: T0960
title: "Player: 適合テストの player-loss-during-load が CI で失敗する (コンテキストを失った後の load でモデルが開かれる)"
status: done
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

- [x] 原因を確かめ、メモに書いた (Player の扱いの誤りか、シナリオの前提の誤りか)
- [x] Player の誤りなら Player を直す (失っている間の load は保留して作り直した後に開く: T0955 の仕様)。シナリオの誤りならシナリオを直す
- [x] `just player-browser` を何度か (例 5 回) 流して揺れないことを確かめた
- [x] `just ci` が通る

## 参考

- T0955・T0486、packages/player、tests/conformance/web/avatar.js (`context-loss-during-load`)
- CI のページのログ: 失敗した run の成果物 `conformance` の `player-browser/page.log`

## メモ

- CI の失敗から起票した (2026-10-06)。
- 原因 (シナリオの前提の誤り。Player は正しい): CI の失敗の文 `model avatar.ugoe` は、起票時に見た 982 行目
  (`the model was opened while the context is lost`) ではなく、2 回目の喪失の後の
  `player.modelName === "dress-up.ugoe"` の検査 (文は `model ${player.modelName}`) のもの。Player は失っている間の load を正しく保留していた。
  シナリオは作り直しの完了を「canvas が替わり `status === "ready"`」で待っていたが、作り直しの途中も `status` は前のモデルの
  `ready` のままで、PuppetSurface は `WebPuppet.create` を待つ前に新しい canvas を置く。作成が遅い CI (SwiftShader) では、
  作り終える前 (保留した dress-up.ugoe を開く前) に条件が満ちて確かめていた。page.log の順 (喪失→作り直し→喪失→作り直し、
  時間切れの作り直しは無い) もこれと合う。`player-loss-restore` も同じ条件で待っていた (モデル名は変わらないので落ちにくいだけ)。
- やったこと (packages/player/browser-test/scenarios.js): 待ち方を `untilPlayerRecreated` にまとめ、`waitingForGpu` が下りる
  (`created`) まで待つようにした (`player-loss-restore`・`player-loss-during-load`)。`WebPuppet.create` の包みに作成を遅らせる
  `createDelayMs` を足し、この 2 つのシナリオで 500 ms 遅らせて CI の遅さを手元で再現する。直す前の待ち方に遅れだけを入れると
  CI と同じ `FAIL player-loss-during-load: model avatar.ugoe` で落ちることを確かめた。直した後は `just player-browser` を 5 回流して
  5 回とも 21 of 21。
- やらなかったこと: Player の振る舞い (作り直しの間の `status`) は変えていない (アプリは `waitingForGpu` で待っている間を示す)。

