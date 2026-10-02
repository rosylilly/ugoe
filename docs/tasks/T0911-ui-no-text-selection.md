---
id: T0911
title: UI: 画面の文字を既定で選択できないようにする (デスクトップアプリらしく)
status: done
milestone: M8
crates: [packages/ui, packages/face-tracking, apps/editor, apps/viewer, apps/studio, docs]
depends_on: [T0909]
assignee: claude-worker-T0911
---

# T0911: 文字の選択を既定で止める

## 目的

利用者の要望。画面の文字 (入力の欄の中を除く) は選択できる必要がなく、ドラッグしたはずみに文字の選択になって操作の邪魔をする。デスクトップアプリらしく、既定で選択できないようにする。

## 受け入れ条件

- [x] @ugoe/ui の土台の CSS で、画面全体の文字を既定で選択できないようにする (`user-select: none`。WebKitGTK・WKWebView 向けの接頭辞も)
- [x] 入力の欄 (input・textarea・contenteditable・@ugoe/ui の NumberField など) の中は、今までどおり選択・コピーできる
- [x] 選択できる方が便利な所だけ、明示の印 (クラスか属性) で選択を許す。例: ライセンスの全文、エラーの詳細、ログ、コピーさせたい URL やトークン (欄で出していないもの)。どこを許したかを docs/ui.md に書く
- [x] 画像やリンクをドラッグしたときに、ブラウザの既定のドラッグ (画像の持ち出し) が起きないようにする (アプリのドラッグ・ドロップは壊さない)
- [x] エディタ・ビューア・スタジオ (操作ウィンドウと出力ウィンドウ) のどれにも効く。スタジオの出力ウィンドウは @ugoe/ui を読まないので、そこにも同じ指定を入れる
- [x] ヘッドレスの Chrome で、ドラッグしても文字が選択されないこと、入力の欄では選択できることを確かめる。`just ci` が通る

## 参考

- docs/ui.md、packages/ui/src/tokens.css

## メモ

- **土台:** `packages/ui/src/tokens.css` の末尾で `:root` に `user-select: none` (と `-webkit-user-select`)。文字の `<input>`
  (button・checkbox・color・file・radio・range・reset・submit の型を除く)・`<textarea>`・`contenteditable`・`.ugoe-selectable` は
  `text` に戻す。`img`・`a` は `-webkit-user-drag: none`、`[draggable="true"]` は `element`。どれも `:where()` で詳細度 0。
  スタジオの出力ウィンドウ (`apps/studio/output.html`) は tokens.css を読まないので、`<style>` に同じ指定 (選択と画像のドラッグ) を書いた。
- **例外 (`.ugoe-selectable`):** 3 つのアプリのライセンスの全文、エディタのモジュールの問題の詳細・マニュアルを開けなかったときの知らせ
  (前から `user-select: text`)・エージェントの接続の設定の例 (同じ)、スタジオの API の URL・仮想カメラの失敗の詳細と入れ方のコマンド。
  一覧は docs/ui.md の「文字の選択」。エディタの 2 か所の `user-select: text` はクラスに置き換えた (接頭辞が無く WebKitGTK で効いていなかった)。
- **確認:** ヘッドレスの Chrome (CDP でマウスのドラッグを送る) で、tokens.css を読んだ試しの頁と読まない頁を比べた。読んだ頁では、文字の上・頁を斜めに越える
  ドラッグで選択が空、input・textarea・contenteditable・`.ugoe-selectable` はドラッグで選択できる、画像とリンクのドラッグは始まらず
  `draggable="true"` の要素は始まる。読まない頁ではどれも選択・ドラッグされる。部品の見本の画面では頁全体のドラッグで選択が空、
  `NumberField` の欄は選択できる。出力ウィンドウ (output.html) も選択が空。記録とスクリーンショットは scratchpad の T0911/。
- **エラーの文 (利用者の指示で追加):** `Notice` と Toast は、エラー (`danger`) のときだけ部品の側で本文を選択できる
  (`Notice` は見出しと本文の `.ugoe-notice__body`、Toast は文)。アプリの失敗の文 (エディタの `.error` のすべて・ツリーのドラッグの失敗・
  アバターの検査のエラーの一覧・着せ替えの問題の詳細、ビューアの `player.error`、スタジオの `.studio__error`・仮想カメラの失敗の文) に
  `.ugoe-selectable` を付けた。ヘッドレスの Chrome で、`danger` の Notice の見出しと本文と Toast の文はドラッグで選択でき、info の Notice と
  success の Toast はできないことを確かめた (scratchpad の T0911/errors.png)。
- **確かめていないこと:** 実際の Tauri のウィンドウ (WebKitGTK・WebView2・WKWebView) での動き。`packages/face-tracking` のエラーの文 2 か所と
  スタジオの `hotkeyError` も選択できるようにした (team-lead が本文に `.ugoe-selectable` を付けた)。
  T0808 のブラウザソースの頁が output.html と別の HTML なら、同じ指定が要る。
- team-lead が review の後に、packages/face-tracking のエラーの文 2 か所 (承認済みの範囲外) とスタジオのホットキーのエラー (warning の Notice) の本文にも .ugoe-selectable を付けた。
