# 見本のモデル

アプリやビューアで読み込んで動きを確かめるための、完成したモデル (`.ugoe`) を置きます。

| ファイル | 内容 | 大きさ | 出所 / ライセンス |
| --- | --- | --- | --- |
| `avatar.ugoe` | アバタープロファイル ([docs/spec/avatar-profile.md](../../docs/spec/avatar-profile.md)) のレベル 3 に準拠した胸から上のモデル。キャンバス 1024x1024、パラメータ 27、メッシュ 24、デフォーマ 12、動的物理のリグ 5 (前髪・後ろ髪・耳飾りの揺れ) | 約 1.3MB | `assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd` (AI 生成、MIT) から `ugoe sample avatar` で生成 / MIT |
| `avatar-params.jpg` | `avatar.ugoe` を各パラメータの端と途中の値 (と表情の組み合わせ) で `ugoe render` した顔の拡大の一覧 | 約 270KB | 同上 |
| `avatar-physics.jpg` | 頭を振る (`AngleX` 0→25→-25→0 を 0.8 秒) モーションと傾ける (`AngleZ` 0→30→0) モーションの途中を `ugoe render --motion --time` で描いたもの (上 4 つが振る、下 4 つが傾ける。0.3/0.5/0.7/0.9 秒) | 約 100KB | 同上 |

T0702 の全身の絵 (`comfyui-see-through-full-body-parts.psd`) から作った前の見本は、目・口が 1 枚ずつで表情が不自然だったので、
T0704 でこのバストアップの PSD から作ったものに置き換えました (全身の見本は残していません。要るなら T0702 のコミットのジェネレータで作れます)。

## avatar.ugoe

作り直し (リポジトリのルートで):

```sh
just cli sample avatar -o assets/samples/avatar.ugoe
just cli validate --profile avatar assets/samples/avatar.ugoe   # "claims level 3, satisfies level 3"
```

ジェネレータ (`crates/ugoe-cli/src/sample.rs`) を変えたら作り直してコミットします。
`cargo test -p ugoe-cli` の `committed_sample_is_up_to_date` が、コミットしたものと今の出力が同じ形 (と同じ物理の設定) か確かめます。

### 組み立て方

PSD を取り込み、エディタと同じ `ugoe_editor_core::Command` の列でリグを組みます。部位はレイヤーの名前
(`face`・`eyewhite-l`・`irides-l`・`eyelash-l`・`eyeclose-l`・`eyebrow-l`・`mouth-open`・`mouth-close`・`front hair`・`back hair` など) で見つけ、
変形の量はその部位の外接矩形の大きさに対する割合で決めます。キャンバスは PSD のまま (切り詰めない) です。

| 部位 | デフォーマ / 要素 | パラメータ | 作り方 |
| --- | --- | --- | --- |
| 体 | 回転 `BodyRotation`・ワープ `BodyWarp` | `BodyAngleX/Y/Z`・`Breath` | 上ほど大きく動かす。斜めのセルは加算合成 |
| 頭 | 回転 `HeadRotation` (あごが中心) | `AngleZ` | ±30 で ±10 度。首 (`neck`) と体は動かない |
| 顔 | ワープ `FaceWarp` (6x6) | `AngleX/Y` | ドームの深度から **深度からの回転** (角度の倍率 0.7) |
| 後ろ髪 | ワープ `BackHairWarp` | `AngleX/Y` | 奥の平面 (負の一様な深度) を、顔と同じ中心で深度からの回転 |
| 開いた目 | ワープ `EyeLWarp` / `EyeRWarp` (白目・黒目・まつげ) | `EyeLOpen`・`EyeLSmile`・`EyeLWide` (右も) | 閉じるにつれ、目の下寄り (高さの 70%) の線へ縦につぶす (0.35 で 22%、0 で 4%)。笑い目は下まぶたを押し上げる (控えめ)。つぶれた目と笑い目の組み合わせのセルは書いた (加算合成では裏返るため) |
| 閉じた目 | ワープ `EyeLCloseWarp` / `EyeRCloseWarp` (`eyeclose-*`) | `EyeLSmile` (右も) | 絵は上向きの弧 (笑った閉じ目)。笑い目 0〜0.5 は中ほどを下げて平らにしたふつうの閉じ目、1 で絵のまま |
| 目の切り替え | メッシュの不透明度 | `EyeLOpen` (右も) | 閉じた目は 0.36→0.3 で現れ、開いた目は 0.3→0.22 で消える。どの値でもどちらかが不透明 (消えない・半透明の 2 枚が重ならない) |
| 黒目 | メッシュ `irides-l/r` (白目でマスク) | `EyeBallX/Y` | 白目の幅の 22%・高さの 12% だけ平行移動 |
| 眉 | メッシュ `eyebrow-l/r` (PSD のとおり前髪の奥) | `BrowLY`・`BrowLAngle`・`BrowInnerUp` (右も) | 上下・外側の上げ下げ・内側の持ち上げ。前髪の手前に描くと髪の上に線が浮いて見えるので奥のまま |
| 口の切り替え | メッシュ `mouth-open` / `mouth-close` の不透明度と形 | `MouthOpenY` | 開いた口は閉じた口の幅・半分の高さの細い隙間から開く (0→0.12 で現れ、0.12→1 で広がる)。閉じた口は 0.12→0.3 で消える |
| 口の形 | 両方の口のメッシュ | `MouthForm` | 口角を閉じた口の高さの 45% だけ上げ下げする (開いた口も同じ量。開きとの組み合わせのセルはすべて書いた) |
| 口のまわり | ワープ `MouthWarp` | `MouthX`・`JawX`・`MouthPucker`・`MouthPress`・`MouthShrug` | 左右・あご (下ほど横)・すぼめ・押し付け・口元の持ち上げ |
| 揺れもの (動的物理) | ワープ `FrontHairSwayWarp` (顔のワープの子)・`BackHairSwayWarp` (後ろ髪のワープの子)・メッシュ `earwear-l/r` | なし (物理の入力は `AngleX`) | 振り子の先の粒子の横の変位を、制御点・頂点に重み付きで足す (`vertices`)。根元は頭の回転デフォーマ (`frame`) に付くので、`AngleZ` と体の動きでも揺れる。前髪は毛先だけ (上 40% は 0、下端で 0.5) を小さく、後ろ髪は顔の中ほどから 2 区間で大きくゆっくり (下端で 0.8)、耳飾りは金具 (上 35%) を動かさず飾りを速く (加速度 4)。衝突体は使わない |

`Model.avatar` (メタデータ・許諾・`Head` / `Body` の当たり判定) は、設定するコマンドがまだないので組み終えたモデルに直接書いています。

![各パラメータの端と途中の値](avatar-params.jpg)

![頭を振る・傾けるときの揺れ](avatar-physics.jpg)
