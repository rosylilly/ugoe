# よくある警告とエラー

メッセージは英語で出ます。出る場所と、意味・対処をまとめます。

| 出る場所 | 内容 |
| --- | --- |
| ヘッダーのボタンの並びの右 (赤い字) | ファイル操作のエラー |
| ヘッダーの「N import warnings」 | PSD の取り込みの警告 (押すと一覧が開く) |
| 右の欄の先頭 (赤い字) | 編集の操作が拒否されたときのエラー。モデルは変わっていない |
| インスペクタの上の枠 | ミラーのやり直しの通知 |
| ビューポートの左上 | ギズモ・フォーム編集の状態とエラー |
| ツリーの下 | ツリーの操作 (ドラッグ・名前・削除) のエラー |

**編集が拒否されたとき、モデルは一切変わりません。** 原因を直してもう一度操作してください。

## ファイル操作

| メッセージ | 意味と対処 |
| --- | --- |
| `Unsaved changes will be lost. Click again to continue.` | 保存していない変更があります。捨ててよければ、同じボタン (「New」「New from PSD…」「Open…」) をもう一度押します。残したければ先に「Save」を押します |
| `…: not a ugoe project (project.json is missing)` | 「Open…」で `.ugoeproj` ではないフォルダを選びました。`project.json` と `model.json` の入った `.ugoeproj` フォルダを選びます |
| `…: directory is not empty and is not a ugoe project` | 「Save As…」で、中身のある別のフォルダを保存先に選びました (上書きしないように拒否します)。新しい名前を付けます |
| `path must be absolute: …` | パスが絶対パスではありません。ふつうはダイアログで選ぶので出ません |
| `the project has not been saved yet; use Save As` | 保存先が決まっていません。「Save As…」で保存先を選びます |

## PSD の取り込みの警告

取り込みは続きますが、見た目が PSD と変わることがあります。警告の先頭の `体 / 服:` のような部分は、警告の出たレイヤーの場所です。

| メッセージ | 意味と対処 |
| --- | --- |
| `the PSD canvas (320x400) differs from the model canvas (1024x1024); the model canvas is kept and the layers are placed at their PSD pixel positions, …` | 「Import PSD…」で、空でないプロジェクトに大きさの違う PSD を取り込みました。キャンバスは変わらず、レイヤーは PSD のピクセルの位置に置かれるので、キャンバスからずれることがあります。新しく始めるなら「New from PSD…」を使います |
| `empty layer skipped` / `fully transparent layer skipped` | 中身のない (透明な) レイヤーは取り込みません。必要なら PSD に絵を描きます |
| `layer too complex to mesh, skipped` | 形が複雑すぎてメッシュを作れませんでした。レイヤーを分けるか、形を単純にします |
| ``blend mode `…` is not supported, using normal`` | 対応していない描画モードのレイヤーを「通常」で取り込みました。使えるのは通常・乗算・覆い焼き (リニア)・加算です |
| ``group blend mode `…` ignored (parts have none)`` | グループの描画モードは使えません (パーツには描画モードがありません)。中のレイヤーに描画モードを設定します |
| `clipping of a group is not supported, imported unclipped` | グループのクリッピングは使えません。マスクなしで取り込みました。インスペクタの「Masks」で設定し直します ([マスク](masks.md)) |
| `clipping base was not imported, imported unclipped` | クリッピングの土台のレイヤーを取り込めなかった (空など) ので、マスクなしで取り込みました |
| `adjustment or fill layer (…) skipped` | 調整レイヤー・塗りつぶしレイヤーは取り込みません。必要なら PSD でラスタライズします |
| `smart object without rendered pixels skipped` | 画素のないスマートオブジェクトは取り込みません。PSD でラスタライズします |
| `layer effects ignored` | レイヤー効果 (ドロップシャドウなど) は取り込みません。効果を除いた絵になります |
| `layer mask ignored` | レイヤーマスクは取り込みません。マスクを掛ける前の絵になります。PSD でマスクを適用してから取り込みます |
| `unsupported channel compression …, layer skipped` | 対応していない圧縮 (ZIP など) のレイヤーです。PSD を保存し直すと直ることがあります |
| `no color channel data, layer skipped` | 色のデータがないレイヤーです |
| `no layer data, imported the composite image` | レイヤーの情報がない PSD です。合成画像を 1 枚のレイヤーとして取り込みました |

## キーと形の編集

