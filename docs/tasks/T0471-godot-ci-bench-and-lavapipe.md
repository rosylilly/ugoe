---
id: T0471
title: "Godot: CI の bench.gd の結果を README に記録し、CI の lavapipe で Forward+ が動くか確かめる"
status: in-progress
milestone: M9
crates: [bindings/godot, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0471
---

# T0471: Godot: CI の bench.gd の結果を README に記録し、CI の lavapipe で Forward+ が動くか確かめる

## 目的

T0467 で CI の `godot` のジョブに、lavapipe での Forward+ の適合テストと、bench.gd (Compatibility・Forward+) の計測を足した。
手元 (Mesa 26.2.3) では通ったが、CI で流れた後に回したことが 2 つ残っている (T0467 の候補)。

1. CI で流れた bench.gd の結果 (ジョブの要約・成果物 `godot-bench`) を bindings/godot/README.md に記録する (T0467 の受け入れ条件の 1 つめ)。
2. CI のランナー (ubuntu-24.04 の Mesa 24 の lavapipe) で Forward+ が動くか確かめる。動かなければ (実行器が描き方の食い違いで止まる)
   Mesa の版を上げる (PPA など) か、Forward+ の段を手元だけにするなどの対処をする。

push と CI の実行が要る (オーケストレーターか利用者が流す)。

## 受け入れ条件

- [ ] CI の `godot` のジョブで Forward+ の適合テストが通った (通らなければ対処して通した。対処できなければ理由と案をメモに)
- [ ] CI の bench.gd の値 (Compatibility・Forward+ の mean・median・p95、ランナーの情報) を bindings/godot/README.md に記録した
- [ ] `just ci` が通る

## 参考

- T0467 のメモ (起票の候補・確かめていないこと)
- .github/workflows/ci.yml の `godot` のジョブ、bindings/godot/README.md

## メモ

- T0036 で起票した (元の候補: T0467 (2 つ))。
