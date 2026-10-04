---
id: T0141
title: 検証の漏れ: 数値の有限性と物理の粒子の添字・周波数
status: done
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

- [x] 上の値ごとに「検証エラーにする / ランタイムのフォールバックに任せる」を決め、`docs/spec/model.md#検証` と `#物理の検証`、公開仕様の対応する節に書いた
- [x] エラーにすると決めたものを検証に足し、ランタイムのフォールバックとの食い違いがないことをテストで確かめた
- [x] キーフォームの軸のキー (ブレンドシェイプ格子を含む) と、パラメータの `min`・`default`・`max` を有限値に限るかを決めた (T0145。加えるなら `InvalidValue` などで報告し、テストを足す)
- [x] `validate_physics` が、`gravity`・`wind` の成分、`fps` (0 以下は既定の 60 を使うので許す)、粒子の `position`・`radius`・`delay`・`acceleration`
      (0 以上の有限値)・`mobility` (1 を超えると発散するので 0〜1 にするかを決めて書く)、入力の `weight`・出力の `scale`・`weight`・正規化の範囲の有限性を検査する (T0318)
- [x] 既存のフィクスチャと `assets/compat/` のファイルが検証を通る
- [x] `just ci` が通る

## 参考

- docs/tasks/T0136-review-core-model.md (所見の表)
- docs/tasks/T0137-review-runtime.md・docs/tasks/T0316-review-physics.md のメモ、まとめる前の T0145・T0318
- spec/model.md#data-conventions、spec/model.md#model-validation、spec/evaluation.md#conformance
- docs/spec/model.md#検証、docs/spec/physics.md

## メモ

### 決めたこと (方針は `docs/spec/model.md#数値の検証の方針`)

- モデルと物理の設定の `f32` は、すべて有限値でなければ `InvalidValue` (公開仕様の「`f32` は有限値 (MUST)」に検証を合わせた)。
  正しいファイルには NaN・無限大はないので、読めていたファイルを拒むことにはならない。フィクスチャ・`assets/compat/`・`spec/examples/` は通る。
- 範囲は「範囲の外で定義の計算が壊れるもの」だけをエラーにした: 物理の `mobility` は 0〜1 (1 を超えると発散)、`delay`・`acceleration`・`radius` は 0 以上、
  正規化の範囲は `min <= default <= max` (エディタの `SetPhysicsNormalization` と同じ)。
- ランタイムのフォールバックに任せる (エラーにしない): `fps` が 0 以下 (60)、出力の `particle` が 0 か範囲外 (無視)、粒子 0 個のリグ (何もしない)、
  出力の `weight` の範囲外 (クランプ)、不透明度・描画順・色・回転の拡大率・`uvs`・グルーの値・キャンバスの大きさの範囲 (範囲を定めない)。
  キャンバスの大きさを正に限らないのは、モジュールの検査が大きさ 0 のキャンバスで行うため。
- T0145: キーフォームの軸のキー (ブレンドシェイプ格子を含む) とパラメータの `min`・`default`・`max` を有限値に限った。
  NaN のキーは従来どおり `KeysNotAscending`、無限大のキーは `InvalidValue`。パラメータは 3 つとも有限のときだけ `ParameterRange` を見る
  (NaN の値域は `ParameterRange` から `InvalidValue` に変わった)。

### やったこと

- `validate`: キャンバス・パラメータの値域・キー・パーツ/ワープ/回転/メッシュ/グルーのキーフォームの値・`uvs`・グルーの重み・呼吸の値。
  点の列は有限でない最初の 1 点だけ報告する。`check_grid` に格子の場所を渡すようにし、ブレンドシェイプ格子の `mirror_source`・`inference` の
  値の場所も `….keyforms.blend_shapes[i]` になった (以前は最上位の格子と同じ場所の名前だった)。
- `validate_physics`: `gravity`・`wind`・`fps`・入力の `weight`・出力の `scale`・`weight`・粒子の値・正規化の範囲。
- テスト: `non_finite_model_values_are_reported`・`infinite_keys_are_reported`・`parameter_range_must_be_finite`・`breath_values_must_be_finite`・
  `reports_invalid_numbers`・`runtime_fallbacks_are_not_errors`、ugoe-physics の `values_left_to_runtime_fallbacks_pass_validation_and_stay_finite`
  (検証を通るフォールバックの値で物理を動かし、パラメータが有限のままであることを確かめる)。
- 仕様: `docs/spec/model.md` (方針の節・検証・物理の検証)・`docs/spec/physics.md`・`spec/model.md` (Data conventions・Model validation・
  Physics validation・粒子と正規化の表)・`spec/physics.md` (`mobility` の行)。
- 範囲外 (小さな変更): `ugoe-editor-core` の `SetPhysicsParticles` の検査で `acceleration` も 0 以上にした (検証と同じ。以前は負を通し、
  `Document::apply` の後の `validate_physics` で拒まれるのでメッセージが分かりにくくなるところだった)。テストに 1 行足した。

### やらなかったこと・起票の候補

- アバタープロファイルの数値 (`scale`・`offset`・`range_x` など) はこの検証の対象外のまま (準拠の検査の側)。有限性を準拠の検査で見るかは別タスクの候補。
- モーションと表情は T0323 (この方針にそろえる)。
- エディタの物理の入力欄 (`RigEditor.svelte`) には `min` がなく、負の `delay`・`acceleration` を入れるとコマンドがエラーになる。入力欄に `min=0` を付けるのは起票の候補。
- `ValidationError::InvalidValue` は「最初の 1 つだけ報告する」箇所 (点の列・重み・深度) が増えた。どれがそうかを doc にまとめるのは T0136 の横断の所見のまま。

