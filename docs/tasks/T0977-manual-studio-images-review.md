---
id: T0977
title: "マニュアル: スタジオの画像に撮った機械の状態 (透過の警告・負荷の数値・ポート・デバイス) が写るのを点検し、撮り方を決める"
status: done
milestone: M10
crates: [tests/e2e, docs]
depends_on: [T0059]
assignee: claude-worker-T0977
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

- [x] 上の画像ごとに、写っている状態がマニュアル (docs/manual/studio.md) の説明の意図に合うかを作業者が決め、決めたことと理由をメモに書いた
      (例: 透過の警告は「警告の出方」の例として残すか、警告の出ない状態で撮るか。負荷の数値は決まった値にするか、写ってよいとするか)
- [x] 合わないものは、撮り方 (tests/e2e/specs/studio.manual.mjs の筋書き、要るなら偽のホストや環境変数) を変えて撮り直したか、
      U06 の手で撮る手順 (docs/manual-check.md) に移して手順を直した。アプリのコードを変える必要があれば、変えずにメモに書いた
- [x] マニュアルの画像の説明 (alt と本文) が撮った画像と合う
- [x] `just e2e --only studio` と `just ci` が通る

## 参考

- T0968 のメモ (スタジオの画像が撮れないこと)、docs/manual-check.md の U06、tests/e2e/README.md#マニュアルの画像
- tests/e2e/specs/studio.manual.mjs、docs/manual/studio.md、T0849 (透過の知らせ)

## メモ

- T0058 で起票した (元の候補: 画像の点検 (studio-output-panel・studio-load・studio-virtual-camera・studio-api・studio-source)、
  T0968 (studio-output.png・studio-virtual-camera.png が撮れていない))。
- 同じ tests/e2e を触る T0059 (ビルドの形) の後にした。
- 画像ごとに決めたこと (T0977):
  - studio-output-panel.png (透過の警告): **残す**。マニュアルの「はじめての配信の準備」の 3 が「透過が効いていません」と
    「背景をクロマキーにする」を説明し、画像の説明もこの注意の例として書いてある。Xvfb (`--manual` の既定) ではいつも出るので
    撮るたびに変わらない。筋書きで注意が出ていることを確かめてから撮るようにした (コンポジタのある画面の `--headed` では失敗する)。
  - studio-load.png (負荷の数値と警告): **数値は写ってよい、注意は写さない** に変えた。数値は測定そのもので、マニュアルも
    「数値は計算機と描く大きさで変わります」と書き足した。一方、注意 (描画が重い・回数が足りない) は Xvfb のソフトウェアの描画の
    遅さから出るもので、GPU で描く読者の画面ではふつう出ず、「重いアプリ」に見えるので写さない。筋書きは注意の無い時を待って撮り、
    撮った後も無いことを確かめる (出ていれば撮り直す。10 秒 × 10 回で失敗)。同じ機械で 2 回とも数秒で撮れた。
    画像の説明 (alt) の「ブラウザソース・仮想カメラの描画の負荷」は画像に無かったので、写っているもの (出力ウィンドウだけ) に直した。
    偽の値で決まった画面にする案は、負荷はバックエンドの指揮役から来るのでアプリのコードに口が要り、採らなかった。
  - studio-api.png・studio-source.png (ポート): **既定のポート 28451 で撮る** に変えた。マニュアルの文が「既定は
    `ws://127.0.0.1:28451/api`」で、ソースの画像は前から既定のポートで撮れていた (API の画像だけ空いているポートを割り当てていた)。
    `--manual` は同じ機械で同時に流せない (ロック) ので、28451 がふさがるのは利用者のスタジオなどが動いているときだけ。
    最初の手順で 28451 が空いていることを確かめ、ふさがっていれば理由を出して止まる。studio-api.png を撮り直した (studio-source.png は
    前から 28451 で、撮り直しても同じ)。
  - studio-virtual-camera.png (`/dev/video10`): **残す**。マニュアルの Linux の手順が `video_nr=10` で読み込むので、読者の画面と同じ。
    U06 に、出す先を `/dev/video10` にすることと、撮った機械のもの (パス・ほかのウィンドウ・壁紙) が写っていないことを確かめる手順を足した。
  - studio-output.png・studio-virtual-camera.png (撮れていない): この WSL2 には今もコンポジタと v4l2loopback が無いので、U06 の手で撮る
    手順のまま (撮り直していない。U06 の結果は L 未)。
- 撮り直した画像 (studio-load.png・studio-api.png) を見て、/home・worktree・mbx のハッシュ・一時フォルダが写っていないことを確かめた。
  ほかのスタジオの画像は撮り直すと描画の揺れ (スクロールバー・トラッキングの値) で数十〜千画素だけ変わるので、前のまま残した。
- 書いた文書: tests/e2e/README.md (マニュアルの画像の「スタジオの画像に出る、撮った機械に依るもの」・流れのポートの文)、
  docs/manual/studio.md (studio-load の alt と数値の文)、docs/manual-check.md (U06)。
- 確かめたこと: `just e2e --manual --only studio`・`just e2e --only studio`・`just ci` が通る (rebase の後)。
- 起票の候補: なし。
