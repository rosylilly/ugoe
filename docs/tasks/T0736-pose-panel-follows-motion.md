---
id: T0736
title: "ビューアとエディタのプレビュー: ポーズの欄をモーションによる切り替えに追従させる"
status: done
milestone: M9
crates: [packages/player, apps/viewer, apps/editor, docs]
depends_on: [T0478]
assignee: claude-worker-T0736
---

# T0736: ビューアとエディタのプレビュー: ポーズの欄をモーションによる切り替えに追従させる

## 目的

T0735 のビューアのポーズの欄 (`PosePanel.svelte`) とエディタのプレビューのポーズの欄は、最後に選んだ要素を出すだけで、
モーションがパーツの不透明度のカーブで同じグループの要素を要求しても欄は変わらない (描く姿とは食い違う。マニュアルと README に書いてある)。
T0478 の表示中の要素を読む口で、欄を追従させる (T0735 の候補)。

## 受け入れ条件

- [x] `Player` が表示中の要素を読み (毎フレームか、再生中だけ間引いて)、`poseShown` をモーションによる切り替えに追従させる。偽の WebPuppet の Vitest で確かめる
- [x] ビューアの欄とエディタのプレビューの欄が追従する (部品のテスト)。利用者が選んだ後にモーションが切り替えたときの規則 (どちらが勝つか) を docs に書いた
- [x] 見本の `wave` (着せ替えの本体) で欄が `Wave` → `HandsDown` と変わることを確かめた (ブラウザのテストか手での確認。どちらかをメモに書く)
- [x] docs/manual/dress-up.md・apps/viewer/README.md・packages/player/README.md の「欄はモーションに追従しない」を直した。画像の撮り直しが要れば T0919 のメモに書き足した
- [x] `just ci` が通る

## 参考

- T0735 のメモ (やらなかったこと・起票の候補)、T0478
- packages/player/src/pose.ts・player.svelte.ts、apps/viewer/src/lib/PosePanel.svelte、apps/editor/src/lib/preview/PreviewPanel.svelte

## メモ

- T0038 で起票した (元の候補: T0735 (欄の追従))。
- やったこと:
  - `@ugoe/player`: `Player` がふつうのモードで毎フレーム `update` の後に `pose_visible_json()` を読み、前のフレームの JSON と違うときだけ
    `poseShown` を合わせる (`pose.ts` の `followPoseShown`。グループは要素のパーツで突き合わせる。`parsePose` が要素の無いグループを
    飛ばしても並びがずれない)。グループが無いモデルでは読まない。グループを読み直したら次のフレームで読み直す。受け手のモードでは読まない。
    `WebPuppet` の型に `pose_visible_json` を足し、偽の WebPuppet も表示中の要素を返す。Vitest: `pose.test.ts`・`playerPose.test.ts`
    (モーションの切り替えへの追従・変わらないフレームで配列を作り直さない・選んだ後にモーションが要求すれば欄はモーションの要素・付け外しの後も追従)。
  - エディタのプレビュー: `PreviewMode.afterUpdate` で同じように読み、`poseShown` (編集中のモデルの ID。`previewPart` で WebPuppet の ID にして
    突き合わせる) と `modulePoseShown` を合わせる (`preview.ts` の `visiblePoseParts`・`followShown`)。テスト: `preview.test.ts`・
    `poseFollow.test.ts` (PreviewMode と偽の WebPuppet)・`posePanel.test.ts` (PreviewPanel の欄が表示する要素に追従する)。
  - ビューア: `PosePanel.svelte.test.ts` に欄が `poseShown` に追従するテストを足し、欄の説明文 (en・ja) に「欄もモーションの選んだパーツになる」を足した。
  - 規則 (後から来たほうが勝つ: モーションが要求している間はモーションの要素、モーションが要求をやめた後に選んだ要素は残る) を
    packages/player/README.md・apps/viewer/README.md・docs/editor.md・docs/manual/dress-up.md・docs/manual/preview.md に書いた。
  - 見本の `wave` は **ブラウザのテスト** で確かめた: `just player-browser --only player-pose-follows-motion` (本物の wasm とヘッドレスの Chrome。
    `dress-up.ugoe` で `HandsDown` → `Wave` → `HandsDown` と追従し、終わった後に選んだ `Wave` が残る)。`#followPose` を呼ばないようにすると
    このシナリオが落ちることも確かめた。docs/testing.md のシナリオの一覧に足した。
  - 画像の撮り直し: 欄の見た目は変わらず、ビューアのポーズの欄を写した画像も無い (dress-up.md の `module-compare-pose.png` はエディタの違いの強調) ので、T0919 には書き足していない。
- やらなかったこと:
  - スタジオの操作ウィンドウのポーズのパネル (T0842) は Player ではなく指揮役 (Rust の側) のシーンの選択を出すので、追従させていない
    (docs/studio.md・docs/manual/studio.md の「パネルの表示はモーションに追従しない」はそのまま。正しい記述)。
  - 再生中だけ間引く読み方はしていない (毎フレーム読み、JSON の文字列を前と比べるだけ。グループの無いモデルでは読まない)。
- 起票の候補:
  - スタジオの操作ウィンドウのポーズのパネルを、指揮役の Puppet の表示中の要素 (`PosePlayer::visible_parts`) に追従させる
    (シーンに覚える選択とモーションの切り替えをどう分けて見せるかの決めが要る)。

