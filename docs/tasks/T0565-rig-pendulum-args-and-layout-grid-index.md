---
id: T0565
title: editor-core の Rig::pendulum の引数をまとめ、ugoe-rigging の Layout の添字の計算を grid_index にそろえるか確かめる
status: done
milestone: M9
crates: [ugoe-editor-core, ugoe-rigging]
depends_on: []
assignee: claude-worker-T0565
---

# T0565: editor-core の Rig::pendulum の引数をまとめ、ugoe-rigging の Layout の添字の計算を grid_index にそろえるか確かめる

## 目的

T0561 で格子の添字の計算を `grid_index` にまとめ、`module_rig` の引数を文脈の構造体にまとめた。次が残っている (T0561 の候補)。

1. editor-core の `Rig::pendulum` は 11 個の引数を取る。振り子の設定の構造体にまとめる (振る舞いは変えない)。
2. `ugoe-rigging` の `Layout` の `stride` による添字の計算 (`additive.rs`・`mirror.rs`) も `grid_index` にそろえるか。内側のループなので、性能を測ってから決める。

## 受け入れ条件

- [x] `Rig::pendulum` の引数が構造体にまとまり、`clippy::too_many_arguments` の許可が要らない。呼ぶ側を直し、振る舞いのテストが今のまま通る
- [x] `Layout` の添字の計算を `grid_index` にそろえた場合の時間を測り (ベンチか、大きな格子の推定の時間)、そろえるかを決めた。数字と判断をメモに書いた
- [x] `just ci` が通る

## 参考

- T0561 のメモ (起票の候補)

## メモ

- T0032 で起票した (元の候補: T0561 (2 つ))。
- やったこと:
  - `Rig::pendulum` は `ugoe-ai` の `rig.rs` にあった (タスクの題の「editor-core」は誤り。`crates:` の外だが、
    `pub(crate)` の関数と 2 つの呼び出し (`rig.rs` の髪の揺れ・`module_rig.rs` の `rig_sway`) だけの小さな変更なので
    行った)。引数を `rig::Pendulum` (ID・表示名・取り付け先・座標系・入力・根元・区間の長さ・`Sway`・`shift`・頂点の
    持ち主・重み) にまとめ、`#[allow(clippy::too_many_arguments)]` を外した。中身は変えていない。`ugoe-ai` の既存の
    テストはそのまま通る。
  - `Layout` の添字: 加算合成の `combine` の `Layout::cell` を `grid_index::linear_index` で求める版 (const generic で
    切り替え) を一時的に作り、同じ実行の中で 2 つを交互に 20 回ずつ走らせて中央値を比べた (release、`combine` を直接、
    `finish` は値の複製だけ。1 軸セルと 2 軸セルの半分が authored)。結果 (stride / grid_index の中央値、2 回の実行):
    | 格子 | 値の数 | stride | grid_index | 比 |
    | --- | --- | --- | --- | --- |
    | 3 軸 × 3 キー (27 セル) | 1 | 2.2 / 1.9 us | 2.6 / 2.0 us | 1.18 / 1.05 |
    | 6 軸 × 3 キー (729 セル) | 1 | 130 / 128 us | 211 / 194 us | 1.62 / 1.51 |
    | 6 軸 × 3 キー (729 セル) | 64 | 291 / 172 us | 371 / 229 us | 1.27 / 1.33 |
    | 8 軸 × 3 キー (6561 セル) | 1 | 4257 / 2493 us | 6295 / 4168 us | 1.48 / 1.67 |
    | 4 軸 × 5 キー (625 セル) | 64 | 133 / 110 us | 170 / 136 us | 1.27 / 1.24 |
    結果は同じ (assert で確かめた)。
  - 判断: **そろえない**。`grid_index` は軸ごとに飽和の積で間隔を求め直すので、部分集合ごとに添字を求める内側の
    ループでは 1.2〜1.7 倍遅い。`Layout` の `stride` は `Layout::new` が積のあふれ (`checked_mul`) を検査した後の値で、
    飽和の要らない場面なので、前もって求めた間隔を使うほうが合う。理由を `Axis::stride` の doc コメントに書いた。
    ミラーの添字 (`mirror.rs`) は目標のセルごとに 1 回だけなので差は小さいが、同じ `stride` の使い方にそろえて残した。
    測った一時のコードはコミットしていない。
- やらなかったこと: ランタイム層は触っていないので `just wasm-check` は流していない。
- 起票の候補: なし。
