---
id: T0328
title: "見本: アバターにポーズを切り替えるモーションと、振る手を回転デフォーマで揺らす"
status: todo
milestone: M9
crates: [ugoe-cli, assets, docs]
depends_on: []
assignee:
---

# T0328: 見本: アバターにポーズを切り替えるモーションと、振る手を回転デフォーマで揺らす

## 目的

T0474 で、着せ替えの本体にポーズを切り替えるモーション `wave` を入れたが、手は動かない絵のまま表示を切り替えるだけで、
見本のアバター (`avatar.ugoe`) にはポーズを切り替えるモーションが無い (T0474 の候補 2 つ)。見本として、ポーズとデフォーマの動きを組み合わせた例にする。

## 受け入れ条件

- [ ] 着せ替えの本体の振る手 (`HandWave`。袖の `SleeveWave` と `Sailor` の袖も) を回転デフォーマに付け、`wave` で実際に揺れる
      (パラメータを足すなら名前と範囲を assets/samples/README.md の表に書く)
- [ ] 見本のアバターにポーズを切り替えるモーション (例: 耳飾りを外す) を足すか決めて、足したなら `sample::animations::check` を通り、足さないなら理由をメモに書いた
- [ ] テスト (`crates/ugoe-cli/src/sample` の既存の形) で、`wave` の途中で手のメッシュの頂点が動くことと、ポーズの切り替えが今までどおりであることを確かめる
- [ ] `assets/samples/` を作り直し、`committed_sample_is_up_to_date` が通る。assets/samples/README.md・docs/manual/motions.md の見本の説明を直した
- [ ] `just ci` が通る

## 参考

- T0474 のメモ (判断と理由・起票の候補)
- crates/ugoe-cli/src/sample/dress_up.rs・sample/animations.rs、assets/samples/README.md

## メモ

- T0038 で起票した (元の候補: T0474 (2 つ))。
