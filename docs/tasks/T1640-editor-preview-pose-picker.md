---
id: T1640
title: "エディタ: プレビューのポーズの欄を @ugoe/player の PosePicker に移す"
status: done
milestone: M9
crates: [apps/editor, packages/player, docs]
depends_on: [T0676]
assignee: claude-worker-T1640
---

# T1640: エディタ: プレビューのポーズの欄を @ugoe/player の PosePicker に移す

## 目的

T0948 で、ビューアとスタジオのポーズの欄を `@ugoe/player` の `PosePicker` にまとめた。エディタのプレビューのポーズの欄は、見出しと要素の表示名が違うので
自前のまま残った (T0948 の候補)。

## 受け入れ条件

- [x] `PosePicker` に見出しと要素の表示名を差し替える props があり (ビューア・スタジオの表示は変わらない)、部品のテストがある
- [x] エディタのプレビューのポーズの欄が `PosePicker` を使う。要素が 2〜4 個のグループは SegmentedControl になるので、プレビューのテスト (`posePanel.test.ts`) と
      エディタの e2e を直した。T0736 のモーションへの追従のテストが通る
- [x] packages/player/README.md と docs/editor.md を直し、画面が変わったところを T0919 のメモに書き足した
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0948 のメモ (やったこと・起票の候補)、T0736 (欄の追従)
- packages/player/src/PosePicker.svelte、apps/editor/src (プレビューのポーズの欄)

## メモ

- T0041 で起票した (元の候補: T0948 (エディタのプレビューのポーズの欄))。T0676 (作業中) がモジュール・着せ替えのパネルを触るので、その後に着手する。

### やったこと

- **`PosePicker` の props** (`packages/player/src/PosePicker.svelte`): 見出しを差し替える `groupLabel(group, n)` (`n` は `poseGroupNumbers` の
  番号) と、要素の表示名を差し替える `entryLabel(group, part)` を足した。省くと今までの既定 (「グループ n」「<モジュール> のグループ n」と
  `poseEntryLabel`) なので、ビューアとスタジオは変えていない。`onchange` に選んだグループの位置 (`index`) を 2 つ目の引数で渡すようにした
  (エディタは本体のグループとモジュールのグループで送り先が違い、要素のパーツだけではどちらか決められないため。ビューア・スタジオは使わない)。
  部品のテストに、渡した見出しと表示名で出すことと、`index` を渡すことを足した。
- **エディタのプレビュー** (`apps/editor/src/lib/preview/PreviewPanel.svelte`): ポーズの欄を `PosePicker` にした。`preview.ts` の
  `previewPoseGroups` が編集中のモデルのグループ (`module: null`) の後にモジュールのグループを並べて表示中の要素を出し、
  `previewPoseGroupLabel` がエディタの文言の見出し (「Group N」「`<モジュール>` group N」・名前の無いモジュール) を作る
  (`modulePoseLabels` を置き換えた)。本体の要素の表示名は今までどおり `labelOf` (パーツの名前と ID)、モジュールの要素は接頭辞を除いたもの。
  選んだグループの位置で `showPose`・`showModulePose` に振り分ける。要素が 2〜4 個のグループは SegmentedControl になった。
- テスト: `posePanel.test.ts` (欄の探し方を `PosePicker` に合わせ、見出し・表示名・本体とモジュールへの振り分け・段を切ると選べないことを足した。
  T0736 の追従のテストも通る)、`preview.test.ts` (`previewPoseGroups`・`previewPoseGroupLabel`)、`unreadableModel.test.ts` (欄の探し方)。
  偽の `PreviewMode` (`test/fakePreview.svelte.ts`) は選んだ要素も記録するようにした。
- 文書: packages/player/README.md (props と使うアプリ)、docs/ui.md (パッケージの部品の表)、docs/editor.md (プレビューモードのポーズ)、
  docs/manual/preview.md (2〜4 つはボタンの並び)、T0919 のメモ (撮り直す画像)。
- `just ui-audit --compare` (着手前の `9624c15d` の `apps/editor/src`・`packages/player/src` で撮った結果と比べた): editor `+0 -0 ~0`、
  viewer `+0 -0 ~0`、studio `+0 -0 ~0`。エディタの `-preview-dress-up`・`-module-preview` の場面で「Pose」の欄が radiogroup「Group 1」
  (`HandsDown` / `Wave`) になったことを aria の写しで確かめた。
- `just e2e --only editor`・`--only viewer`・`--only studio` はすべて通った (エディタの e2e はプレビューのポーズの欄を操作していないので、
  直すところは無かった)。`just ci` が通った (1 回目は `apps/editor/src/lib/panels/unreadableModel.test.ts` の実行中に happy-dom の
  `Fetch.onError` の未処理のエラーが 2 つ出て落ちた。テストは 689 件すべて通っており、そのファイルはこの変更で触っていない。単独と
  エディタの vitest の全体を 2 回流し直しても出ず、2 回目の `just ci` は通った。並行するほかのワーカーの負荷による揺らぎとみている)。

### やらなかったこと

- docs/manual/images の画像の撮り直し (T0919 のメモに書いた)。

### 起票の候補

- `apps/editor/src/lib/panels/unreadableModel.test.ts` が負荷の高いときに happy-dom の fetch の未処理のエラー (`Fetch.onError`) を出すことがある。
  何を fetch しているかを調べ、テストの中で止める。

