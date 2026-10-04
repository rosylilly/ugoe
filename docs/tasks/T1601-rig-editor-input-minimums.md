---
id: T1601
title: エディタの物理の入力欄に下限を付ける (負の delay・acceleration を入れさせない)
status: in-progress
milestone: M9
crates: [apps/editor]
depends_on: [T0693]
assignee: claude-worker-T1601
---

# T1601: エディタの物理の入力欄に下限を付ける (負の delay・acceleration を入れさせない)

## 目的

T0141 で物理の数値の検証を足したので、負の `delay`・`acceleration` などはコマンドがエラーになる。ところがエディタの物理の入力欄
(`RigEditor.svelte` ほか) には `min` が無く、利用者は負の値を入れられ、エラーになってから気づく。検証の規則 (docs/spec/model.md) に合わせて
入力欄に下限 (`min=0` など) を付ける。

## 受け入れ条件

- [ ] 物理のパネルの数値の欄のうち、検証が下限を持つものに `min` (と必要なら `max`) を付けた。対応を表にしてメモに書いた
- [ ] 下限の外の値を入れられないことをコンポーネントのテスト (または `NumberField` の既存のテストの形) で確かめた
- [ ] `just ci` が通る

## 参考

- T0141 のメモ (起票の候補)、docs/spec/model.md (物理の検証)
- apps/editor/src/lib/panels/physics/

## メモ

- T0019 で起票した (元の候補: T0141 の `min=0`)。同じ部品を触る T0693 の後に着手する。
