---
id: T0305
title: 動的物理の実装 (2 段階の評価)
status: done
milestone: M3
crates: [ugoe-physics, ugoe-runtime, ugoe]
depends_on: [T0301, T0304]
assignee: claude-worker-T0305
---

# T0305: 動的物理の実装 (2 段階の評価)

## 目的

デフォーマ・頂点への物理の出力、衝突体、重力の基準を実装し、評価を 2 段階にする。

## 受け入れ条件

- [x] 揺れ用パラメータなしで、回転デフォーマの鎖が揺れるテストがある
- [x] 衝突体の外に粒子が押し出されるテストがある
- [x] 基準のデフォーマを傾けると重力の向きが変わるテストがある
- [x] 動的物理を使わないモデルでは評価のコストが変わらない
- [x] 決定性のテストがある
- [x] `docs/spec/evaluation.md` の「実装待ち」を更新する
- [x] `just ci` が通る

## 参考

- `docs/adr/0011-dynamic-physics.md`
- `docs/spec/evaluation.md#2-評価の順序`

## メモ

- やったこと
  - `ugoe-runtime`: 2 段階の評価。`ModelInstance::update_first_stage` (加算量なしでデフォーマだけ評価し、パラメータを控える) の後の `update` が 2 段目で、加算量を受けるデフォーマ・パラメータが変わったデフォーマとその子孫だけを評価し直す。加算量は `DynamicOffsets` (`dynamic_offsets_mut`)。角度は局所の角度に、原点・ワープの制御点・メッシュの頂点はワールド座標に足す。問い合わせ用に `rotation_local_keyform` を追加。
  - `ugoe-physics`: `PhysicsSystem::update_instance` (1 段目 → 根元・衝突体・重力の基準・角度の基準を求めて進める → パラメータと加算量に書く)、`bind`、`is_dynamic`、衝突体 `collider::WorldCollider` (円・カプセル)。`Chain::settle` に押し出しのフックを足し、`chain::rest_offset` を追加。パラメータだけの `update` は従来どおり。
  - `ugoe`: `Puppet::with_physics` で `bind`、`Puppet::update` の物理の段で `update_instance` (T0303 の適用順に合わせた。動的物理でなければ評価は 1 回)。
  - 決めたこと (仕様書に記載): 根元の取り付け先は「最初の `deformer_angle`/`deformer_offset` の出力先 → なければ `frame` → なければキャンバス」、動的物理のリグの物理空間はキャンバスのピクセル。`deformer_angle` の粒子 1 の基準は出力先の親の座標系での重力 (頭を傾けても `frame` なしなら髪は真下に垂れる)、親が裏返っていれば符号を反転。変位 (`deformer_offset`・`vertices`) は静止姿勢からのずれで、キャンバス座標で足す (`source = angle` は無視)。衝突体の半径は取り付け先の拡大・ワープの伸縮を受けない (T0304 の残課題を確認して仕様に明記)。
  - パラメータへの物理の出力は 1 段目の後に書く (ADR 0011 の「1 段目の前」とは順序が違うが、そのパラメータを束縛するデフォーマを 2 段目で評価し直すので結果は同じ。evaluation.md に明記。ADR は変えていない)。
  - コスト: 動的物理を使わない設定では 1 段目を行わず、`ModelInstance` に動的物理の状態を作らない (テストで確認)。`rotation_chain` などの `update()` の時間を変更前後で release で測り、差は揺れの範囲 (60〜207 ns/update)。`dynamic_hair` の 2 段階は 1 フレーム 210 → 270 ns 程度。
  - 仕様書: `docs/spec/physics.md` (動的物理の節、座標と単位)、`docs/spec/evaluation.md` (2 段階の評価の「実装待ち」を解消、`rotation_local_keyform`)、`docs/spec/motion.md` (フレーム内の適用順の物理の段)。
- 範囲外の小さな変更: `ugoe-model` のフィクスチャ `dynamic_hair` / `dynamic_hair_settings` を追加 (既存のフィクスチャは変えていない)。
- やらなかったこと・後続の候補
  - `ugoe-format` の読み込みで `validate_physics` を呼ぶこと (T0304 の残課題。`ugoe-format` は範囲外。参照先が見つからない出力・衝突体は実行時に無視する)。
  - `validate_physics` で、ベクトルの出力先 (`deformer_offset`・`vertices`) の `source = angle` を警告する (今は実行時に無視するだけ)。
  - 衝突体の位置を 1 フレームの中のステップで補間する (今は 1 段目の位置で固定)。衝突体どうし・粒子どうしの衝突。
  - 取り付け先を明示するフィールド (出力先と `frame` からの推定で足りないモデルが出てきたら、データ定義のタスクとして起票)。
  - エディタの動的物理のプレビューと編集 (T0629 など)、wasm / FFI からの `is_dynamic` などの公開。
