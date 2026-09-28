---
id: T0301
title: 物理演算
status: done
milestone: M3
crates: [ugoe-physics, ugoe-runtime]
depends_on: []
assignee: claude-worker-T0301
---

# T0301: 物理演算

## 目的

docs/spec/physics.md の振り子チェーンを実装する。

## 受け入れ条件

- [x] 固定ステップ、補間、ステップ数の上限が実装されている
- [x] 同じ入力列から同じ出力が得られる (決定性) テストがある
- [x] 係数の意味を確定させ、仕様書の `> 未決:` を解消する
- [x] 物理のフィクスチャを追加する
- [x] `just ci` が通る

## 参考

- `docs/spec/physics.md`

## メモ

- 実装: `PhysicsSystem` (入出力・固定ステップ・上限 8・出力の補間・入力のステップ間補間・最初の update での静止姿勢) と、
  パラメータを知らない振り子チェーンの積分 `chain::Chain` を分けた。`Chain::step` は重力・風を引数に取り、
  距離拘束の直後に位置を補正するフック (衝突体の押し出し用) を持つので、T0305 はこれを使える。
- 係数の意味を `docs/spec/physics.md` で確定した (`> 未決:` を解消)。Cubism を並べて動かすことはできないので、
  公開マニュアルの説明と物理的な解釈で決め、出典の URL を書いた。時間の基準は 1/30 秒、角度は度 (時計回りが正)。
  Cubism の出力角はラジアンなので、設定を移すときは `scale` に π/180 を掛ける (仕様書に記載)。
- 範囲外の小さな変更 (multi-agent.md の「小さな変更」):
  - `ugoe-runtime`: `ParameterValues` に `min` / `max` / `default_value` の読み取りを追加 (入力の正規化と出力のクランプに
    パラメータの値域が必要なため)。オーケストレーターの承認を得て `crates:` に追加した。
  - `ugoe-model`: フィクスチャ `physics_hair` / `physics_hair_settings` を追加。T0304 が `PhysicsRig` などに
    フィールドを足すと、このフィクスチャの構造体リテラルを直す必要がある (後からマージする側が直す)。
- T0304 との合流: T0304 は `PhysicsOutput::parameter` を `target: PhysicsTarget` に置き換え、`output.parameter()`
  (パラメータ以外の出力先では `None`) を用意する。後からマージする側は、`write_outputs` で `output.parameter()` を使い
  `None` の出力を飛ばすこと、フィクスチャとテストの構造体リテラルに新しいフィールドを足すことが必要。
- やらなかったこと: アプリから重力・風を実行時に変える API、入力のしきい値 (微小な揺れの切り捨て)。
- 後続の候補: Cubism の `physics3.json` からの変換 (単位の換算を含む)、エディタの物理プレビュー (T0629)。
