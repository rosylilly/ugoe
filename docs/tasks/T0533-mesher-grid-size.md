---
id: T0533
title: ugoe-mesher の作業用の格子を中身の範囲に絞り、大きさに上限を設ける
status: done
milestone: M9
crates: [ugoe-mesher, docs]
depends_on: [T0525]
assignee: claude-worker-T0533
---

# T0533: ugoe-mesher の作業用の格子を中身の範囲に絞り、大きさに上限を設ける

## 目的

T0525 のレビューで見つけた所見。`generate` は画像全体 (と `margin` の余白) の格子を作り、1 セルあたり
20 バイトほど (二値のマスク 2 枚・距離変換の f64・深さの f32・輪郭の辺 ID の u32 x 2) を確保する。

- 中身が画像の一部だけでも画像全体の分を確保する。PSD のレイヤーは書き手が外接矩形に切り出すことが多いが
  (`assets/fixtures/psd-real` の 2 つはそう)、キャンバス全体の大きさのレイヤーを書く書き手もある。
  30000x30000 なら 20 GB 近くになる。
- 輪郭の辺 ID は `u32` (`contour.rs` の `2 * (y * w + x)`)。格子が 2^31 セルを超えると ID があふれ、
  空きを表す `u32::MAX` と衝突して誤った輪郭になる (確保が先に失敗することが多いが、黙って誤る経路がある)。

## 受け入れ条件

- [x] 不透明な画素の外接矩形 (と余白) だけの格子で処理する。出力 (頂点・インデックス) は今と同じになることをテストで確かめる
- [x] 格子のセル数に上限を設け、超えたら `MeshError` を返す (新しい変種を足すなら公開 API の変更として扱う)。
      辺 ID があふれないことを保証する
- [x] README の「性能の目安」と「保証」を更新する
- [x] `just ci` が通る

## 参考

- `crates/ugoe-mesher/src/lib.rs` の `generate`、`src/contour.rs` の `extract`

## メモ

### やったこと

- `generate` は先に画像をなめて、アルファが `alpha_threshold` 以上の画素の外接矩形を求め (`Rect::of_content`)、
  その矩形に `margin` + 2 の余白を付けた格子だけを確保する。中身がなければ確保の前に `Empty`。
  格子の座標から画像の座標へのずらしを軸ごと (`[x, y]`) にした (`contour::extract`・`Bounds`・内部点・`triangulate::build`)。
  格子の中の処理 (距離変換・輪郭の走査順・番兵・安全策の走査順) は平行移動で変わらず、座標は整数のずらしなので
  浮動小数点でも同じ値になる。テスト `cropping_to_content_keeps_the_output` で、4 つの形 x 3 つのオプションについて、
  画像全体の格子で処理したとき (`generate_in` に画像全体の矩形を渡す) と出力が完全に一致することを確かめた。
- 格子のセル数の上限 `MAX_GRID_CELLS` (公開の定数、2^28 = 16384 x 16384、約 5 GB) を足し、超えたら確保の前に
  新しい変種 `MeshError::TooLarge { width, height }` (格子の大きさ、u64) を返す。`2 * MAX_GRID_CELLS < u32::MAX` を
  コンパイル時に検査し、`contour::extract` に `debug_assert!` を置いた。大きさの計算は u64 の飽和乗算であふれない (テスト)。
- **公開 API の変更:** `MeshError::TooLarge` と `MAX_GRID_CELLS` を足した。`MeshError` は `non_exhaustive` でないので
  外の網羅的な `match` は壊れるが、リポジトリの中の利用者 (`ugoe-editor-core` の `import.rs`・`automesh.rs`) は
  どれも `_`/`Err(source)` で受けていてそのままビルドが通る。
- README の「アルゴリズム」(1. 二値化)・「保証」・「性能の目安」を更新した。ベンチマーク (`--ignored`) に、
  同じ中身を 4096x4096 の透明なキャンバスに置いた場合を足した (手元で 1024x1024 が約 50〜90ms、4096 のキャンバスでも
  ほぼ同じ。並行するビルドで負荷が高く、ばらつきが大きい)。

### やらなかったこと

- 1 セルあたりの確保 (約 20 バイト) は減らしていない (距離変換の f64 など)。上限で抑えるだけにした。
- `TooLarge` を実際に起こす入力 (外接矩形が 16384 四方を超える画像、1 GB 超) のテストは重すぎるので、
  `grid_size` の単体テストで境界を確かめた。

### 起票の候補

- `ugoe-editor-core` の PSD 取り込み (`import.rs`) は `MeshError::TooLarge` を `Err(source)` で受け、取り込み全体を
  失敗させる。`TooComplex` と同じく警告を出してそのレイヤーだけ飛ばすほうがよいかもしれない (翻訳のキーも要る)。
- `MeshError` に `#[non_exhaustive]` を付けるか (変種を足すたびに公開 API の変更になるのを避ける)。