| メッセージ | 意味と対処 |
| --- | --- |
| `Not on a key: editing would not know which cell to write.` (ビューポート) / `Between keys.` (インスペクタ) | パラメータの値がキーの間にあるので、どのセルを編集するか決まりません。「Move to nearest key」を押すか、インスペクタのキーの値を押して、キーの上に移ります |
| `Select a mesh or deformer` | 「Form edit」を押しましたが、形を持つ要素を選んでいません |
| `invalid range: min … <= default … <= max … must hold` | パラメータの値域が、最小 ≤ 既定 ≤ 最大 になっていません |
| ``parameter `…` already exists`` | 同じ ID のパラメータがあります。別の ID にします |
| ``… still has children: …`` | 子の残っているパーツ・デフォーマは削除できません。先に子を削除するか、ツリーのドラッグで別の親へ移します |
| `only … vertices would remain (at least 3 are needed)` | メッシュの頂点は 3 つより少なくできません |
| ``mesh `…` is a draw clone of `…`; edit the shape of the original`` | 描画用の複製の形は元に従います。元のメッシュの頂点を編集します ([マスク](masks.md#前後関係が入れ替わる部分の作り方)) |
| `… cannot be reordered: change the draw order instead (…)` (ツリー) | 描画順の数字が違うメッシュどうしは、ドラッグで並べ替えられません。インスペクタの「Draw order」で数字を変えて「Set for all cells」を押します |

## 推定とミラーのエラー

`keyform inference failed:` に続く部分が原因です。

| メッセージ (`failed:` の後) | 意味と対処 |
| --- | --- |
| `axis … has no one-to-one mirrored axis in the other grid` | ミラーの元と先で、軸 (パラメータ) が対応しません。パラメータの対 (`EyeLOpen` ↔ `EyeROpen`) を設定したか ([チュートリアルの 9.1](tutorial.md#91-パラメータの対を設定する))、先の要素にもキーを追加したかを確かめます |
| `source axis … has no key … required by the mirror` | 先のキーに対応するキーが元にありません。自分自身と対にした `AngleX` なら、30 と -30 の両方のキーが要ります |
| `… is not the mirror of …` | 元と先が対になっていません。「Mirror partner」の「Partner」で対にします |
| `… cannot be mirrored from … (different kind or warp size)` | 種類が違うか、ワープの行数・列数が違います。同じ「Rows」「Cols」で作ったワープどうしをミラーします |
| `target vertices [...] have no mirrored source vertex within …` | メッシュの頂点が左右で対応しません。「Tolerance」を大きくするか、頂点の位置をそろえます。ワープで包んでワープどうしをミラーすると、この問題は起きません |
| `Nothing was mirrored: all … cells that … maps to are authored …` (エラーではなく「Mirror」の下に出る) | 書く先のセルがすべて `A` (手で作ったセル。基準のセルを含む) だったので、1 つも書きませんでした。上書きしてよければ、その下の「Overwrite authored cells and mirror」を押します (「Undo」で戻せます) |
| `axis … has no neutral key (default … is not among its keys)` | 軸に既定値のキーがありません。既定値にスライダーを戻し (「Reset」)、「◆」でキーを追加します |
| `base cell … must be authored (or derived by another method)` | 基準のセル (すべて既定値のセル) は推定に戻せません |
| `… has no depth` | 深度からの回転に深度がありません。「Depth」で深度を設定します。`R` のセルが残っている要素の深度を外そうとしたときにも出ます |
| `… is under a warp deformer; depth rotation needs an affine parent space` | 親がワープの要素には深度からの回転を使えません。回転デフォーマの下か最上位の要素に使います |
| `rotation parameter … is not an axis of the grid` | 「Yaw」「Pitch」のパラメータが格子の軸ではありません。先にそのパラメータのキーを追加します |

## ミラーのやり直しの通知

インスペクタの上に、次のような枠が出ることがあります。

```
Could not re-mirror deformer WarpLayer12 from deformer WarpLayer13: .... Its mirrored cells keep their previous values.
Run “Mirror” on the grid to retry, or fix the source.
```

ミラー元 (この例では `WarpLayer13`、左目のワープ) を編集した後、ミラーした要素を自動で作り直そうとして失敗しました。
編集は適用されていますが、ミラーしたセル (`M`) は前の形のままです。

- よくある原因: ミラー元のメッシュの頂点を動かしすぎて、左右の頂点が対応しなくなった。左右の対を外した・付け替えた。
- 対処: 原因を直してから、ミラーした要素を選んで「Mirror」を押し直します。元に戻すなら「Undo」を押します。
- 「Dismiss」で枠を閉じます。
