---
id: T0050
title: "e2e: D-Bus のセッションで鍵の保管庫 (gnome-keyring) を動かし、AI の API キーの欄と保存を確かめる"
status: todo
milestone: M10
crates: [tests, docs]
depends_on: []
assignee:
---

# T0050: e2e: D-Bus のセッションで鍵の保管庫 (gnome-keyring) を動かし、AI の API キーの欄と保存を確かめる

## 目的

`just e2e` は `dbus-run-session` の下でアプリを動かすが、鍵の保管庫 (Secret Service) が無いので、エディタの AI の設定は
「保管庫が無い」の案内になる。API キーの欄・保存・削除を本物の WebView で確かめられず、マニュアルの ai-settings.png も API キーの欄を撮れない (T0919)。

## 受け入れ条件

- [ ] `tests/e2e/run.mjs` が、D-Bus のセッションの中で鍵の保管庫 (gnome-keyring-daemon など。空のパスワードで解錠した使い捨ての保管庫) を動かす。
      無い機械では、今と同じく保管庫の要る筋書きを飛ばし、理由を出す (足りない道具の案内に入れ方)
- [ ] エディタの e2e に、API キーを保存する → 開き直しても「保存済み」→ 削除する の筋書きがある。保管庫は使い捨てで、利用者の保管庫に触れない
- [ ] `just e2e --manual` の ai-settings.png が API キーの欄を写すようになった (撮り直すのは T0969)
- [ ] tests/e2e/README.md (要るもの・保管庫の扱い) と CI のジョブ (入れるパッケージ) を直した
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」
- tests/e2e/README.md、tests/e2e/run.mjs、apps/editor/src-tauri の API キーの保存 (keyring)

## メモ

- T0046 で起票した (元の候補: T0919 (e2e の D-Bus で gnome-keyring を動かす))。
- CI (`.github/workflows`) の e2e のジョブにパッケージを足すのは範囲の外の小さな変更として行ってよい。
