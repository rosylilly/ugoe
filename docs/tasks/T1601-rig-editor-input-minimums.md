---
id: T1601
title: エディタの物理の入力欄に下限を付ける (負の delay・acceleration を入れさせない)
status: done
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

- [x] 物理のパネルの数値の欄のうち、検証が下限を持つものに `min` (と必要なら `max`) を付けた。対応を表にしてメモに書いた
- [x] 下限の外の値を入れられないことをコンポーネントのテスト (または `NumberField` の既存のテストの形) で確かめた
- [x] `just ci` が通る

## 参考

- T0141 のメモ (起票の候補)、docs/spec/model.md (物理の検証)
- apps/editor/src/lib/panels/physics/

## メモ

- T0019 で起票した (元の候補: T0141 の `min=0`)。同じ部品を触る T0693 の後に着手する。

### やったこと

範囲は `apps/editor/src/lib/panels/physics/physics.ts` の `PARTICLE_BOUNDS`・`normalizationBounds` に置き、`RigEditor.svelte` の欄に渡した。
`NumberField` は確定の値を `min`・`max` に収める (`aria-valuemin`・`aria-valuemax` も出る) ので、範囲の外の値はコマンドに乗らない。

| 欄 | 検証 (docs/spec/model.md#物理の検証) | 入力欄 |
| --- | --- | --- |
| 粒子の `mobility` (Sway) | 0〜1 | `min=0`・`max=1` (今回) |
| 粒子の `delay` (Speed)・`acceleration` (Settle)・`radius` (Length) | 0 以上 | `min=0` (今回) |
| 粒子の `collision_radius` (Hit r) | 0 以上 | `min=0` (今回) |
| 粒子の位置 x・y | 有限値だけ | なし |
| 正規化の範囲 (Input range) の min・default・max | `min <= default <= max` | min は `max=default`、default は `min=min`・`max=max`、max は `min=default` (今回) |
| 衝突体の `radius` | 0 以上 | `min=0` (既存。`ColliderSection.svelte`) |
| 出力の `particle` | 範囲外は無視 (エラーにしない) | `min=1`・`max=粒子の数-1` (既存) |
| 世界の `fps` | 有限値だけ (0 以下は既定の 60) | `min=1` (既存) |
| 重力・風・入力の `weight`・出力の `scale`・`weight` | 有限値だけ (出力の重みのクランプはランタイム) | なし |

- テスト: `panels/physics/RigEditor.test.ts` (happy-dom で `RigEditor` を描く。docs/testing.md#エディタの部品のテスト)。負の値・1 を超える `mobility`・
  正規化の順序を破る値を入れて Enter で確定し、送ったコマンドの値が範囲の端になることと、収めた値が今の値と同じなら何も送らないことを確かめた。
  変更を戻すと 9 件が失敗することを確かめた。
- docs/editor.md の「リグの編集」の行に範囲を書いた。

### やらなかったこと・気づいたこと

- 正規化の範囲は、ほかの 2 つの今の値を端にするので、範囲全体を上へずらすときは max (または default) から順に変える必要がある。順序の違反はどのみち検証エラーなので、この挙動にした。
- 作業中、main の `ToolOverlay.test.ts` の「Delete で頂点を消す」が失敗していた (T0693 で `apply` が失敗時に null を返すようになったのに、T0699 の `FakeDocument.apply` が null を返していた)。
  直すコミットを作ったが、rebase の時点で T0698 (33f6d028) が同じ修正を main に入れていたので、自分のコミットは落とした。

