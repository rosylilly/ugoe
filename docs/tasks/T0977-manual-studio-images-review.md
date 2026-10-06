---
id: T0977
title: "マニュアル: スタジオの画像に撮った機械の状態 (透過の警告・負荷の数値・ポート・デバイス) が写るのを点検し、撮り方を決める"
status: todo
milestone: M10
crates: [tests/e2e, docs]
depends_on: [T0059]
assignee:
---

# T0977: マニュアル: スタジオの画像に撮った機械の状態 (透過の警告・負荷の数値・ポート・デバイス) が写るのを点検し、撮り方を決める

## 目的

`just e2e --manual` で撮ったスタジオのマニュアルの画像に、撮った機械に依る状態が写っている。

- studio-output-panel.png: コンポジタの無い環境 (WSLg・Xvfb) でだけ出る「透過が効いていません」の警告
- studio-load.png: 撮った機械の遅い描画の数値と、それによる警告
- studio-api.png・studio-source.png: 撮ったときに割り当てたポートの番号
- studio-virtual-camera.png: `/dev/video10` (docs/manual-check.md の U06 で手で撮る)

また、studio-output.png・studio-virtual-camera.png は WSL2 に v4l2loopback もコンポジタも無く、T0968 では撮れていない (U06 で手で撮る)。
これらがマニュアルの意図 (読者が自分の画面で見るものの例) に合うかを決め、合わなければ撮り方を変える。

## 受け入れ条件

- [ ] 上の画像ごとに、写っている状態がマニュアル (docs/manual/studio.md) の説明の意図に合うかを作業者が決め、決めたことと理由をメモに書いた
      (例: 透過の警告は「警告の出方」の例として残すか、警告の出ない状態で撮るか。負荷の数値は決まった値にするか、写ってよいとするか)
- [ ] 合わないものは、撮り方 (tests/e2e/specs/studio.manual.mjs の筋書き、要るなら偽のホストや環境変数) を変えて撮り直したか、
      U06 の手で撮る手順 (docs/manual-check.md) に移して手順を直した。アプリのコードを変える必要があれば、変えずにメモに書いた
- [ ] マニュアルの画像の説明 (alt と本文) が撮った画像と合う
- [ ] `just e2e --only studio` と `just ci` が通る

## 参考

- T0968 のメモ (スタジオの画像が撮れないこと)、docs/manual-check.md の U06、tests/e2e/README.md#マニュアルの画像
- tests/e2e/specs/studio.manual.mjs、docs/manual/studio.md、T0849 (透過の知らせ)

## メモ

- T0058 で起票した (元の候補: 画像の点検 (studio-output-panel・studio-load・studio-virtual-camera・studio-api・studio-source)、
  T0968 (studio-output.png・studio-virtual-camera.png が撮れていない))。
- 同じ tests/e2e を触る T0059 (ビルドの形) の後にした。
