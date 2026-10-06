---
id: T0948
title: "ビューアとスタジオのポーズのパネルを共有の部品にまとめる"
status: done
milestone: M9
crates: [packages/player, apps/viewer, apps/studio, docs]
depends_on: [T0736]
assignee: claude-worker-T0948
---

# T0948: ビューアとスタジオのポーズのパネルを共有の部品にまとめる

## 目的

ビューア (`apps/viewer/src/lib/PosePanel.svelte`、T0735) とスタジオ (`apps/studio/src/PosePanel.svelte`、T0842) のポーズのパネルは、
グループごとに `SegmentedControl` (5 個以上は `Select`) を出す作りが同じ (T0842 の候補)。表示する要素の出どころ (ビューアは `Player`、スタジオはバックエンドの通知) だけが違う。
1 つの部品にまとめ、T0736 (ビューアとエディタのプレビューの欄をモーションに追従させる) の振る舞いも揃える。

## 受け入れ条件

- [x] グループと選んでいる要素と選んだときの処理を受け取る共有の部品が、`@ugoe/player` (か `@ugoe/ui`。置き場と理由をメモに書く) にあり、部品のテストがある
- [ ] ビューアとスタジオ (とエディタのプレビューにポーズの欄があればそれも) がその部品を使い、各アプリの `PosePanel.svelte` のテストが通る (重複したテストは部品の側へ移す)
- [x] `just ui-audit --only viewer,studio` の指摘が増えない
- [x] docs/ui.md (部品の一覧) と packages の README を直した
- [x] `just ci` が通る

## 参考

- T0842 のメモ (起票の候補)、T0736
- apps/viewer/src/lib/PosePanel.svelte、apps/studio/src/PosePanel.svelte

## メモ

- T0040 で起票した (元の候補: T0842 (`PosePanel.svelte`))。ビューアのパネルを触る T0736 (作業中) の後に着手する。

### やったこと

- **置き場は `@ugoe/player`** (`packages/player/src/PosePicker.svelte`、`index.ts` から `PosePicker`): 欄は `PoseGroupInfo` と
  `poseGroupNumbers`・`poseEntryLabel` (どちらも `@ugoe/player` の `pose.ts`) を使い、見出しの文 (「グループ n」) もポーズの言葉なので、
  汎用の部品だけを持つ `@ugoe/ui` ではなく再生の層に置いた (`@ugoe/face-tracking` の `TrackingPanel` と同じ形。依存の向き
  `@ugoe/player → @ugoe/ui` は docs/ui.md のとおり)。`@ugoe/player` に `@ugoe/ui` の依存 (と部品のテストのための happy-dom) を足し、
  `vitest.config.ts` で svelte をブラウザ向けの版で解決するようにした (ビューアと同じ。既存のテストもすべて通る)。
- props は `groups`・`shown`・`disabled`・`onchange(part)`。`shown` をそのまま出すので、ビューアは `Player.poseShown` を渡してモーションによる
  切り替えに追従する (T0736 の振る舞い)。`onchange` が Promise を返す間は選んだ要素を先に出す (スタジオの T0842 の振る舞い。同期で返すビューアでは
  何もしない)。見出しの文はカタログ `player` の `pose.group`・`pose.moduleGroup` へ移し、ビューアとスタジオの `poses.group`・`poses.moduleGroup` を消した。
- ビューアとスタジオの `PosePanel.svelte` は、パネルの見出し・グループ数・説明・空のときの文だけを持つ薄い包みにした。
- テスト: `PosePicker.svelte.test.ts` (グループとモジュールの見出し・表示名・付け外し・選んだ要素を渡す・`shown` への追従・送っている間の表示と
  届いた値への戻り・5 個以上で Select・無効)。重複していたアプリのテストは部品へ移し、アプリには Player との結び付き (ビューア: グループ数・
  `showPose`・`poseShown` への追従) と送る処理と空のとき (スタジオ) だけを残した。
- 文書: docs/ui.md (「パッケージ」の置き場の約束と、部品の一覧の下に「パッケージの部品」の表 (`TrackingPanel`・`PosePicker`))、
  packages/player/README.md (依存・ファイルの一覧・「画面の部品」の節)、apps/viewer/README.md・docs/studio.md (欄が共有の部品であること)。
- `just ui-audit --only viewer,studio --compare` (着手前の `55895eb3` と比べた): viewer `+0 -0 ~0`・studio `+0 -0 ~0` (どちらも指摘 0 件)。
- `just e2e --only viewer` は通った。`just ci` が通った。

### やらなかったこと

- 受け入れ条件の 2 つ目は、ビューアとスタジオは満たしたが、エディタのプレビューが残るのでチェックしていない (下)。

- **エディタのプレビューのポーズの欄は移していない** (`apps/editor` はこのタスクの `crates:` の外)。プレビューの欄は編集中のモデルの
  `pose.groups` (要素はパーツの名前 `labelOf` で出す) とモジュールのグループ (見出しは `modulePoseLabels` のスロットの名前) を別々に並べ、
  どのグループも Select で、`showPose(group, part)` はグループを受け取るので、`PosePicker` に見出しと表示名を差し替える口を足す必要がある。
- スタジオのグループの間の隙間は `--ugoe-space-3` から部品の `--ugoe-space-2` (ビューアと同じ) になった (説明の文との間は `space-3` のまま)。

### e2e のスタジオの失敗 (この変更と無関係)

- `just e2e --only studio` は 13 手順のうち 2 つが落ちる: 「connects a client to the studio API」「serves the browser source」
  (`timed out after 10000ms waiting for the API badge` / `the source badge`)。手順は `.studio__api` の要素を探すが、
  この class は main (`d5ac4e4a`) の `apps/studio/src` のどこにも無い (`git grep studio__api main -- apps/studio/src` が空。
  失敗時の画面では「API on」の印は出ている)。ポーズのパネルを含むほかの手順は通った。

### 起票の候補

- エディタのプレビューのポーズの欄を `PosePicker` にする (見出しと要素の表示名を差し替える props を足す。2〜4 個は SegmentedControl になるので
  エディタの e2e とプレビューのテスト (`posePanel.test.ts`) も直す)。
- e2e のスタジオの「API」「ブラウザソース」の手順を、今の操作ウィンドウの状態の印 (T0817) の探し方に直す。
