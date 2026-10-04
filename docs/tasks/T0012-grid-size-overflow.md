---
id: T0012
title: 検証していない格子の大きさと画素の数の積のあふれを塞ぐ (editor-core ほか)
status: done
milestone: M9
crates: [ugoe-editor-core, ugoe-ai, ugoe-cli, ugoe-psd, ugoe-mesher, docs]
depends_on: []
assignee: claude-worker-T0012
---

# T0012: 検証していない格子の大きさと画素の数の積のあふれを塞ぐ (editor-core ほか)

## 目的

M9 のコードレビューの横断の所見 (T0136・T0211・T0526・T0528。まとめは docs/workflow/code-review.md#m9-の結果)。
キーフォーム格子の大きさ (軸のキー数の積) は `ugoe-model` の検証 (T0136) と `ugoe-rigging` (T0526) で飽和させるようにしたが、
次の箇所は検査なしで掛ける。

- `ugoe-editor-core/src/grid.rs` の `stride *= binding.keys.len()` (3 か所)。エディタは読み込んだファイルを検証しないので (T0668 の方針)、
  軸の多い格子では debug でパニックし、release では回り込む
- `ugoe-ai/src/rig.rs`・`ugoe-cli/src/sample.rs` の同じ形 (自分で作った格子だけなら問題ないことを確かめる)
- 画素の数を `u32` のまま `width * height * 4` と掛ける形 (T0528)。`ugoe-psd`・`ugoe-mesher`・エディタのテクスチャの処理に残っていないか

## 受け入れ条件

- [x] 上の箇所を `checked_mul`・`saturating_mul`・`usize` での計算にし、軸の多い格子・大きな画像の失敗するテストを先に書いた
- [x] 自分で作った値だけを掛けるので問題ない箇所は、理由をコメントかメモに書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0136・T0526・T0528 のメモの「横断」

## メモ

### やったこと

- `ugoe-editor-core/src/grid.rs`: セルの線形インデックスを飽和させて求める `linear_index` を足し、`rest_cell_index`・`unedited_cells`・
  `cell_index_in` で使う。`slice_mask` の stride も飽和させる。飽和した値は本当の値以上なので、キーフォーム数との比べ
  (範囲の外か) は正しいまま。`add_key` の stride は `check_count` (`expected_len` は飽和する) を通った後なので、そのままにして理由をコメントに書いた。
  テスト `grids_whose_cell_count_overflows_do_not_panic` (キー 2 つの軸 70 本) は修正の前に debug でパニックすることを確かめた。
- `ugoe-editor-core/src/texture.rs`: PNG の画素のバイト数を `checked_mul` で求める (デコードできた画素なので実際には収まる)。
- `ugoe-ai/src/grid.rs`: `cell_count` (キー数の積を飽和) を足し、`cell_index` のインデックスを飽和させる (`rig.rs` の `keyform_at` は
  範囲の外を `cell … is missing` のエラーにする)。`module_rig.rs` (2 か所)・`module_rig/mesh.rs` のセル数の `.product()` は、
  読み込んだモデルの格子の軸から作るので `cell_count` に替えた (上限 `MAX_FOLLOW_CELLS`・`MAX_MESH_CELLS` との比べは同じ)。
- `ugoe-ai/src/classify/image.rs`: サムネイルのバイト数を `checked_mul(4)` で比べる (`u32::MAX` 四方のレイヤーのテストを足した)。
  なお、このクレートのテストは修正と同じ手で書いたので、修正前に失敗することは理屈 (`(2^32)^2 * 4` があふれる) でしか確かめていない。

### あふれないので理由をコメントに書いた箇所

- `ugoe-cli/src/sample.rs` (`keyform_at` の `find`)・`sample/dress_up.rs` (`grid`): サンプルが自分で作った格子だけを掛ける。
  `sample/paint.rs` は u32 の積をやめて usize で数える形にだけ直した。
- `ugoe-psd/src/parse.rs`: レイヤーは一辺 60000・キャンバスは 30000 までに限り、チャンネルの展開は `Budget` が先に止めるので、
  64 ビットでは収まる (キャンバスの `plane * 4` は 32 ビットでも収まる)。
- `ugoe-ai/src/classify/image.rs` の `composite_rgba`: キャンバスは `ugoe-psd` が 30000 までに限る。
- `ugoe-editor-core/src/atlas.rs`: アトラスは詰めた画像 (デコードできた PNG) を覆う程度の大きさ。
- `ugoe-mesher`: `generate` はすでに u64 で比べ、作業用の格子は `MAX_GRID_CELLS` で止める (T0533)。変更なし。
- `ugoe-editor-core/src/import.rs` の `pad_image`: PSD のレイヤーの大きさ (一辺 60000) に余白を足すだけなので 64 ビットでは収まる。変更なし。

### やらなかったこと

- ツール層 (psd・ai・editor-core) を 32 ビットのターゲットで動かすことは想定していないので、`as usize` の積を 64 ビットの前提で判断した
  (ランタイム層はこのタスクで触っていない)。
- エディタのアプリ (`apps/editor/src-tauri`) は `crates:` の外。grep した範囲では同じ形の積はテストの中だけだった。

### 起票の候補

- なし

