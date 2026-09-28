---
id: T0702
title: アバタープロファイルに準拠した見本のモデル
status: done
milestone: M3
crates: [ugoe-cli, assets, docs]
depends_on: [T0110]
assignee: claude-worker-T0702
---

# T0702: アバタープロファイルに準拠した見本のモデル

## 目的

フェイストラッキング (T0306・T0703) で表情が付くことを確かめるための、アバタープロファイルに準拠した見本のモデルを作る。
ARKit のブレンドシェイプから計算するパラメータ (目の開閉・視線・眉・口の開き・口の形・頭の向きなど) が、見て分かる形で絵を動かすこと。

## 受け入れ条件

- [x] `assets/fixtures/character.psd` (または `psd-real/` の PSD) から、`ugoe_editor_core::Command` の列でリグを組むジェネレータ (例: `ugoe sample avatar -o …`) がある。手書きの座標の定数は最小限にし、レイヤーの名前から要素を見つける
- [x] 標準パラメータのうち、最低 `AngleX/Y/Z`・`EyeLOpen`・`EyeROpen`・`EyeBallX/Y`・`BrowLY`・`BrowRY`・`MouthOpenY`・`MouthForm` が絵を動かす。準拠レベル (決めたもの) を満たし、`ugoe validate --profile avatar` を通る
- [x] 生成した `.ugoe` をコミットするか、ビルド時に作るかを決める (コミットするなら大きさと出所を `assets/fixtures/README.md` に書く)
- [x] 各パラメータの端の値で `ugoe render` した絵を目で確かめ、メモに書く (必要なら小さな画像を `docs/` に)
- [x] `just ci` が通る

## 参考

- `docs/spec/avatar-profile.md`
- `docs/manual/tutorial.md` (character.psd のリグの組み方)

## メモ

- **入力の PSD:** `psd-real/comfyui-see-through-full-body-parts.psd` を使った。`character.psd` には眉がなく、目も 1 枚
  (白目・黒目・まつげが分かれていない) なので、`BrowLY/RY` と `EyeBallX/Y` を見て分かる形で動かせない。
  AI の PSD は `eyewhite`・`irides`・`eyelash`・`eyebrow`・`mouth` が左右別に分かれ、顔の肌は目と口を塗りつぶした絵
  (目を閉じると肌が見える) なので、顔の表情の見本に向く。全身の絵なので、回転デフォーマ `Root` の平行移動と
  `SetCanvas` で顔を中心に胸から上 (320x384) へ切り詰め、キャンバスに入らない靴のメッシュとテクスチャは消した。
- **ジェネレータ:** `ugoe sample avatar -o <.ugoe> [--psd] [--project]` (`crates/ugoe-cli/src/sample.rs`)。
  部位はレイヤー名で探し、変形の量は部位の外接矩形の大きさに対する割合で決める (座標の定数なし)。
  セルの形はキャンバス座標でのずれとして書き、`ugoe_editor_core::space` で親の座標系に直して `SetKeyformCell` で入れる。
  使った推定: 顔と後ろ髪の `AngleX/Y` は深度からの回転 (顔はドーム、後ろ髪は負の一様な深度で逆に動く)、
  それ以外の斜めのセルは加算合成。左右の目は絵が左右対称でない (目の位置と大きさが違う) ので、`MirrorKeyforms`
  (絶対位置を反転する) ではなく、同じ関数をそれぞれの目の外接矩形に当てて作った。
- **準拠レベル:** 3 (`ugoe validate --profile avatar` で `claims level 3, satisfies level 3`)。レベル 2 のすべてと、拡張パラメータ
  11 個 (`EyeL/RWide`・`EyeL/RSmile`・`BrowInnerUp`・`BrowL/RAngle`・`MouthX`・`MouthPucker`・`MouthPress`・`MouthShrug`) が絵を動かす。
  `JawX`・`CheekPuff`・`TongueOut`・`NoseSneer` などは作っていない (頬・舌・あごの絵がない)。
- **口の開き:** PSD の唇は閉じた 1 枚なので、唇を縦に伸ばすだけでは開いて見えない。ジェネレータが描いた暗い楕円
  (32x16 の PNG) を唇の合わせ目に置き、`MouthOpenY` で不透明度 0→1 にした。これだけは PSD にない絵。
- **コミットか生成か:** コミットした (`assets/samples/avatar.ugoe`、約 630KB、MIT)。ビューア (T0703) は生成なしで開ける。
  出所と作り直し方は `assets/samples/README.md` (`assets/fixtures/README.md` からも参照)。
  テスト `committed_sample_is_up_to_date` がコミットしたものと今の出力の一致 (ID・プロファイル・評価した頂点の差 0.01px 未満) を検査する。
- **目での確認:** 各パラメータの端の値で `ugoe render` (llvmpipe) した顔の拡大を `assets/samples/avatar-params.jpg` に置いた。
  - `AngleX=±30`: 顔が画面の右/左へ回り込み、反対側の後ろ髪が見える。`AngleY=+30` で目鼻が上がり (見上げ)、`-30` で頭頂が見える (うつむき)。
    倍率 1.0 では顔の中身が大きくずれすぎたので `angle_scale` を 0.7 にした。`AngleZ=±30` で頭が ±10 度傾く (首は動かない)。
  - `EyeLOpen=0` で画面の右の目 (キャラクターの左) だけが下向きの細い線になって閉じる。`EyeLSmile=1` は上向きの弧の細い目。`EyeLWide=1` は上に見開く。
  - `EyeBallX=1` で両目の黒目が画面の右へ、`EyeBallY=1` で上へ動く (白目のマスクで切り抜く)。
  - 眉は前髪の手前に描く (前髪に隠れて動きが見えないため)。`BrowLY/RY=±1` で上下、`BrowL/RAngle` で外側の上げ下げ、`BrowInnerUp` で困り眉。
  - `MouthOpenY=1` で唇の間に口の中が見える。`MouthForm=±1` で口角が上がる/下がる。`MouthX`・`MouthPucker`・`MouthPress`・`MouthShrug` も形が変わる。
  - `BodyAngleX/Y/Z`・`Breath` は上半身と頭がわずかに傾く・上下する (控えめ)。
- **自動テスト:** `sample::tests` がモデルを生成し、構造の検証とプロファイルの検査 (エラーなし・レベル 3) を通ること、
  すべてのパラメータの最小・最大で、あるメッシュの頂点が 1px を超えて動くか不透明度が 0.5 を超えて変わること、
  `EyeLOpen` が左目だけを閉じることを確かめる。
- **コマンドを通さない部分:** `Model.avatar` (メタデータ・許諾・`Head`/`Body`) と `Model.hit_areas` は、設定する `Command` がまだないので、
  組み終えたプロジェクトのモデルに直接書いた。コマンドは T0631 (`SetAvatarProfile` とその画面) で足す。コードには `TODO(T0631)` を付けた。
- **後続の候補:** 物理 (髪揺れ) とアイドルのモーションを付ける。`character.psd` に眉と黒目のレイヤーを足して、小さい見本も作る。
