---
id: T0984
title: "マニュアル: スタジオの「入力」のパネルの画像を撮り直し、調整 (感度と境目) を開いた画像を足す"
status: done
milestone: M8
crates: [tests/e2e, docs]
depends_on: []
assignee: claude-worker-T0984
---

# T0984: マニュアル: スタジオの「入力」の画像

## 目的

T0877 で、スタジオの「入力」のパネルに折りたたみの「調整 (感度と境目)」を足した (閉じた見出しが 1 行増えた)。
docs/manual/images/studio-inputs.png は前の画面のままで、撮り直していない。また docs/manual/studio.md#調整-感度と境目 には、
メーター (帯・縦の線・点線) の見方を文だけで書いているので、調整を開いた画像があると分かりやすい。

## 受け入れ条件

- [x] `just e2e --manual` (tests/e2e/specs/studio.manual.mjs の `studio-inputs`) で studio-inputs.png を撮り直した
- [x] 調整を開いて値を既定から動かした状態 (メーターに帯・縦の線・点線が出る) の画像 (例: `studio-inputs-tuning.png`) を足し、
      docs/manual/studio.md#調整-感度と境目 に載せた。画像の説明 (alt) を書いた。撮った機械のもの (デバイスの名前・パス) が写っていないことを確かめた (T0977 の撮り方)
- [x] 画像の減らし方は `just e2e --update-images` と同じ (tests/e2e/README.md#マニュアルの画像)
- [x] `just e2e --only studio` が通る
- [x] `just ci` が通る

## 参考

- T0877 のメモ (やらなかったこと)、T0977・T0980 のメモ (撮り方・写ってよいもの)、tests/e2e/README.md#マニュアルの画像、tests/e2e/specs/studio.manual.mjs

## メモ

- T0083 で起票した (元の候補: T0877 (docs/manual/images/studio-inputs.png を撮り直す。調整を開いた画像を足すか))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T0879・T0880 で調整の項目が増えたら、そのタスクで撮り直す (このタスクは今の画面で撮る)。
- やったこと (claude-worker-T0984):
  - `just e2e --update-images --only studio` で studio-inputs.png を撮り直した (閉じた「調整 (感度と境目)」の見出しが入った)。
  - 筋書き (tests/e2e/specs/studio.manual.mjs) に `studio-inputs-tuning` を足した。記録の再生の首が正面の近く (左右・傾きが 3° 未満) を
    通る時に「キャリブレーション」を押し、`tracker.setTuning` で値を既定から動かす (首の左右の感度 1.5 倍・遊び 3°、傾きの遊び 2°、
    瞼の境目 0.1〜0.7、口の開きの境目 0.1〜0.75)。開いた「調整 (感度と境目)」だけを切り出す (下はすぐ「記録」の行が続くので余白 2px)。
    撮った後は「既定に戻す」を押して閉じる。値はスライダーを操作せずトラッカーに直接入れた (スライダーのつまみの操作を撮る必要は無く、
    フォーカスの輪を写さないため)。
  - docs/manual/studio.md#調整-感度と境目 に画像と alt を載せ、tests/e2e/README.md#マニュアルの画像 に撮り方を書いた。
  - 撮った 2 枚を開いて見た。デバイスの名前は「カメラ (既定)」(記録の再生) と「既定の機器」だけで、パス (ホーム・worktree・mbx・
    一時フォルダ) は写っていない。
  - `--update-images` はスタジオの画像をすべて写すが、ほかの画像 (overview・scenes・load・api-approval) は描画の揺れの差分だけなので
    前のまま戻した (T0977 と同じ)。
- 確かめたこと: `just e2e --manual --only studio`・`just e2e --only studio`・`just ci` が通る (rebase の後)。最初の `--manual` の 1 回は
  起動の手順で見本のプロファイルが読まれずに止まった (機械の負荷による起動の揺れ。同じコードで流し直して通った。この変更の前の手順)。
- やらなかったこと: 記録 (tests/e2e/lib/tracking.mjs の `swingRecording`) は瞼を閉じず首の上下も動かさないので、瞼と上下のメーターは
  帯が出ない (縦の線と点線は出る)。共有の記録は変えなかった。
- 起票の候補: なし。
