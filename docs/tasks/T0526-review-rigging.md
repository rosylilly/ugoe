---
id: T0526
title: コードレビュー: ugoe-rigging
status: done
milestone: M9
crates: [ugoe-rigging, docs]
depends_on: []
assignee: claude-worker-T0526
---

# T0526: コードレビュー: ugoe-rigging

## 目的

ugoe-rigging のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `crates/ugoe-rigging` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 加算合成・ミラー・深度からの回転の推定の境界 (対が無い・親が違う・パラメータが 1 つ)
  - 推定の方法の記録 (T0614) と実際の計算の一致
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0536〜T0540 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 所見

場所は `crates/ugoe-rigging/` からの相対パス。重大度「高」の所見はなかった。

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `src/grid.rs` (`Layout::new`) | 軸のキー数の積 (`stride *= keys.len()` と `KeyformGrid::expected_len`) を検査せずに掛ける。2 キーの軸 64 本 (軸の数の上限内) で debug はあふれでパニック、release は 0 に回って `keyforms` が空なら基準のセルの添字で範囲外 | 直した (`checked_mul`。あふれたら `GridSizeMismatch { expected: usize::MAX }`。テスト `oversized_and_overlong_grids_are_errors`) |
| 中 | `src/normal.rs` (`gaussian_blur`) | カーネルの半径 `ceil(3σ)` に上限がない。有限で巨大な `smoothing` (1e12 など) で `i * i` があふれてパニック、そうでなくても巨大な確保で止まる | 直した (半径を画像の長いほうの辺で打ち切る。テスト `huge_blur_is_bounded_by_the_image`) |
| 中 | `src/normal/group.rs` (`GroupField::build`) | まとまりの格子の大きさに上限がなく、遠い位置や極端に小さい `canvas_per_texel` で巨大な確保になる。`w as u32` は切り捨てうる | T0536 を起票 (`NormalMapError` に種類を足す) |
| 中 | `tests/depth.rs` | 深度からの回転で書いたセルの由来 (`depth_rotation`、T0614) と `InferScope::Own` (別の方法のセルを推定の元にする) のテストがなかった | 直した (`own_scope_records_the_method_and_keeps_other_methods_as_sources`)。計算と記録は仕様どおりだった |
| 低 | `src/normal.rs`・`normal/group.rs` | 公開の `HeightField` (公開のフィールド)・`fill_missing`・`NormalMap::extend_edges` が `values` / `mask` の長さを検査せず、合わなければ添字でパニック。`HeightField::add` は `values` の長さを比べない | T0536 を起票 |
| 低 | `src/grid.rs` (`Layout::new`) | キーの昇順・重複・有限と、同じパラメータの軸の重複を検査せず `validate` に頼る。検証前のモデルでは 1 軸の補間が誤り、重複キーで NaN | T0537 を起票 (`InferError` に種類を足す。上の `usize::MAX` の流用も専用の種類にする) |
| 低 | `tests/` | `ParentCycle`・`UnknownParent`・`TooManyAxes` と、ミラーで元の格子の軸が余る場合のテストがなかった | 直した (`broken_parent_chains_are_errors`・`oversized_and_overlong_grids_are_errors`・`self_mirrored_parameter_needs_the_negated_key` に追加) |
| 低 | `src/normal.rs` (`BulgeOptions::alpha_threshold`) | 0 を 1 とみなすことが doc に書かれていない | 直した (doc) |
| 低 | `src/world.rs` (`WorldError`) | `LengthMismatch`・`NotConverged` に doc コメントがない | 直した |
| 低 | `src/normal.rs` (`gaussian_blur`) | `sigma` が 1e-23 より小さいと `2σ²` が 0 になりカーネルが NaN | 見送り (`HeightField::smooth` が `total > 0` で弾くので、結果はならさない場合と同じ。実害なし) |
| 低 | `src/mirror.rs` (`match_vertices`) | 頂点の対応付けが総当たり (O(n·m)) | 見送り (メッシュの頂点は多くて数千で、明示的な実行と自動の作り直しの 1 回ごとなので問題にならない) |
| 低 | `src/world.rs` (`ancestors_only`) | 親の写像を作るたびにモデル全体を複製してから削る | 見送り (ミラーの 1 回に 2 回だけ。エディタの座標変換で頻繁に呼ぶなら、必要なものだけを写すように変える) |

確かめたこと (所見なし):

- 加算合成の境界: パラメータ (軸) が 1 つ・軸のキーが中立だけ・片側だけに `authored` のセル・基準のセルだけ、は仕様の式どおり (既存のテストもある)。
- ミラーの境界: 対がない (`NotMirrorPair`)・種類やワープの行列が違う (`IncompatibleMirror`)・親が違う (キャンバス座標を経由、`tests/mirror_parents.rs`)・
  軸が 1 対 1 に対応しない・符号を反転したキーがない、はエラーか仕様どおりの結果。「2 本の先の軸が同じ元の軸に対応する」のは `Parameter::mirror` が
  双方向でない (検証を通らない) モデルでだけ起こり、`used` の検査でエラーになる。
- 推定の方法の記録 (T0614) と計算の一致: 加算合成は `additive`、深度からの回転は `depth_rotation`、ミラーは `mirror` を書いたセルにだけ記録し、
  `Own` は `KeyformGrid::derivation` で方法を判定する (方法を記録していない `derived` は格子の `inference`)。書き換えないセルはどの方法でも推定の元になる。
- ブレンドシェイプ (T0137 の補足): `ugoe-rigging` はブレンドシェイプ格子を読まず書かない。仕様 (`docs/spec/keyform-inference.md` の冒頭) も
  「ブレンドシェイプ格子は推定で書き換えず、ミラーもしない」で一致。ミラーはセルの値 (座標・色など) だけを写し、格子の中の ID を書き換える処理を持たない
  (ミラー元の ID は `mirror_source` に書くだけ) ので、T0137 と同じ見落としはない。
- 依存方向 (ツール層。`ugoe-runtime` は親のワールド変換にだけ使う)・`unsafe` なし・ライブラリに `unwrap` / `expect` なし・`TODO` なし。

### 横断 (T0008 向け)

- **キー数の積のあふれ:** 格子の大きさを `Iterator::product` や `*=` で計算する箇所は、検証前の格子で debug はパニック・release は回り込む。
  `ugoe-model` (T0136 で修正)・`ugoe-rigging` (この T0526 で修正) のほかに、`ugoe-runtime` のコンパイル・`ugoe-editor-core` の格子の編集
  (`grid.rs` のキーの追加・削除) にも同じ形がないか見るとよい。
- **画素単位の設定の上限:** ぼかしの `sigma`・ふくらみの `radius` など、画素単位で受け取る有限の値に上限がなく、巨大な値で確保やループが爆発する形。
  `ugoe-ai` の法線マップ・深度の設定や `ugoe-mesher` の間隔の設定にもないか。
- **公開のフィールドを持つ「長さが揃っている前提」の型:** `HeightField` のように、公開のフィールドで作れるのに内部で長さの一致を前提に添字を引く型。

