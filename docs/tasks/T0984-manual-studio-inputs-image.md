---
id: T0984
title: "マニュアル: スタジオの「入力」のパネルの画像を撮り直し、調整 (感度と境目) を開いた画像を足す"
status: todo
milestone: M8
crates: [tests/e2e, docs]
depends_on: []
assignee:
---

# T0984: マニュアル: スタジオの「入力」の画像

## 目的

T0877 で、スタジオの「入力」のパネルに折りたたみの「調整 (感度と境目)」を足した (閉じた見出しが 1 行増えた)。
docs/manual/images/studio-inputs.png は前の画面のままで、撮り直していない。また docs/manual/studio.md#調整-感度と境目 には、
メーター (帯・縦の線・点線) の見方を文だけで書いているので、調整を開いた画像があると分かりやすい。

## 受け入れ条件

- [ ] `just e2e --manual` (tests/e2e/specs/studio.manual.mjs の `studio-inputs`) で studio-inputs.png を撮り直した
- [ ] 調整を開いて値を既定から動かした状態 (メーターに帯・縦の線・点線が出る) の画像 (例: `studio-inputs-tuning.png`) を足し、
      docs/manual/studio.md#調整-感度と境目 に載せた。画像の説明 (alt) を書いた。撮った機械のもの (デバイスの名前・パス) が写っていないことを確かめた (T0977 の撮り方)
- [ ] 画像の減らし方は `just e2e --update-images` と同じ (tests/e2e/README.md#マニュアルの画像)
- [ ] `just e2e --only studio` が通る
- [ ] `just ci` が通る

## 参考

- T0877 のメモ (やらなかったこと)、T0977・T0980 のメモ (撮り方・写ってよいもの)、tests/e2e/README.md#マニュアルの画像、tests/e2e/specs/studio.manual.mjs

## メモ

- T0083 で起票した (元の候補: T0877 (docs/manual/images/studio-inputs.png を撮り直す。調整を開いた画像を足すか))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T0879・T0880 で調整の項目が増えたら、そのタスクで撮り直す (このタスクは今の画面で撮る)。
