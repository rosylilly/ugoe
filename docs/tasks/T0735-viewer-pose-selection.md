---
id: T0735
title: "ビューア: ポーズの要素を選ぶ画面"
status: done
milestone: M9
crates: [packages/player, apps/viewer, docs]
depends_on: [T0734]
assignee: claude-worker-T0735
---

# T0735: ビューア: ポーズの要素を選ぶ画面

## 目的

T0468 で見本のアバター (耳飾りの有無) と着せ替えの本体 (手を振る) がポーズを持つようになったが、ビューアにはポーズの要素を選ぶ画面が無い
(エディタのプレビューにはある。T0468 の候補)。wasm には `show_pose`・`pose_json` があるので、ビューアから見本のポーズを試せるようにする。

## 受け入れ条件

- [x] モデルにポーズがあれば、ビューアにグループごとに表示する要素を選ぶ欄があり、選ぶと `show_pose` で切り替わる (ポーズが無ければ欄を出さない)
- [x] モデルの読み直し・着せ替えの付け外しの後も、欄が今のポーズ (`pose_json`) と食い違わない
- [x] `packages/player` の Vitest と部品のテストがある。文言は `@ugoe/i18n` のカタログ (日英) にある
- [x] docs/manual のビューアの節を直した (画像の撮り直しは T0919 のメモに書き足す)
- [x] `just ci` が通る

## 参考

- T0468 のメモ (起票の候補)
- crates/ugoe-wasm/README.md (`show_pose`・`pose_json`)、docs/spec/motion.md (ポーズ)

## メモ

- T0036 で起票した (元の候補: T0468 (ビューアのポーズ))。同じパネルの周りを触る T0734 の後に着手する。

### やったこと

- `packages/player/src/pose.ts`: `parsePose` (`pose_json` → `PoseGroupInfo[]`。要素のパーツと、モジュールのグループならその名前)、
  `carryPoseShown` (付け外しの後、要素の並びが同じグループの選択を保ち、ほかは先頭の要素。`PosePlayer::carry_over` と同じ規則)、
  `poseEntryLabel` (モジュールの接頭辞を除く)・`poseGroupNumbers` (本体とモジュールごとに 1 から数える)。
- `Player`: `poseGroups`・`poseShown` と `showPose(part)`。グループは `#syncDressUp` のたびに (= モデルを読んだときと付け外しのたびに)
  読み直す。モデルを読むと前の選択を捨てて先頭の要素から。読み直した後は選んでいる要素を `show_pose` で渡し直し、欄と描く姿を
  食い違わせない。`wasm.ts` に `show_pose`・`pose_json` を足した。偽の WebPuppet にポーズ (本体・モジュールのグループ、表示中の要素、
  読み直しで初めに戻る) を足した。テストは `pose.test.ts`・`playerPose.test.ts`。
- ビューア: `PosePanel.svelte` (グループごとに `SegmentedControl`、要素が 5 個以上なら `Select`。docs/ui.md#部品の一覧)。
  ポーズのグループがあるときだけ、モーションのパネルの下に出す。文言は `viewer` のカタログ `poses.*` (日英)。部品のテスト
  `PosePanel.svelte.test.ts` (happy-dom)。
- 文書: docs/manual/dress-up.md に「ビューアでポーズを切り替える」、apps/viewer/README.md に「ポーズ (T0735)」、packages/player/README.md の
  ファイルの一覧。画像の撮り直しは T0919 のメモに書き足した。
- `just ci` が通った。`just ui-audit` も流し、ビューアの見本 (`avatar-lit.ugoe`) でパネルが出て、パネルに由来する指摘が無いことを確かめた
  (ビューアの指摘 2 件 (光源の向きの円の印・`.viewport` の nested-interactive) は前からあるもの)。

### やらなかったこと

- 欄はモーションに追従しない。モーションがパーツの不透明度のカーブで同じグループの要素を要求すると、描く姿はモーションに従うが、
  欄は最後に選んだ要素のまま (wasm に表示中の要素を読む口が無い。Rust には `PosePlayer::visible_parts` がある)。マニュアルと README に書いた。
  見本のモーションはポーズのパーツを動かさないので、見本では食い違わない。
- 選んだ要素はモデルごとに覚えない (開き直すと先頭の要素)。
- 実機の WebView での見た目は確かめていない (ヘッドレスの Chrome の ui-audit の画像だけ)。

### 起票の候補

- wasm (と C ABI・`@ugoe/runtime`) に表示中のポーズの要素を読む口 (`pose_visible_json` など。`PosePlayer::visible_parts`) を足し、
  ビューア・エディタのプレビューの欄をモーションによる切り替えに追従させる。
- スタジオのプロファイルやシーンで、ポーズの要素を選んで覚える (配信で衣装の差分を切り替える)。
