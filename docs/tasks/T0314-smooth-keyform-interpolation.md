---
id: T0314
title: キーフォームの曲線補間 (拡張補間に相当)
status: done
milestone: M3.5
crates: [ugoe-model, ugoe-runtime, ugoe-rigging, ugoe-editor-core, ugoe-format, ugoe-cli, assets, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0314
---

# T0314: キーフォームの曲線補間 (拡張補間に相当)

## 目的

高可動域のモデルでは、角度 X を -30 → 0 → 30 と振ったとき、キーの間を直線で補間すると頬や髪の頂点が円弧でなく直線上を動き、
中間の値で形が痩せたり角ばったりする。Live2D の実践では「拡張補間」でキーの間を曲線にしている (参考の記事 1)。
ugoe の補間は各軸で線形 (`Σ w_i keyform[i]`) なので、同じ見た目にするにはキーを増やすしかない。

## 方針 (案)

軸ごとの重みを、3 つ以上のキーを通る滑らかな曲線 (例: Catmull-Rom、端は片側) の重みに替える。補間後の値は今と同じ **キーフォームの線形結合** (重みは負にもなる) のままなので、
評価の構造・ブレンドシェイプ・推定・変更フラグはそのまま使え、コストもほぼ変わらない。オーバーシュートを避ける方式 (単調な 3 次など) との比較をして決める。

## 受け入れ条件

- [x] 格子 (または軸) ごとの補間の種類 (`linear` 既定 / `smooth`) を足す。古いデータは `linear`。`docs/spec/model.md`・`docs/spec/evaluation.md` と、未知の値のフォールバック
- [x] ランタイムの実装と、次のテスト: キーの上では今と同じ値、キーが 2 つの軸では線形と同じ、3 つのキーが円弧上にあるとき中間の値が直線より円弧に近い、決定性、変更フラグ
- [x] 推定 (加算合成・ミラー・深度からの回転) と、ブレンドシェイプ格子との組み合わせを確かめる (必要なら直す)
- [x] エディタで格子ごとに切り替えられる (コマンドと Undo)。見本 (T0704) の頭の回転で、直線と曲線を並べた絵を目で確かめる
- [x] `just ci`・`just gen`・`just wasm-check` が通る

## 参考

- 乾物ひもの「高可動域モデルの顔の角度 XY」: https://note.com/himono_vtuber/n/na1eb00fbc3b4
- `docs/spec/evaluation.md`

## メモ

- **データ:** `KeyformGrid::interpolation` (`linear` 既定 / `smooth`、未知の値は `linear`)。格子ごと (ブレンドシェイプ格子もそれぞれ)。
  軸ごとにしなかったのは、キーが 2 つの軸は `smooth` でも線形と同じなので、格子ごとで足りるため。追加のフィールドなので `SCHEMA_VERSION` は据え置き。
- **方式:** 3 次エルミートで、接線は隣り合う 3 キーを通る放物線の微分 (内側は Bessel、端は放物線の端)。キーが 3 つなら放物線そのもの。
  円弧のテストで Catmull-Rom (端は片側) と線形と比べた (`docs/spec/evaluation.md#方式の比較-t0314`)。単調な 3 次と求心的 Catmull-Rom は重みが値に依存し、
  キーフォームの線形結合にならないので採らなかった。係数はコンパイル時に求め、毎フレームの確保はない (`update_does_not_reallocate_buffers` に `smooth_arc` を追加)。
- **形以外は線形:** 不透明度・描画順・色・`mask_strength`・反転は線形の重みで補間する (オーバーシュートで描画順が入れ替わる・不透明度が 1 を超えるのを避ける)。
  パーツとグルーの格子はいつも線形。線形の格子は重みの列を 1 つしか作らず、結果は以前とビット単位で同じ (既存のテストがそのまま通る)。
- **エディタ:** `SetKeyformInterpolation { target, blend_shape, interpolation }` (Command の末尾)。インスペクタの「Smooth curve between keys」。
  曲線補間の格子にキーを足すと、新しいセルの形の値を曲線の値で埋める (`KeyformValues::smooth_len` を `ugoe-rigging` に追加)。推定は補間の種類に依存しない (テスト `ugoe-rigging/tests/interpolation.rs`)。
- **目視:** 見本の `FaceWarp`・`BackHairWarp` (深度からの回転、キー -30・0・30) を曲線補間にし、7.5 度ごとのキーで作り直した正解と比べた。
  `AngleX = 15` で頂点のずれの最大が 4.43px → 0.34px、`AngleX = AngleY = 15` で 5.88px → 0.52px。頭の拡大の差分画像でも、線形のずれ (輪郭の二重線) がほぼ消える。
  上の結果を受けて、見本のジェネレータを曲線補間にした (下)。
- **範囲の追加 (team-lead の承認済み):** `ugoe-format` は機能の表 (`FEATURES`) に基準の機能 `deform.smooth_interpolation` (フォールバックは線形) を足すため
  (仕様書の表と `FEATURES` の一致をテストが検査する)。`ugoe-cli` と `assets` は、見本のジェネレータ (`sample.rs` の `depth_rotation`) で
  深度からの回転のワープ `FaceWarp`・`BackHairWarp` を曲線補間にし、`assets/samples/avatar.ugoe` を作り直すため。セルの値は変わらず、補間の種類だけが変わる。
  比べた絵 `assets/samples/avatar-smooth.jpg` (約 130KB) も足した。`avatar-params.jpg` は作り直していない (途中の値の顔がわずかに変わる)。
- **後続の候補:** ブレンドシェイプ格子の切り替えの画面。`avatar-params.jpg` の作り直し。左右の対で補間の種類を揃える操作。
