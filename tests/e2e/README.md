# 実機の e2e テスト (`just e2e`)

本物の Tauri のアプリ (エディタ・ビューア・スタジオ。Linux では WebKitGTK) を起動し、WebDriver で操作して主な流れを確かめる (T0006)。
フロントエンドの Vitest とヘッドレスの Chrome での確認では見つからない、WebView と Tauri の間の不具合
(T0812 のトラッキングの固まり: 重い処理で刻みが溜まり `invoke` の応答が何秒も遅れる、など) を捕まえるためのもの。

## 実行

```sh
just e2e                    # = node tests/e2e/run.mjs (3 つのアプリをビルドしてすべて流す)
just e2e --only studio      # スタジオだけ (カンマ区切りで複数)
just e2e --no-build         # ビルドを省き、前に作った target/e2e/bin/ の実行ファイルを使う
just e2e --headed           # Xvfb を使わず、今の画面 (DISPLAY) にウィンドウを出す (手元で様子を見るとき)
just e2e --out DIR          # 結果の置き場 (既定 target/e2e)
```

要るもの (Linux だけ):

| もの | 入れ方 |
| --- | --- |
| tauri-driver | `cargo install tauri-driver --locked` (`TAURI_DRIVER` で場所を指定できる) |
| WebKitWebDriver | Debian・Ubuntu の `webkit2gtk-driver` (`WEBKIT_WEBDRIVER` で場所を指定できる) |
| Xvfb | `xvfb` (`--headed` なら要らない) |
| dbus-run-session | `dbus` |
| アプリのビルドに要るもの | [apps/editor/README.md](../../apps/editor/README.md) (Tauri のシステムの依存・wasm-bindgen-cli など) |
| (あれば) xwd と ImageMagick | 失敗したときに画面全体のスクリーンショットを撮る |

