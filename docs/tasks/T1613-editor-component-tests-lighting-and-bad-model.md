---
id: T1613
title: "エディタ: 光源のドラッグと、読めないモデルで部品が止まらないことを部品の単位でテストする"
status: in-progress
milestone: M9
crates: [apps/editor]
depends_on: [T1612]
assignee: claude-worker-T1613
---

# T1613: エディタ: 光源のドラッグと、読めないモデルで部品が止まらないことを部品の単位でテストする

## 目的

T0699 でビューポートのツールの部品を描くテストの道具 (`viewport/tools/test/harness.svelte.ts` の `FakeDocument`) ができた。

- `LightingOverlay` (`viewport/tools/lighting/LightingOverlay.svelte`) の光源のドラッグのテストが無い (T0699 の候補)。
- モデルの JSON が読めないとき (`doc.parsedModel` が無い) にパネルと重ねものが止まらないことを、部品ごとに確かめるテストが無い (T0698 の候補)。

## 受け入れ条件

- [ ] `LightingOverlay` のドラッグで、離したときに 1 つのコマンドが送られ、ドラッグ中は送らないテストがある
- [ ] 読めないモデルで、主なパネルと重ねもの (どれを対象にしたかをメモに) が例外を出さずに描けるテストがある
- [ ] `just ci` が通る

## 参考

- T0699・T0698 のメモ (起票の候補)

## メモ

- T0026 で起票した (元の候補: T0699 の `LightingOverlay`、T0698 の部品の単位のテスト)。部品を触る T1612 の後に着手する。
- T0699 の候補「T0698 が `doc.model` の読み方を変えたら `FakeDocument` にも足す」は T0698 で入った (`parsedModel`) ので起票しない。
