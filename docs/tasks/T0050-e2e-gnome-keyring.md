---
id: T0050
title: "e2e: D-Bus のセッションで鍵の保管庫 (gnome-keyring) を動かし、AI の API キーの欄と保存を確かめる"
status: done
milestone: M10
crates: [tests, docs]
depends_on: []
assignee: claude-worker-T0050
---

# T0050: e2e: D-Bus のセッションで鍵の保管庫 (gnome-keyring) を動かし、AI の API キーの欄と保存を確かめる

## 目的

`just e2e` は `dbus-run-session` の下でアプリを動かすが、鍵の保管庫 (Secret Service) が無いので、エディタの AI の設定は
「保管庫が無い」の案内になる。API キーの欄・保存・削除を本物の WebView で確かめられず、マニュアルの ai-settings.png も API キーの欄を撮れない (T0919)。

## 受け入れ条件

- [x] `tests/e2e/run.mjs` が、D-Bus のセッションの中で鍵の保管庫 (gnome-keyring-daemon など。空のパスワードで解錠した使い捨ての保管庫) を動かす。
      無い機械では、今と同じく保管庫の要る筋書きを飛ばし、理由を出す (足りない道具の案内に入れ方)
- [x] エディタの e2e に、API キーを保存する → 開き直しても「保存済み」→ 削除する の筋書きがある。保管庫は使い捨てで、利用者の保管庫に触れない
- [x] `just e2e --manual` の ai-settings.png が API キーの欄を写すようになった (撮り直すのは T0969)
- [x] tests/e2e/README.md (要るもの・保管庫の扱い) と CI のジョブ (入れるパッケージ) を直した
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」
- tests/e2e/README.md、tests/e2e/run.mjs、apps/editor/src-tauri の API キーの保存 (keyring)

## メモ

- T0046 で起票した (元の候補: T0919 (e2e の D-Bus で gnome-keyring を動かす))。
- CI (`.github/workflows`) の e2e のジョブにパッケージを足すのは範囲の外の小さな変更として行ってよい。
- やったこと:
  - `tests/e2e/run.mjs`: エディタ (`APPS.editor.keyring`) の `dbus-run-session` の中で、tauri-driver の前に
    `gnome-keyring-daemon --unlock --components=secrets` を動かす。保管庫のファイルは `target/e2e/editor/xdg/data/keyrings/` (毎回消す)。
    `XDG_RUNTIME_DIR` を新しい一時ディレクトリ (`/tmp/ugoe-e2e-*`。終わったら消す) にする。
  - 空のパスワードではない: gnome-keyring 46.1 は空のパスワードではログインの保管庫を作らない (`ReadAlias default` が `/` のまま)。
    決まったパスワード `ugoe-e2e` で作って解錠する (保管庫は使い捨てで、秘密ではない)。
  - `XDG_RUNTIME_DIR` を分けるのは要る: 分けないと gnome-keyring (と D-Bus で起動される `--start` の gnome-keyring) が
    `/run/user/<uid>/keyring` の利用者の保管庫を見つけてそちらにつなぐ (`discover_other_daemon: 1`)。制御のソケットのパスは 108 バイトまでなので
    `target/` の下 (worktree では長い) ではなく `/tmp` の短い一時ディレクトリにした。そのセッションで起動される xdg-document-portal が
    FUSE で `doc` をつなぎ、止まるまで消せないので、消し直しながら少し待つ。
  - gnome-keyring-daemon が無ければ (または `GNOME_KEYRING_DAEMON=` で空にすれば)、`step(..., { skip: keyringSkip() })` で飛ばし、
    理由 (`sudo apt-get install gnome-keyring`) を run.mjs の出力と node:test の skip に出す。両方を手元で確かめた。
  - 筋書き `saves an API key in the keyring, still has it after reopening and deletes it` (既定の Claude): 保存 → 欄が空になりキーが
    ページのどこにも無い → 設定を開き直しても `stored` (画面とバックエンドの `ai_settings`) → 削除の確かめを通して `not_stored`。
  - マニュアルの `ai-settings`: OpenRouter を選んでモデル (`openai/gpt-4o`) を入れて適用し、API キーの欄まで撮る (保管庫が無ければ前のとおり
    「Apply」の行まで)。`--manual --no-build --out target/e2e-manual` で撮れることを確かめた (docs/manual/images には写していない。撮り直しは T0969)。
  - `.github/workflows/e2e.yml` に `gnome-keyring` を足し、`dpkg-query` で入ったことを確かめる (CI で筋書きが黙って飛ばないように)。
    tests/e2e/README.md (要るもの・流れ・「鍵の保管庫」の節・筋書き・マニュアルの画像) と docs/testing.md を直した。
  - 利用者の保管庫 (`~/.local/share/keyrings`) の更新の時刻が変わっていないこと、流した後に gnome-keyring-daemon と一時ディレクトリが残らないことを確かめた。
- 分かったこと: エディタの API キーの保存は、重い負荷の下 (並行のビルドで load average 20 ほど) で 1 回
  「DBus error: Did not receive a reply」で失敗した。dbus-secret-service 4.1 は Secret Service の呼び出しを 2 秒で諦める
  (`proxy/mod.rs` の `Duration::from_millis(2000)`)。筋書きは、この失敗のときだけ知らせを閉じて 1 回やり直す (2 回続けば失敗)。
- 確かめていないこと: CI (ubuntu-24.04) での実行 (e2e.yml は main への push・毎晩・手動で流れる)。
- 調べている途中で、XDG_RUNTIME_DIR を分ける前に `dbus-run-session` の中で D-Bus が起動した gnome-keyring が利用者の保管庫の制御のソケットを
  見つけた (`discover_other_daemon: 1`)。書き込みは無く、利用者の保管庫のファイルは変わっていない。
- 起票の候補:
  - エディタ: Secret Service の呼び出しの 2 秒の上限で、重い負荷の下で API キーの保存・読み出しが失敗する (dbus-secret-service の
    時間の上限を延ばせるか、失敗の後に 1 回やり直すか。apps/editor/src-tauri/src/ai.rs)。
