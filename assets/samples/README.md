# 見本のモデル

アプリやビューアで読み込んで動きを確かめるための、完成したモデル (`.ugoe`) を置きます。

| ファイル | 内容 | 大きさ | 出所 / ライセンス |
| --- | --- | --- | --- |
| `avatar.ugoe` | アバタープロファイル ([docs/spec/avatar-profile.md](../../docs/spec/avatar-profile.md)) のレベル 3 に準拠した胸から上のモデル。キャンバス 320x384、パラメータ 26、メッシュ 22 | 約 630KB | `assets/fixtures/psd-real/comfyui-see-through-full-body-parts.psd` (AI 生成、MIT) から `ugoe sample avatar` で生成 / MIT |
| `avatar-params.jpg` | `avatar.ugoe` を各パラメータの端の値で `ugoe render` した顔の拡大の一覧 | 約 115KB | 同上 |

## avatar.ugoe

作り直し (リポジトリのルートで):

```sh
just cli sample avatar -o assets/samples/avatar.ugoe
just cli validate --profile avatar assets/samples/avatar.ugoe   # "claims level 3, satisfies level 3"
```

ジェネレータ (`crates/ugoe-cli/src/sample.rs`) を変えたら作り直してコミットします。
`cargo test -p ugoe-cli` の `committed_sample_is_up_to_date` が、コミットしたものと今の出力が同じ形か確かめます。

### 組み立て方

PSD を取り込み、エディタと同じ `ugoe_editor_core::Command` の列でリグを組みます。部位はレイヤーの名前
(`face`・`eyewhite-l`・`irides-l`・`eyelash-l`・`eyebrow-l`・`mouth`・`front hair`・`back hair` など) で見つけ、
変形の量はその部位の外接矩形の大きさに対する割合で決めます。

| 部位 | デフォーマ / 要素 | パラメータ | 作り方 |
| --- | --- | --- | --- |
| 全体 | 回転 `Root` | なし | 全身の絵を顔を中心に切り詰める平行移動。キャンバスに入らないメッシュ (靴) とテクスチャは消す |
| 体 | 回転 `BodyRotation`・ワープ `BodyWarp` | `BodyAngleX/Y/Z`・`Breath` | 上ほど大きく動かす。斜めのセルは加算合成 |
| 頭 | 回転 `HeadRotation` (あごが中心) | `AngleZ` | ±30 で ±10 度 |
| 顔 | ワープ `FaceWarp` (6x6) | `AngleX/Y` | ドームの深度から **深度からの回転** (角度の倍率 0.7) |
| 後ろ髪 | ワープ `BackHairWarp` | `AngleX/Y` | 奥の平面 (負の一様な深度) を、顔と同じ中心で深度からの回転 |
| 目 | ワープ `EyeLWarp` / `EyeRWarp` | `EyeLOpen`・`EyeLSmile`・`EyeLWide` (右も) | 閉じる・笑い目・見開き。組み合わせは加算合成 |
| 黒目 | メッシュ `irides-l/r` (白目でマスク) | `EyeBallX/Y` | 白目の幅の 22%・高さの 12% だけ平行移動 |
| 眉 | メッシュ `eyebrow-l/r` (前髪の手前に描く) | `BrowLY`・`BrowLAngle`・`BrowInnerUp` (右も) | 上下・外側の上げ下げ・内側の持ち上げ |
| 口 | ワープ `MouthWarp` | `MouthOpenY`・`MouthForm`・`MouthX`・`MouthPucker`・`MouthPress`・`MouthShrug` | 開き・口角・左右・すぼめ・押し付け・口元の持ち上げ |
| 口の中 | メッシュ `MouthInside` | `MouthOpenY` | PSD の唇は閉じた 1 枚なので、唇の合わせ目に暗い楕円 (ジェネレータが描く 32x16 のテクスチャ) を置き、開くと現れる |

`Model.avatar` (メタデータ・許諾・`Head` / `Body` の当たり判定) は、設定するコマンドがまだないので組み終えたモデルに直接書いています。

![各パラメータの端の値](avatar-params.jpg)
