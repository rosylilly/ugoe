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
2. Xvfb を起動する (空いている番号を選ぶ。下の「Xvfb の番号」)。利用者の画面にはウィンドウを出さない。
3. アプリごとに、新しい D-Bus のセッション (`dbus-run-session`) の中で tauri-driver を起動し、`specs/<アプリ>.test.mjs` を
   node:test で流す。tauri-driver は WebKitWebDriver に中継し、WebKitWebDriver がアプリを起動する。
4. 結果をまとめて表示する。どれかが失敗すれば終了コード 1。

### Xvfb の番号

同じ機械で並行に流しても (並列のエージェントなど) ぶつからないように、run.mjs は使われていない番号を探して Xvfb を起動する (T0022)。

- ロックファイル (`/tmp/.X<n>-lock`)・ソケット (`/tmp/.X11-unix/X<n>`)・抽象ソケット (`@/tmp/.X11-unix/X<n>`。`/proc/net/unix`) の
  どれかがある番号は使わない。抽象ソケットはネットワークの名前空間で共有されるので、`/tmp` を別に持つサンドボックスで動く Xvfb もここに見える。
- 探し始める番号はプロセスごとに変え (99 から 498)、2 つが同じ番号を同時に選んで片方が起動できなければ
  (`SocketCreateListener() failed`・`Server is already active`)、次の番号で試し直す (20 回まで)。
  前は 99 から順に探して試し直さなかったので、並行に流すと同じ番号を選んで片方が起動できなかった (T0820)。
- Xvfb に番号を選ばせる (`-displayfd` だけ) のは使わない。WSLg のように今の画面がロックファイルを作らない環境では `:0` を選び、今の画面とぶつかる。

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
  <アプリ>/<アプリ>.console.json  終わったときの、各ウィンドウのページの console (起動の直後から) と CSP が止めたもの
  <アプリ>/shots/                途中のスクリーンショット (成功したときも見返せるように)
  <アプリ>/failures/<手順>/       失敗したとき: 各ウィンドウのスクリーンショット・画面全体 (screen.png)・console.json
  studio/tracking.json          重い検出の下でのトラッキングの測定 (set_tracking の回数と応答の時間・AngleX の推移)
