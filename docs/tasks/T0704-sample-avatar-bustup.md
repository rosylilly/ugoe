---
id: T0704
title: バストアップの PSD から見本のアバターを作り直す
status: in-progress
milestone: M3
crates: [ugoe-cli, assets, docs, apps/viewer]
depends_on: [T0702, T0703]
assignee: claude-worker-T0704
---

# T0704: バストアップの PSD から見本のアバターを作り直す

## 目的

T0702 の見本のモデルは全身の絵を切り出したもので、目・口が 1 枚ずつしかなく、動きが不自然でフェイストラッキング (T0703) の細かい確認ができなかった (利用者の確認、2026-09-29)。
利用者が用意したバストアップの PSD (`assets/fixtures/psd-real/comfyui-see-through-bustup-parts.psd`) は、目・口が開き・閉じの別レイヤーになっている (`eyeclose-l/r`、`mouth-open`・`mouth-close`)。
これらを合成で切り替えて、自然に表情が付く見本のモデルを作る。

## 受け入れ条件

- [ ] `psd-real/README.md` の表にこの PSD の行を足す (ChatGPT で生成した画像を ComfyUI の See-Through でパーツに分けたもの、MIT、利用者の提供。1MB を超える理由も書く)。`real_world` のテストが通る
- [ ] ジェネレータ (`ugoe sample avatar`、T0702) がこの PSD からリグを組む。目は開き (白目・黒目・まつ毛) と閉じ (`eyeclose-*`) を、口は `mouth-close` と `mouth-open` を、パラメータに応じて不透明度と変形で自然につなぐ (途中の値で二重に見えたり消えたりしない)
- [ ] 頭の向き (AngleX/Y/Z)・目の開閉・笑い目・視線・眉・口の開閉・口の形が自然に動く。体の向き (BodyAngle) や呼吸があればなおよい。`ugoe validate --profile avatar` を通る
- [ ] 各パラメータの端と途中の値で描いた一覧の画像を目で確かめ、メモに書く。ビューアで実際の人の顔の画像・動画を入力して、表情が自然に付くことを確かめる
- [ ] ビューアの「サンプル」がこの新しいモデルを開く。前の見本 (全身) を残すかどうかを決めて書く
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0702-sample-avatar.md`、`docs/tasks/T0703-viewer-face-tracking.md` のメモ
- `assets/samples/README.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
