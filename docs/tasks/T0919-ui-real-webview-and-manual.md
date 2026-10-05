---
id: T0919
title: UI 品質: 実機の WebView での確認とマニュアルの画像の更新
status: todo
milestone: M10
crates: [docs, tests, apps/editor, apps/viewer, apps/studio]
depends_on: [T0918]
assignee:
---

# T0919: UI 品質: 実機の WebView での確認とマニュアルの画像の更新

## 目的

M10 の見直しを、ヘッドレスの Chrome ではなく実機の WebView (WebView2・WKWebView・WebKitGTK) で確かめ、変わった画面に合わせてマニュアルを更新する。
基準は docs/ui.md (T0915 で足す「UI の品質の基準」を含む)。直せないもの・大きいものは後続のタスクにする (milestone は M10)。

## 受け入れ条件

- [ ] e2e (T0006・`just e2e`) で、WebKitGTK の 3 つのアプリの主な画面を撮れるようにし、ヘッドレスの Chrome との差をメモに書いた
- [ ] WebView2 と WKWebView で確かめる手順を docs/manual-check.md に足した (利用者の確認)
- [ ] M10 の各タスクのメモに書かれた「マニュアルの更新の候補」を反映し、docs/manual/images の画像を撮り直した
- [ ] docs/roadmap.md の M10 に完了の判断と後続の候補を書いた
- [ ] `just ci` が通る

## 参考

- docs/ui.md
- docs/manual/README.md
- docs/manual-check.md
- tests/e2e/README.md

## メモ

- T0036 (4 回目の起票のまとめ) から書き足した: T0468 で見本のモデルにパラメータのグループ (`Head`・`Body`・`Eyes`・`Brows`・`Mouth`) が付き、
  パラメータの一覧に区切りが出るようになった。パラメータの一覧を写した docs/manual/images の画像 (ビューア・エディタのプレビュー) を撮り直す (T0468 の候補)。
- マニュアルの更新の候補 (T1622): docs/manual/dress-up.md の本体の問題の表 (「Open base project」の行) と適用のエラーの案内に、
  本体を開いた後のヘッダーの下の帯「Editing the base of module …」と「Return to module」(本体を保存してから押すとモジュールを開き直す。
  設定の画面で拒否された設定は戻った後に適用し直し、また拒否されたら設定の画面に出る) を足す。画面の画像があれば帯を撮り直す
  (docs/editor.md#モジュールに戻る)。
