---
id: T0190
title: 性能: Puppet の 1 フレームの評価の時間を測る例 (中程度のモデル・数千頂点) を足し、1ms の目標と比べる
status: done
milestone: M7
crates: [ugoe, docs]
depends_on: []
assignee: claude-worker-T0190
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

- [x] `crates/ugoe/examples/puppet_timing.rs` を足した。`cargo run --release -p ugoe --example puppet_timing` で次を測り、1 フレームの時間
      (ほかの処理の揺れを除くため、`physics_timing` と同じく一定のフレームの平均のうち最も速いもの、と中央値) を表で出す
  - 中程度のモデルを模した生成のモデル: 頂点の合計が数千 (目安 3,000〜5,000。数を出力に書く)、メッシュ 100〜200 枚、ワープと回転の
    デフォーマの入れ子、キーフォームの格子 (2〜3 軸)、マスク、標準のパラメータ (角度・目・口・体) に、物理のリグ (静的と動的) と
    目パチ・呼吸・リップシンク。モデルの作り方は例の中に置く (`ugoe-model` の公開 API は増やさない。`compose_timing` の作り方を参考にする)
  - 見本の `assets/samples/avatar.ugoe` (実際のリグの重さの目安として。頂点の数も出す)
  - それぞれ `update` (入力を毎フレーム動かす。トラッキング相当のパラメータの書き込みとモーションの再生を含む) と `follow`
- [x] 今の手元の機械 (CPU の名前・コア数) での結果を docs/testing.md の「性能の計測」の表に足し、1ms の目標を満たすかを書いた。
      満たさなければ、どの段が重いか (物理・変形・頂点など。例の中で段ごとに分けて測るか、プロファイラの結果) をメモに書き、
      改善の候補を「起票の候補」に書いた
- [x] 「中程度の PC」と「数千頂点」をどう読んだか (生成のモデルの大きさと、測った機械) を docs/testing.md に書いた
- [x] `just ci` が通る

## 参考

- docs/roadmap.md の M7 (性能)、docs/testing.md#性能の計測
- crates/ugoe-runtime/examples/compose_timing.rs (中規模のモデルの作り方)、crates/ugoe-physics/examples/physics_timing.rs (測り方)
- crates/ugoe/src/lib.rs (`Puppet::update`)、crates/ugoe/src/follow.rs (`Puppet::follow`)、docs/spec/motion.md#フレーム内の適用順

## メモ

- T0075 で起票した (元: docs/roadmap.md の M7 の「性能: 1 体を 1ms 以内」)。
- 遅くなったら気づける検査は T0191 (この例を使う)。

### 結果 (T0190 の作業)

- `crates/ugoe/examples/puppet_timing.rs` を足した。生成のモデル (メッシュ 144 枚・頂点 4,532・ワープ 14・回転 28・格子 2〜3 軸・
  マスク・標準のパラメータ 21・物理のリグ 10 (静的 3・動的 7)・ループするモーション・目パチ・呼吸・リップシンク) と見本の
  `avatar.ugoe` (メッシュ 24 枚・頂点 1,410・リグ 4・モーション `idle`) で、`update`・`follow`・物理を止めた `update`・評価だけを測る。
  モデルの作り方は例の中に置き、`ugoe-model` のフィクスチャと公開 API は変えていない。生成のモデルは検査 (`validate`) を通し、
  物理が髪・房・スカートのメッシュを実際に動かすことを例の始めで確かめる (計測が空回りしていないこと)。
- 結果 (AMD Ryzen 9 7900・WSL2、load average 6 前後、5 回の中央値): 生成のモデルの `update` 54.5µs・`follow` 53.9µs、
  `avatar.ugoe` の `update` 18.0µs・`follow` 17.8µs。**1ms の目標を満たす** (目標の約 5%)。内訳は評価 (変形と頂点) が約 45µs、
  物理が約 8µs、モーション・効果・ポーズが 1µs ほど。docs/testing.md の「性能の計測」に表と読み方 (中程度の PC・数千頂点) を書き、
  docs/roadmap.md の M7 の行に結果へのリンクを足した。crates/ugoe/README.md に例への案内を足した。
- 確かめていないこと: 中程度の PC (ノート PC など) の実機、wasm (ブラウザ)、ARM。手元の機械の 1 コアの速さから数倍遅くても
  収まると見込んでいるだけ。
- 起票の候補: wasm (ブラウザ) で同じモデルの `update` を測る例か計測ページ (Web の組み込みで 1ms を確かめるため)。
  遅くなったら気づける検査は既存の T0191 (この例の出力をそのまま使える)。
