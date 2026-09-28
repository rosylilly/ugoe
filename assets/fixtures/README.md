# テスト用アセット

コードで組み立てられないテスト入力 (PSD や画像など) だけを置きます。

- 1 ファイル 1MB 以下 (`psd-real/` は理由があれば例外を認める。`psd-real/README.md`)
- 追加したら、下の表に出所とライセンスを書く

| ファイル | 内容 | 出所 / ライセンス |
| --- | --- | --- |
| `layers.psd` | 64x48 の 8 ビット RGB の PSD。取り込みの構造を試す小さなもので、画素は位置から色を決める模様 (虹色に見える)。入れ子のグループ (`体` / `顔`)、非表示のレイヤー (`目`)、不透明度 50% のレイヤー (`メモ`、キャンバスの外にはみ出す負の位置)、クリッピングされたレイヤー (`影`、塗りの不透明度 80%、乗算)、無圧縮と PackBits のチャンネル、何も変えないレベル補正の調整レイヤー (`レベル補正`) を含む。合成画像はレイヤーから計算したもの (白の上に重ねた RGB) | ugoe の自作 (`crates/ugoe-psd/tests/common/fixtures.rs` の定義を同じディレクトリのライターで書き出す) / MIT |
| `character.psd` | 320x400 の 8 ビット RGB の PSD。エディタに取り込んで目で確かめるための、単色の図形で描いた簡単なキャラクター。下から `背景`・`後ろ髪`・グループ `体` (通常: `首`・`服`)・グループ `頭` (通過: `顔`・`顔の影` (顔でクリッピング、乗算)・`頬` (不透明度 50%)・グループ `目` (`右目`・`左目`)・`口`・`口 (開き)` (非表示)・`前髪`)。合成画像はレイヤーから計算したもの | ugoe の自作 (同上。図形は `tests/common/shapes.rs` で描く) / MIT |
| `psd-real/` | 実際のペイントツールや生成ツールが書き出した PSD を置く場所。置いたファイルは自動でテストされる。追加の手順とライセンスの記録は `psd-real/README.md` | 各ファイルの行を `psd-real/README.md` に書く |

見本のモデル `assets/samples/avatar.ugoe` (`psd-real/` の PSD から `ugoe sample avatar` で作る。約 1.3MB、MIT) の
出所と作り方は `assets/samples/README.md` に書く。

`layers.psd` と `character.psd` は Adobe の "Adobe Photoshop File Formats Specification" に沿って書き出し、
ImageMagick 6.9 で読めること (レイヤーの位置と大きさ、合成画像の画素) をテストで確かめている
(`crates/ugoe-psd/tests/fixtures.rs`)。どちらも Photoshop で開けることを確かめた (T0619)。
作り直すときは `UGOE_UPDATE_FIXTURES=1 cargo test -p ugoe-psd --test fixtures`。
