---
id: T0471
title: "Godot: CI の bench.gd の結果を README に記録し、CI の lavapipe で Forward+ が動くか確かめる"
status: done
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

- [x] CI の `godot` のジョブで Forward+ の適合テストが通った (通らなければ対処して通した。対処できなければ理由と案をメモに)
- [x] CI の bench.gd の値 (Compatibility・Forward+ の mean・median・p95、ランナーの情報) を bindings/godot/README.md に記録した
- [x] `just ci` が通る

## 参考

- T0467 のメモ (起票の候補・確かめていないこと)
- .github/workflows/ci.yml の `godot` のジョブ、bindings/godot/README.md

## メモ

- T0036 で起票した (元の候補: T0467 (2 つ))。

### やったこと (2026-10-06、claude-worker-T0471)

- main の CI ([run 37429758787](https://github.com/rosylilly/ugoe/actions/runs/37429758787)、`376f1cd`) の `godot` のジョブ
  (ID 112157609939、成功) のログと成果物 `godot-bench` を確かめた (成果物はリポジトリの外のスクラッチに置いた)。
- **Forward+ は CI の lavapipe で動いた。** 起票の時点の想定 (Mesa 24) と違い、ランナーの `noble-updates` の Mesa は
  25.2.8-0ubuntu0.24.04.4 (LLVM 20.1.2)。ログに `Vulkan 1.4.318 - Forward+ - Using Device #0: Unknown - llvmpipe` と
  `godot: rendering method forward_plus` が出て (Compatibility に戻っていない)、`drew 69 cases (0 failed, 0 skipped)`・
  `69 of 69 cases passed` (画像の最悪 1/255)。Compatibility も 69 / 69 (最悪 3/255)。対処 (PPA など) は要らなかった。
- bench.gd の値 (Compatibility・Forward+ の mean・median・p95) とランナーの情報 (イメージ 20260927.320、Ubuntu 24.04.5、
  論理プロセッサ 2、Mesa・LLVM・Godot の版、OpenGL / Vulkan の版) を bindings/godot/README.md の「CI の bench.gd の値」に記録した。
  bench の段は失敗せずに流れた (リリースビルドを含めて約 8 分)。
- README の「制限」の、Forward+ を確かめていないという記述を、比べ合わせは Compatibility の Xvfb と Forward+ の lavapipe だけ
  (実際の GPU の Forward+ と Mobile は未確認) に直した。
- .github/workflows は変えていない (直すことが無かった)。

### 確かめていないこと

- 実際の GPU (Vulkan・D3D12・Metal) の Forward+ と Mobile の描き方。
- Mesa が 24 系の環境での Forward+ (CI のランナーは 25.2.8 になっていた)。

### 起票の候補

- なし。

### 検査

- `just ci` が通った。bindings/godot のコードは変えていないので、README の検査 (`just godot-check` など) は流していない。
