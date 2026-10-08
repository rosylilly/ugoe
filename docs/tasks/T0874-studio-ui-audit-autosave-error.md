---
id: T0874
title: "スタジオの ui-audit: 偽のホストに自動の保存の一時の失敗を足し、失敗の帯を監査する"
status: done
milestone: M8
crates: [tests/ui-audit, docs]
depends_on: []
assignee: claude-worker-T0874
---

# T0874: スタジオの ui-audit: 偽のホストに自動の保存の一時の失敗を足し、失敗の帯を監査する

## 目的

T0870 で、自動の保存が一時の失敗 (ディスクが一杯・共有違反など) をしたら、操作ウィンドウに閉じられる赤の帯 (`Control.svelte` の `autosaveError`) を出し、
「変更を残し、少し待ってからもう一度自動で保存する」(`profile.autosaveRetryHint`) を添えるようにした。ui-audit の偽のホスト
(tests/ui-audit/lib/studio-host.mjs) は保存に失敗しないので、この帯のコントラスト・はみ出し・押せる大きさ (閉じるボタン)・名前の無い部品は
数えていない (T0870 のメモ)。偽のホストに保存の一時の失敗を起こす場面を足して監査する。

## 受け入れ条件

- [x] 偽のホストに、保存 (`save_profile`) が一時の失敗のエラーのコードを返す切り替えを足し、場面 (`ctx.scene(…, { errors: true })`) で
      変更してから自動の保存を待ち、`autosaveError` の帯が出た状態を撮る。書けない場所の失敗 (書き換えないプロファイルになる) の帯も
      偽のホストで出せるなら同じく撮る。帯が出たことを場面の中で確かめる (出なければ場面を失敗にする)
- [x] 日本語と英語の両方 (ui-audit の既定の言語の組) で撮り、狭い幅でもはみ出さない
- [x] `just ui-audit --only studio` が通り (問題 0)、問題が出たら直した (直したのが画面なら `apps/studio` は範囲外なのでメモに書いて止まるか、小さな直しなら行ってメモに書く)
- [x] tests/ui-audit/README.md の場面の一覧 (と「数えないもの」に自動の保存の失敗が挙がっていればそこ) を直した
- [x] `just ci` が通る

## 参考

- T0870 のメモ (やったこと・確かめていないこと・起票の候補)、docs/studio.md#保存の失敗の種類、tests/ui-audit/README.md (場面の足し方)
- tests/ui-audit/lib/studio-host.mjs・lib/apps.mjs、apps/studio/src/lib/autosave.ts (`Autosave`・30 秒後の再試行)、apps/studio/src/Control.svelte (`autosaveError`)

## メモ

- T0081 で起票した (元の候補: T0870 (ui-audit の偽のホストに自動の保存が一時的に失敗する場面を足し、失敗の帯を監査する))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- 自動の保存は変えてから約 0.8 秒で走る。場面の待ちは固定の時間ではなく帯が出るのを待つ形にする (ui-audit の時間を延ばしすぎない)。

### やったこと (T0874)

- 偽のホスト (`tests/ui-audit/lib/studio-host.mjs`) に `window.__uiAuditStudio.failSave(failure)` を足した。今の保存先への保存
  (`saveProfile()`。保存先を選んだ保存は失敗させない) が `failure.error` を投げる。書けない場所 (`failure.readOnly` が
  `notWritable`) なら、本物 (stage.rs の `save_profile` と lib.rs の `with_stage`) と同じく、書き換えないプロファイルの通知を送ってから失敗を返す。
- Node の側の `autosaveFailure(fixture, kind)` が、保存先のある変えたプロファイルの通知と失敗 (`profile.io`。args は `path`・`reason`。
  Linux の `std::io::Error` の文 "No space left on device (os error 28)"・"Permission denied (os error 13)") を作る。
  一時の失敗と書けない場所で保存先を変えてあるので、2 つ目の場面に移ると 1 つ目の帯は `Autosave` の回復で消えてから出直す。
- 場面 `-autosave-error` (一時の失敗: 赤の帯にエラーと `profile.autosaveRetryHint`) と `-autosave-denied` (書けない場所: 赤の帯と、
  写しの保存に誘う `profile.readOnlyHint` の帯) を `-approved` と `-first-run` の間に足した (`errors: true`)。帯は固定の時間ではなく
  `.ugoe-notice[data-tone=…]` に文が出るのを待ち (5 秒)、出なければ場面の失敗にする。前の場面の承認の知らせは閉じ、ポインタと
  フォーカスを外してから撮る (閉じたボタンの位置に後から出る帯の閉じるボタンの Tooltip が出ていたため)。
- 場面の失敗の効き目: 偽のホストの `throw` を一時的に `return null` に替えて `--variants en-light-wide` で流すと
  `FAILED the temporary autosave failure did not show the danger notice` になることを確かめた (戻してある)。
- 単体テスト (`test/hosts.test.mjs`): 作るプロファイルが画面の `shouldAutosave` で自動の保存の対象になること・拡張子・エラーのコードと
  カタログの文言・一時の失敗と書けない場所の違い・保存先が違うこと。
- README の場面の一覧と偽のホストの説明を直した。「数えないもの」には自動の保存の失敗は挙がっていなかったので直していない。
- `just ui-audit --only studio`: 213 場面で問題 0 (日本語・英語、狭い幅 360px・半分・強制色を含む)。`just e2e --only studio` も通る。
- 撮った画像を見て直した画面 (`apps/studio` は範囲外だが小さな直し): 一時の失敗の帯で、`autosaveRetryHint` が OS のエラーの文に
  続けて同じ行に出ており (日本語で「(os error 28)変更はそのまま残し…」)、どこまでがエラーか分からなかった。`Control.svelte` で
  エラーと添える文をそれぞれ `<p>` にして段落を分けた (`ModuleReturnBar` などと同じ書き方)。機械の検査では数えない見た目の問題なので、
  直す前に落ちる検査は無い (撮った画像で前と後を見比べた)。
- `just ci` が通った。

### 確かめていないこと

- Windows・macOS の OS のエラーの文 (偽のホストは Linux の文だけ)。
