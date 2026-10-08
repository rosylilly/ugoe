---
id: T0874
title: "スタジオの ui-audit: 偽のホストに自動の保存の一時の失敗を足し、失敗の帯を監査する"
status: todo
milestone: M8
crates: [tests/ui-audit, docs]
depends_on: []
assignee:
---

# T0874: スタジオの ui-audit: 偽のホストに自動の保存の一時の失敗を足し、失敗の帯を監査する

## 目的

T0870 で、自動の保存が一時の失敗 (ディスクが一杯・共有違反など) をしたら、操作ウィンドウに閉じられる赤の帯 (`Control.svelte` の `autosaveError`) を出し、
「変更を残し、少し待ってからもう一度自動で保存する」(`profile.autosaveRetryHint`) を添えるようにした。ui-audit の偽のホスト
(tests/ui-audit/lib/studio-host.mjs) は保存に失敗しないので、この帯のコントラスト・はみ出し・押せる大きさ (閉じるボタン)・名前の無い部品は
数えていない (T0870 のメモ)。偽のホストに保存の一時の失敗を起こす場面を足して監査する。

## 受け入れ条件

- [ ] 偽のホストに、保存 (`save_profile`) が一時の失敗のエラーのコードを返す切り替えを足し、場面 (`ctx.scene(…, { errors: true })`) で
      変更してから自動の保存を待ち、`autosaveError` の帯が出た状態を撮る。書けない場所の失敗 (書き換えないプロファイルになる) の帯も
      偽のホストで出せるなら同じく撮る。帯が出たことを場面の中で確かめる (出なければ場面を失敗にする)
- [ ] 日本語と英語の両方 (ui-audit の既定の言語の組) で撮り、狭い幅でもはみ出さない
- [ ] `just ui-audit --only studio` が通り (問題 0)、問題が出たら直した (直したのが画面なら `apps/studio` は範囲外なのでメモに書いて止まるか、小さな直しなら行ってメモに書く)
- [ ] tests/ui-audit/README.md の場面の一覧 (と「数えないもの」に自動の保存の失敗が挙がっていればそこ) を直した
- [ ] `just ci` が通る

## 参考

- T0870 のメモ (やったこと・確かめていないこと・起票の候補)、docs/studio.md#保存の失敗の種類、tests/ui-audit/README.md (場面の足し方)
- tests/ui-audit/lib/studio-host.mjs・lib/apps.mjs、apps/studio/src/lib/autosave.ts (`Autosave`・30 秒後の再試行)、apps/studio/src/Control.svelte (`autosaveError`)

## メモ

- T0081 で起票した (元の候補: T0870 (ui-audit の偽のホストに自動の保存が一時的に失敗する場面を足し、失敗の帯を監査する))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- 自動の保存は変えてから約 0.8 秒で走る。場面の待ちは固定の時間ではなく帯が出るのを待つ形にする (ui-audit の時間を延ばしすぎない)。
