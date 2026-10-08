---
id: T0190
title: 性能: Puppet の 1 フレームの評価の時間を測る例 (中程度のモデル・数千頂点) を足し、1ms の目標と比べる
status: todo
milestone: M7
crates: [ugoe, docs]
depends_on: []
assignee:
---

# T0190: 性能: Puppet の 1 フレームの評価の時間を測る例を足す

## 目的

M7 の性能の目標「1 体を 1ms 以内 (中程度の PC、数千頂点) で評価する」(docs/roadmap.md の M7) を、測れる形にする。
今ある計測の例 (docs/testing.md#性能の計測) は着せ替えの合成 (`compose_timing`)・カーブ (`curve_timing`)・物理 (`physics_timing`)・
ID の探索 (`id_lookup_timing`) の部分だけで、ホストが毎フレーム呼ぶ `Puppet::update` の全体 (モーション・表情・効果・物理・
ポーズ・デフォーマの変形・頂点) を、目標の大きさのモデルで測るものが無い。

「評価」は `Puppet::update(dt)` の 1 回とする (描画・GPU への転送・ファイルの読み込みは含めない)。受け手の `Puppet::follow` も
スタジオで毎刻み呼ぶので、同じ例で測る。

## 受け入れ条件

- [ ] `crates/ugoe/examples/puppet_timing.rs` を足した。`cargo run --release -p ugoe --example puppet_timing` で次を測り、1 フレームの時間
      (ほかの処理の揺れを除くため、`physics_timing` と同じく一定のフレームの平均のうち最も速いもの、と中央値) を表で出す
  - 中程度のモデルを模した生成のモデル: 頂点の合計が数千 (目安 3,000〜5,000。数を出力に書く)、メッシュ 100〜200 枚、ワープと回転の
    デフォーマの入れ子、キーフォームの格子 (2〜3 軸)、マスク、標準のパラメータ (角度・目・口・体) に、物理のリグ (静的と動的) と
    目パチ・呼吸・リップシンク。モデルの作り方は例の中に置く (`ugoe-model` の公開 API は増やさない。`compose_timing` の作り方を参考にする)
  - 見本の `assets/samples/avatar.ugoe` (実際のリグの重さの目安として。頂点の数も出す)
  - それぞれ `update` (入力を毎フレーム動かす。トラッキング相当のパラメータの書き込みとモーションの再生を含む) と `follow`
- [ ] 今の手元の機械 (CPU の名前・コア数) での結果を docs/testing.md の「性能の計測」の表に足し、1ms の目標を満たすかを書いた。
      満たさなければ、どの段が重いか (物理・変形・頂点など。例の中で段ごとに分けて測るか、プロファイラの結果) をメモに書き、
      改善の候補を「起票の候補」に書いた
- [ ] 「中程度の PC」と「数千頂点」をどう読んだか (生成のモデルの大きさと、測った機械) を docs/testing.md に書いた
- [ ] `just ci` が通る

## 参考

- docs/roadmap.md の M7 (性能)、docs/testing.md#性能の計測
- crates/ugoe-runtime/examples/compose_timing.rs (中規模のモデルの作り方)、crates/ugoe-physics/examples/physics_timing.rs (測り方)
- crates/ugoe/src/lib.rs (`Puppet::update`)、crates/ugoe/src/follow.rs (`Puppet::follow`)、docs/spec/motion.md#フレーム内の適用順

## メモ

- T0075 で起票した (元: docs/roadmap.md の M7 の「性能: 1 体を 1ms 以内」)。
- 遅くなったら気づける検査は T0191 (この例を使う)。
