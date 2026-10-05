---
id: T0736
title: "ビューアとエディタのプレビュー: ポーズの欄をモーションによる切り替えに追従させる"
status: todo
milestone: M9
crates: [packages/player, apps/viewer, apps/editor, docs]
depends_on: [T0478]
assignee:
---

# T0736: ビューアとエディタのプレビュー: ポーズの欄をモーションによる切り替えに追従させる

## 目的

T0735 のビューアのポーズの欄 (`PosePanel.svelte`) とエディタのプレビューのポーズの欄は、最後に選んだ要素を出すだけで、
モーションがパーツの不透明度のカーブで同じグループの要素を要求しても欄は変わらない (描く姿とは食い違う。マニュアルと README に書いてある)。
T0478 の表示中の要素を読む口で、欄を追従させる (T0735 の候補)。

## 受け入れ条件

- [ ] `Player` が表示中の要素を読み (毎フレームか、再生中だけ間引いて)、`poseShown` をモーションによる切り替えに追従させる。偽の WebPuppet の Vitest で確かめる
- [ ] ビューアの欄とエディタのプレビューの欄が追従する (部品のテスト)。利用者が選んだ後にモーションが切り替えたときの規則 (どちらが勝つか) を docs に書いた
- [ ] 見本の `wave` (着せ替えの本体) で欄が `Wave` → `HandsDown` と変わることを確かめた (ブラウザのテストか手での確認。どちらかをメモに書く)
- [ ] docs/manual/dress-up.md・apps/viewer/README.md・packages/player/README.md の「欄はモーションに追従しない」を直した。画像の撮り直しが要れば T0919 のメモに書き足した
- [ ] `just ci` が通る

## 参考

- T0735 のメモ (やらなかったこと・起票の候補)、T0478
- packages/player/src/pose.ts・player.svelte.ts、apps/viewer/src/lib/PosePanel.svelte、apps/editor/src/lib/preview/PreviewPanel.svelte

## メモ

- T0038 で起票した (元の候補: T0735 (欄の追従))。
