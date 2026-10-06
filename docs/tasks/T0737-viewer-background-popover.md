---
id: T0737
title: "ビューア: 背景の色と「透明」を「背景」の Popover にまとめ、360px のツールバーを 2 行にする"
status: done
milestone: M10
crates: [apps/viewer, packages/ui, docs]
depends_on: []
assignee: claude-worker-T0737
---

# T0737: ビューア: 背景の色と「透明」を「背景」の Popover にまとめ、360px のツールバーを 2 行にする

## 目的

T0725 でビューアのツールバーは幅 360px で 4 行から 3 行になったが、部品の幅の合計が収まらず 3 行が残った (T0725 の候補)。
利用者の判断 (2026-10-06): 案 A。背景の色と「透明」を 1 つの「背景」ボタンの Popover にまとめる。どちらもまれな操作なので、
docs/ui.md の「まれな操作は奥に置く」に合う。

## 受け入れ条件

- [x] ツールバーの背景の色と「透明」を「背景」ボタンの Popover にまとめた。キーボードで開閉でき、フォーカスが戻る
- [x] 幅 360px でツールバーが 2 行に収まる (`just ui-audit --only viewer` の画像で確かめ、`--compare` で新しい崩れが無い)
- [x] 部品のテストがあり、docs/ui.md の「ビューアの見直し (T0725)」の「残したもの」と apps/viewer/README.md を直した
- [x] `just ci` が通る

## 参考

- T0725 のメモ (360px のツールバーの案)、docs/ui.md

## メモ

- オーケストレーターが利用者の判断を受けて直接起票した (T0725 の候補)。
- **やったこと:** `apps/viewer/src/lib/BackgroundPopover.svelte` を足し、ツールバーの `ColorPicker` と `Toggle` を「背景」ボタン 1 つ
  (今の背景の見本を出す。透明なら市松) の `@ugoe/ui` の `Popover` にまとめた (中身は「色」の `ColorPicker` と「透明」の `Toggle`)。
  文言は `toolbar.background.color` を足した。`@ugoe/ui` は変えていない (Popover は既にあった)。
  - 部品のテスト (`BackgroundPopover.svelte.test.ts`): ボタンが 1 つで見本が出る・中身・Esc で閉じてボタンへフォーカスが戻る・
    透明と色の変更で他方を保つ・中の色の枠を Esc で閉じても「背景」の枠は開いたまま (Popover の入れ子)。
  - `just ui-audit --only viewer`: 前 (`target/ui-audit-before`) も後も全検査 0 件、`--compare` は `+0 -0 ~0`。
    360px の画像 (日英・ライト・ダーク) でツールバーは 2 行 (開く・サンプル・覚えたもの / 背景・画像を保存・設定)。
  - **範囲の外 (小さな変更):** `tests/ui-audit/lib/apps.mjs` にビューアの場面 `-background` (「背景」にフォーカスして Enter で開く) を足し、
    `tests/ui-audit/README.md` の場面の表を直した。実際の Chrome でキーボードで開け、中にフォーカスが移ることを画像で確かめた。
  - docs/ui.md の「ビューアの見直し (T0725)」に背景の項を足し、「残したもの」(360px の 3 行) を消した。apps/viewer/README.md を直した。
- **確かめていないこと:** Tauri の実際のウィンドウ (WebView) では開いていない (監査のヘッドレスの Chrome と happy-dom だけ)。
- **起票の候補:** なし。