```

CI では失敗したときに `target/e2e` (実行ファイルを除く) を成果物として残す。

## 筋書き

| アプリ | 手順 |
| --- | --- |
| エディタ (`specs/editor.test.mjs`) | 起動 → 「Open Sample…」で見本のプロジェクトを書き出して開き、ビューポートが描ける → パラメータ (AngleX) の値を入力する → メタデータの Version を変えて保存し、ファイルに書かれ未保存の印が消える → 「Import PSD…」で `assets/fixtures/character.psd` を取り込み、ツリーにレイヤーが出る → 「Export…」で `.ugoe` を書き出し、ファイルができて一時ファイルが残らない → プレビューで見本の `cat-ears.ugoemod` を読み込んで付き、チェックで外し、スロットの選択欄で付け直す → 画像からトラッキングし、MediaPipe が顔を見つけて口の開きが動く → 設定で日本語に切り替え、読み込み直しても日本語のまま → CSP の違反が無い |
| ビューア (`specs/viewer.test.mjs`) | 起動 → 「Samples」のメニューから見本を開く → トラッキングの記録を流し、AngleX が動く → 画像からトラッキングし、口の開きが動く → 設定を開いて閉じる → CSP の違反が無い |
| スタジオ (`specs/studio.test.mjs`) | 起動 → 見本のプロファイルを開く → シーンをクリックで切り替える → 出力ウィンドウを出し、フレームが届く → 重い検出の下でトラッキングが操作ウィンドウと出力に届く (T0812 の回帰) → 出力ウィンドウを隠してもフレームが届き続ける → 画像からトラッキングし、口の開きが動く → 操作 API のクライアントを承認し、シーンを切り替える → ブラウザソースのページとフレームが届く → シーンを足すとプロファイルに自動で保存され、開き直しても残る → CSP の違反が無い |

画面の操作はできるだけ WebDriver の入力 (クリック・キー) で行う。次のものだけは OS のものなので、テストが代わりをする。

- OS のファイルダイアログ: エディタは `window.__UGOE_FILE_DIALOGS__` (apps/editor/src/lib/fileDialogs.ts) を差し替える。
  スタジオのプロファイルを開くダイアログは、ダイアログが返すはずのパスでバックエンドのコマンド (`open_profile`) を呼ぶ。
- OS のファイルダイアログでファイルを選ぶところ (エディタの「Import PSD…」「Export…」・着せ替えの「Load .ugoemod…」): 筋書きが
  `answerDialog` で次のダイアログが返すパスを決めてから、ボタンを押す。
- カメラ: 下の「偽のカメラ」と「画像からのトラッキング」。
- 操作 API のポートとトークン・ブラウザソースの URL は、パネルの表示の代わりにバックエンドのコマンドで読む (クリップボードを使わないため)。

### 偽のカメラ

本物のカメラと MediaPipe の代わりに、決まった動き (頭を左右に振り、傾け、口を開け閉めする) の記録を
`window.__UGOE_TRACKER__.startReplay` で再生する (`lib/tracking.mjs`)。MediaPipe のモデルと顔の映像が要らず、値が決まるので判定が安定する。

重い検出は、トラッカーが刻みごとに値を書くところ (`replayTime`。Svelte の `$state` のアクセサ) をこのインスタンスだけ差し替え、
書くたびに主スレッドを 200ms 塞いで模す (WSL2 の WebKitGTK での MediaPipe の顔と体の検出 1 回の重さ)。
刻みの処理の中で重くなるので、T0812 の不具合 (刻みが溜まり `set_tracking` の応答が何秒も遅れる) がそのまま起きる。
スタジオの筋書きは、10 秒の間に `set_tracking` が 10 回以上届き、応答の中央値が 1 秒未満で、操作ウィンドウの AngleX が動くことを確かめる。

### 画像からのトラッキング

偽のカメラは MediaPipe を通らないので、本物の MediaPipe (アプリに同梱した wasm とモデル。`public/mediapipe/`) を読む手順は別に持つ (T0023)。
3 つのアプリのトラッキングのパネル (@ugoe/face-tracking の TrackingPanel。エディタはプレビュー、スタジオは操作ウィンドウ) の
「File…」のファイルの入力欄に、WebDriver の入力 (Element Send Keys) で顔の画像のパスを渡す。`lib/tracking.mjs` の
`checkImageTracking` が、トラッカーが `running` で入力が画像・顔が見つかる (MediaPipe が読めて検出が動く) ことと、
パネルの口の開き (`MouthOpenY`) のスライダーが 0.15 より大きく動くことを確かめ、トラッキングを止める。

顔の画像は `fixtures/avatar-mouth-open.jpg` (176x168、約 14KB): 見本のアバターの口を開けた顔。`assets/samples/avatar-params.jpg`
(MIT) の「Open=1 Form=-1」の升から `convert assets/samples/avatar-params.jpg -crop 176x168+1+772 +repage -quality 90` で切り出した。
人の写真をリポジトリに入れずに済み、MediaPipe が顔を見つけて `jawOpen` が 0.5 ほどになる (WSL2 の WebKitGTK で検出は 1 回/秒ほど。
MediaPipe を読んで顔を見つけるまで 10〜15 秒)。
MediaPipe の wasm は主スレッドで読むので、CSP が止めれば最後の手順の「CSP の違反」で気づける。

### CSP の違反

3 つのアプリは CSP を持つ (エディタ T0686・ビューア T0727・スタジオ T0820)。CSP が wasm・Worker・blob の画像・IPC などを止めても、
画面の一部が静かに動かなくなるだけでほかの手順では気づけないので、各アプリの筋書きの最後の手順 `has no CSP violations`
(`lib/harness.mjs` の `checkNoCspViolations`) が、すべてのウィンドウで CSP が止めたものが 0 件であることを確かめる (T0022)。

集め方 (`lib/harness.mjs` の `CAPTURE_LOG`。起動の直後と、新しいウィンドウを開いたときの `captureLogs` で入れる):

- `securitypolicyviolation` のイベント。
- `ReportingObserver` の `csp-violation` の報告 (WebView が対応していれば。`buffered` で、入れる前の読み込みの途中のものも受け取る)。
- 文に "Content Security Policy" を含む例外・console の出力。WebKitGTK は wasm のコンパイルを止めても (`'wasm-unsafe-eval'` が無い)
  イベントを出さず、`CompileError` を投げるだけなので、これが要る。
- 最後の手順で、画面に出ている文のうち "Content Security Policy" を含むもの。集める仕掛けを入れる前 (起動の直後) に wasm が止まり、
  アプリがエラーを画面に出すだけのとき (エディタのビューポート) のため。

見逃すもの: 起動の直後に起きて、イベントも報告も画面の文も残らないもの・アプリが握りつぶした例外・Worker の中の違反 (Worker に届く)。
主な機能が動くこと (エディタのビューポートが wasm を読んで描ける、など) は、それぞれの手順でも確かめる。

わざと CSP に止めさせる手順 (スタジオの出力ウィンドウで `new Function` が止まることを確かめる) は、止まったことを
`takeCspViolations` で確かめて取り除く (最後の手順で数えないように)。

`script-src` から `'wasm-unsafe-eval'` を外したビルドで、3 つのアプリとも失敗することを確かめた (T0022)。設定のファイルを書き換えずに
試すには、`{"app":{"security":{"csp":{"script-src":"'self'"}}}}` を書いた JSON を作り、
`pnpm --filter @ugoe/viewer tauri build --debug --no-bundle --config <その JSON>` でビルドして `target/debug/` の実行ファイルを
`<out>/bin/` に写し、`just e2e --no-build --only viewer --out <out>` で流す。

## 筋書きを足す

- `specs/<アプリ>.test.mjs` に `step(app, "名前", async () => { ... })` を足す。`step` は失敗したときの記録を残す (`lib/harness.mjs`)。
  1 つのアプリは 1 回だけ起動し、手順は上から順に同じウィンドウで進む (前の手順が失敗すると後の手順も失敗しやすい)。
  最後の手順は `checkNoCspViolations(app)` のままにする (上の「CSP の違反」)。
- 部品は表示の文字 (英語) と `@ugoe/ui` の部品のクラス・ARIA の属性で探す (`lib/page.mjs` の `window.__e2e`)。
  表示の文字を変えたら筋書きも直す。
- 待つときは決まった時間眠らず、`waitFor` で条件を待つ。
- WebDriver のクライアントは `lib/webdriver.mjs` の小さなもの (W3C WebDriver のうち使う分だけ。npm の依存なし)。

## Windows と macOS

流さない。macOS の WKWebView には WebDriver が無く、tauri-driver も対応していない。Windows は tauri-driver が
Microsoft Edge Driver に中継して動くはずだが、確かめていない。両方とも、リリースの前に手で確かめる一覧 ([docs/manual-check.md](../../docs/manual-check.md)) で見る
([docs/testing.md](../../docs/testing.md#実機の-e2e-テスト))。
