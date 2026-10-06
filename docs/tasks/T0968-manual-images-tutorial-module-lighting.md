---
id: T0968
title: "マニュアル: 残りの画像 (チュートリアルの途中・モジュール・点光源・スタジオの出力と仮想カメラ) を撮り直す"
status: todo
milestone: M10
crates: [tests, docs]
depends_on: [T0972, T1650, T1654]
assignee:
---

# T0968: マニュアル: 残りの画像 (チュートリアルの途中・モジュール・点光源・スタジオの出力と仮想カメラ) を撮り直す

## 目的

T0919 で `just e2e --manual` (本物の Tauri のアプリ、WebKitGTK・Xvfb) でマニュアルの画像 40 枚を撮り直したが、筋書きの要る画像は前のまま残った。
今の画面と合わない画像を、同じ仕組みで撮り直す。AI と MCP の画像は T0969 で扱う。

## 受け入れ条件

- [ ] チュートリアルの途中の画像 (`keyform-grid`・`mesh-edit`・`mirror`・`depth-rotation`) を、チュートリアルの手順を筋書きで通して撮った
      (`tests/e2e/specs/editor.manual.mjs`)。手順と画像が合わなければ docs/manual/tutorial.md の文を直した
- [ ] モジュールの作成と比較の画像 (`module-*` のうち T0919 で撮っていないもの。`module-glue.png` は「Pairs」の一覧 (T1648) が写る) と
      `lighting-point.png` を撮った
- [ ] スタジオの `studio-output.png`・`studio-virtual-camera.png`: コンポジタ・v4l2loopback を e2e で用意できれば撮る。用意できなければ、
      撮り方を docs/manual-check.md に手で撮る項目として書いた
- [ ] 撮った画像が `--update-images` で 256 色になって docs/manual/images に入り、マニュアルの文と画像の説明が合っている
- [ ] tests/e2e/README.md#マニュアルの画像 の一覧を直した
- [ ] `just e2e --only editor` (筋書きを足したアプリ) と `just ci` が通る

## 参考

- T0919 のメモ (やらなかったこと・起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」、T1648 のメモ (module-glue.png)
- tests/e2e/README.md#マニュアルの画像、docs/manual/README.md

## メモ

- T0046 で起票した (元の候補: T0919 (残りのマニュアルの画像)・T1648 (module-glue.png の撮り直し))。
- 写る画面を先に直すタスク (T0972 のマニュアルの文・T1650 のグルーの行・T1654 のツリーの札) の後にした。