足りないものがあれば、入れ方を出して止まる (終了コード 2)。`just ci` には含めない (道具が要り、アプリのビルドが重いため)。
CI では `.github/workflows/e2e.yml` が流す ([docs/testing.md](../../docs/testing.md#実機の-e2e-テスト))。

## 流れ

1. アプリを `tauri build --debug --no-bundle` でビルドする。フロントエンドを埋め込んだデバッグビルドなので、Vite の開発サーバは要らない
   (配布物と同じ読み込み方)。できた実行ファイルは `target/e2e/bin/` に写す。
2. Xvfb を起動する (空いている番号を選ぶ)。利用者の画面にはウィンドウを出さない。
3. アプリごとに、新しい D-Bus のセッション (`dbus-run-session`) の中で tauri-driver を起動し、`specs/<アプリ>.test.mjs` を
   node:test で流す。tauri-driver は WebKitWebDriver に中継し、WebKitWebDriver がアプリを起動する。
4. 結果をまとめて表示する。どれかが失敗すれば終了コード 1。

### 利用者の環境を汚さない

アプリは次の環境で動く (run.mjs の `appEnv`)。

- `XDG_CONFIG_HOME`・`XDG_DATA_HOME`・`XDG_CACHE_HOME`・`XDG_STATE_HOME` を `target/e2e/<アプリ>/xdg/` に向ける。
  アプリの設定 (エディタの AI の設定・スタジオの前回のプロファイル)・データ (スタジオの API の鍵 `studio-api.json`)・
  WebView の localStorage (言語などの設定) はすべてここに入り、毎回消してから始める。
- `dbus-run-session` で新しいセッションバスを使う。利用者の鍵の保管庫 (Secret Service。エディタの API キー) に触らない。
- 言語は `LANG=en_US.UTF-8` に固定する (OS の言語に合わせる画面を、どの環境でも英語にする)。
- テストが書くファイル (スタジオの見本のプロファイル・エディタの見本のプロジェクト) は `target/e2e/<アプリ>/work/` に置く。
- スタジオの操作 API は空いているポートで開く (利用者のスタジオの既定のポート 28451 とぶつけない)。見本のプロファイルの
  ホットキー (OS 全体に登録される) は外してから開く。

### 結果

```
target/e2e/
  bin/                          ビルドした実行ファイル
  <アプリ>/driver.log            tauri-driver・WebKitWebDriver・アプリの標準出力と標準エラー
  <アプリ>/<アプリ>.console.json  終わったときの、各ウィンドウのページの console (起動の直後から)
  <アプリ>/shots/                途中のスクリーンショット (成功したときも見返せるように)
  <アプリ>/failures/<手順>/       失敗したとき: 各ウィンドウのスクリーンショット・画面全体 (screen.png)・console.json
  studio/tracking.json          重い検出の下でのトラッキングの測定 (set_tracking の回数と応答の時間・AngleX の推移)
```

CI では失敗したときに `target/e2e` (実行ファイルを除く) を成果物として残す。

## 筋書き

| アプリ | 手順 |
| --- | --- |
| エディタ (`specs/editor.test.mjs`) | 起動 → 「Open Sample…」で見本のプロジェクトを書き出して開く → パラメータ (AngleX) の値を入力する → メタデータの Version を変えて保存し、ファイルに書かれ未保存の印が消える → 設定で日本語に切り替え、読み込み直しても日本語のまま |
| ビューア (`specs/viewer.test.mjs`) | 起動 → 「Samples」のメニューから見本を開く → トラッキングの記録を流し、AngleX が動く → 設定を開いて閉じる |
| スタジオ (`specs/studio.test.mjs`) | 起動 → 見本のプロファイルを開く → シーンをクリックで切り替える → 出力ウィンドウを出し、フレームが届く → 重い検出の下でトラッキングが操作ウィンドウと出力に届く (T0812 の回帰) → 出力ウィンドウを隠してもフレームが届き続ける → 操作 API のクライアントを承認し、シーンを切り替える → ブラウザソースのページとフレームが届く → シーンを足すとプロファイルに自動で保存され、開き直しても残る |

画面の操作はできるだけ WebDriver の入力 (クリック・キー) で行う。次のものだけは OS のものなので、テストが代わりをする。

- OS のファイルダイアログ: エディタは `window.__UGOE_FILE_DIALOGS__` (apps/editor/src/lib/fileDialogs.ts) を差し替える。
  スタジオのプロファイルを開くダイアログは、ダイアログが返すはずのパスでバックエンドのコマンド (`open_profile`) を呼ぶ。
- カメラ: 下の「偽のカメラ」。
- 操作 API のポートとトークン・ブラウザソースの URL は、パネルの表示の代わりにバックエンドのコマンドで読む (クリップボードを使わないため)。

### 偽のカメラ

本物のカメラと MediaPipe の代わりに、決まった動き (頭を左右に振り、傾け、口を開け閉めする) の記録を
`window.__UGOE_TRACKER__.startReplay` で再生する (`lib/tracking.mjs`)。MediaPipe のモデルと顔の映像が要らず、値が決まるので判定が安定する。

重い検出は、トラッカーが刻みごとに値を書くところ (`replayTime`。Svelte の `$state` のアクセサ) をこのインスタンスだけ差し替え、
書くたびに主スレッドを 200ms 塞いで模す (WSL2 の WebKitGTK での MediaPipe の顔と体の検出 1 回の重さ)。
刻みの処理の中で重くなるので、T0812 の不具合 (刻みが溜まり `set_tracking` の応答が何秒も遅れる) がそのまま起きる。
スタジオの筋書きは、10 秒の間に `set_tracking` が 10 回以上届き、応答の中央値が 1 秒未満で、操作ウィンドウの AngleX が動くことを確かめる。

## 筋書きを足す

- `specs/<アプリ>.test.mjs` に `step(app, "名前", async () => { ... })` を足す。`step` は失敗したときの記録を残す (`lib/harness.mjs`)。
  1 つのアプリは 1 回だけ起動し、手順は上から順に同じウィンドウで進む (前の手順が失敗すると後の手順も失敗しやすい)。
- 部品は表示の文字 (英語) と `@ugoe/ui` の部品のクラス・ARIA の属性で探す (`lib/page.mjs` の `window.__e2e`)。
  表示の文字を変えたら筋書きも直す。
- 待つときは決まった時間眠らず、`waitFor` で条件を待つ。
- WebDriver のクライアントは `lib/webdriver.mjs` の小さなもの (W3C WebDriver のうち使う分だけ。npm の依存なし)。

## Windows と macOS

流さない。macOS の WKWebView には WebDriver が無く、tauri-driver も対応していない。Windows は tauri-driver が
Microsoft Edge Driver に中継して動くはずだが、確かめていない。両方とも、リリースの前に手で確かめる一覧 ([docs/manual-check.md](../../docs/manual-check.md)) で見る
([docs/testing.md](../../docs/testing.md#実機の-e2e-テスト))。
