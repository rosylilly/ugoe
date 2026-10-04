---
id: T0141
title: 検証の漏れ: 数値の有限性と物理の粒子の添字・周波数
status: in-progress
milestone: M9
crates: [ugoe-model, ugoe-physics, docs, spec]
depends_on: [T0136, T0137, T0316]
assignee: claude-worker-T0141
---

# T0141: 検証の漏れ: 数値の有限性と物理の粒子の添字・周波数

## 目的

T0136 のレビューで、構造の検証 (`validate` / `validate_physics`) が見ていない値を見つけた。いまはランタイムが黙って無視するか、
NaN が描画まで伝わる。検証を足すと、これまで読めたファイルが読み込みで拒まれるようになるので、どれをエラーにし、どれを
ランタイムのフォールバックに任せるかを決めてから直す。

見ていない値 (T0136 時点):

- モデル: キーフォームの頂点位置・制御点・不透明度・描画順・色、回転の原点・角度・拡大率、`uvs`、キャンバスの大きさと原点、
  無限大のキー (`[-inf, 0]` は昇順の検査を通り、補間で `inf/inf` = NaN になる)、グルーの `weight` と `intensity`、
  呼吸の `offset` / `peak` / `weight`、パラメータの `min` / `max` の無限大
- 物理: `fps` (0 以下・非有限はランタイムが既定の 60 Hz に置き換える)、出力の `particle` (範囲外はランタイムが無視する)、
  粒子が 0 個のリグ、正規化の範囲 (`min <= default <= max`)、重力・風・粒子の値の有限性、入力と出力の `weight` / `scale`

T0137 (T0145) と T0316 (T0318) のレビューも同じ穴を見つけたので、このタスクにまとめた (T0008)。

- T0145 より: キーフォームの軸のキー (ブレンドシェイプ格子を含む) の無限大は昇順の検査を通り、区間の幅が無限大になって補間の係数が NaN になる
  (NaN の重みは取り除かれ、格子の結果がすべて 0 になる)。値域が無限大のパラメータはクランプが効かない。
- T0318 より: `gravity` が NaN・粒子の `radius` が無限大・出力の `scale` が無限大の設定では、物理はパニックしないが、メッシュの頂点が NaN になって
  レンダラに渡る。入力の `weight` が無限大のリグは、鎖の粒子が NaN のまま戻らない (リセットまで止まる)。JSON の `1e39` は f32 で無限大になる。
- モーションと表情の数値の検証は T0323 で、このタスクで決めた方針に揃えて行う。

## 受け入れ条件

- [ ] 上の値ごとに「検証エラーにする / ランタイムのフォールバックに任せる」を決め、`docs/spec/model.md#検証` と `#物理の検証`、公開仕様の対応する節に書いた
- [ ] エラーにすると決めたものを検証に足し、ランタイムのフォールバックとの食い違いがないことをテストで確かめた
- [ ] キーフォームの軸のキー (ブレンドシェイプ格子を含む) と、パラメータの `min`・`default`・`max` を有限値に限るかを決めた (T0145。加えるなら `InvalidValue` などで報告し、テストを足す)
- [ ] `validate_physics` が、`gravity`・`wind` の成分、`fps` (0 以下は既定の 60 を使うので許す)、粒子の `position`・`radius`・`delay`・`acceleration`
      (0 以上の有限値)・`mobility` (1 を超えると発散するので 0〜1 にするかを決めて書く)、入力の `weight`・出力の `scale`・`weight`・正規化の範囲の有限性を検査する (T0318)
- [ ] 既存のフィクスチャと `assets/compat/` のファイルが検証を通る
- [ ] `just ci` が通る

## 参考

- docs/tasks/T0136-review-core-model.md (所見の表)
- docs/tasks/T0137-review-runtime.md・docs/tasks/T0316-review-physics.md のメモ、まとめる前の T0145・T0318
- spec/model.md#data-conventions、spec/model.md#model-validation、spec/evaluation.md#conformance
- docs/spec/model.md#検証、docs/spec/physics.md

## メモ
