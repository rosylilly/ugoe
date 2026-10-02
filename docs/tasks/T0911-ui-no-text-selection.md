---
id: T0911
title: UI: 画面の文字を既定で選択できないようにする (デスクトップアプリらしく)
status: in-progress
milestone: M8
crates: [packages/ui, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0909]
assignee: claude-worker-T0911
---

# T0911: 文字の選択を既定で止める

## 目的

利用者の要望。画面の文字 (入力の欄の中を除く) は選択できる必要がなく、ドラッグしたはずみに文字の選択になって操作の邪魔をする。デスクトップアプリらしく、既定で選択できないようにする。

## 受け入れ条件

- [ ] @ugoe/ui の土台の CSS で、画面全体の文字を既定で選択できないようにする (`user-select: none`。WebKitGTK・WKWebView 向けの接頭辞も)
- [ ] 入力の欄 (input・textarea・contenteditable・@ugoe/ui の NumberField など) の中は、今までどおり選択・コピーできる
- [ ] 選択できる方が便利な所だけ、明示の印 (クラスか属性) で選択を許す。例: ライセンスの全文、エラーの詳細、ログ、コピーさせたい URL やトークン (欄で出していないもの)。どこを許したかを docs/ui.md に書く
- [ ] 画像やリンクをドラッグしたときに、ブラウザの既定のドラッグ (画像の持ち出し) が起きないようにする (アプリのドラッグ・ドロップは壊さない)
- [ ] エディタ・ビューア・スタジオ (操作ウィンドウと出力ウィンドウ) のどれにも効く。スタジオの出力ウィンドウは @ugoe/ui を読まないので、そこにも同じ指定を入れる
- [ ] ヘッドレスの Chrome で、ドラッグしても文字が選択されないこと、入力の欄では選択できることを確かめる。`just ci` が通る

## 参考

- docs/ui.md、packages/ui/src/tokens.css

## メモ
