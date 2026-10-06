---
id: T1634
title: "エディタ: キーフォームの軸のボタンとパラメータの下のキーの印を 24px の押せる範囲にする"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: [T0675]
assignee: claude-worker-T1634
---

# T1634: エディタ: キーフォームの軸のボタンとパラメータの下のキーの印を 24px の押せる範囲にする

## 目的

T0675 で足した `just ui-audit` の場面 `-gizmo` (デフォーマを選んだ画面) で、インスペクタのキーフォームの軸の「キーへ移る」「キーを消す」
(`button.go`・`button.remove`。高さ 16px) と、パラメータのスライダーの下のキーの印 (`button.marker`。8px) が 24px 未満として数えられた (T0675 の候補)。
パネルの中なので T0675 では直していない。

## 受け入れ条件

- [x] キーフォームの軸の 2 つのボタンとキーの印の押せる範囲が 24px 以上 (見た目の大きさは変えなくてよい。隣の印と範囲が重なるときの選び方をメモに書く)
- [x] `just ui-audit --only editor` の `-gizmo` の場面で、この 3 種類の指摘が 0 件。docs/ui.md の「残したもの」から消した
- [x] 部品のテスト (押せる範囲の端で押してもその操作になる) がある
- [x] `just ci` が通る

## 参考

- T0675 のメモ (監査・起票の候補)、docs/ui.md
- apps/editor/src/lib/panels/ParametersPanel.svelte、インスペクタのキーフォームの節

## メモ

- T0041 で起票した (元の候補: T0675 (24px の押せる範囲))。
- やったこと:
  - インスペクタのキーフォームの軸の `button.go`・`button.remove` に `min-width`・`min-height` 24px (`--ugoe-target-min`)。札は 2px 高くなる。
  - パラメータのキーの印は見た目の◆ (6px、`::before`) をそのままに、押せる範囲を 24×24 の透明な枠にした。印をスライダーの下の帯 (`.markers`。
    上の 4px をスライダーの下の余白に、下の 8px を行の間の余白に重ねる。帯は `pointer-events: none`) に並べた。行は 12px ほど高くなる。
  - **重なったときの選び方:** 近いキーの印は押せる範囲が重なる。重なった所では、上に描かれた印ではなく、押した所に中心が最も近い印のキーへ移る
    (`parameterValue.ts` の `nearestMarkerKey`)。キーボード (`detail` が 0 の click) ではフォーカスのある印のキー。
  - テスト: `ParametersPanel.test.ts` (範囲の両端で押す・重なり・キーボード)、`KeyformInspector.test.ts` (キーへ移る・キーを消す)、
    `parameterValue.test.ts` (`markerLeft`・`nearestMarkerKey`)。happy-dom はレイアウトをしないので、帯の大きさは `getBoundingClientRect` を差し替えて与えた。
    24px の大きさそのものは `just ui-audit` で確かめた。
  - docs/ui.md (T0675 の節の残りの記述を直し、「キーフォームの軸とキーの印 (T1634)」を足した)・docs/editor.md (パラメータ一覧の印の説明)。
- `just ui-audit --only editor --compare` (前は 49316e91): `button.go` 27x16・`button.remove` 24x16・`button.marker` と `.current` 8x8 の 4 種類が消え、
  すべての検査で 0 件 (273 場面)。`just e2e --only editor` は 15 件すべて通った。
- `just ci` は通った。ただし 1 回目は `apps/editor` の Vitest で、`src/lib/panels/unreadableModel.test.ts` に由来する未処理の拒否が 2 件出て失敗した
  (`ugoe_wasm_bg.wasm` の fetch がテストの片付けで中断される。テストは 696 件すべて通る)。`just e2e`・`just ui-audit` が作った
  `src/lib/viewport/generated/` (gitignore) があるときだけ起きる (取り除くと 0 件)。`TimelinePanel` が `loadUgoeWasm` を呼ぶため。この変更とは無関係。
- 起票の候補: エディタの Vitest が、`just wasm` の生成物 (`src/lib/viewport/generated/`) があると `panels/unreadableModel.test.ts` の
  `TimelinePanel` の wasm の読み込みで未処理の拒否を出して失敗する (`just e2e` の後の `just ci` が落ちる)。テストで `loadUgoeWasm` を `vi.mock` するなど。
