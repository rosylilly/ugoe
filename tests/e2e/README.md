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
just e2e --manual           # 筋書きの代わりに、マニュアルの画像を撮る (下の「マニュアルの画像」)
just e2e --update-images    # 撮った画像の色を減らして docs/manual/images に写す (--manual を含む)
UGOE_E2E_TRACKING_MS=60000 just e2e --only studio --no-build  # 重い検出のトラッキングを 60 秒流す (下の「長く流す」)
```

要るもの (Linux だけ):

| もの | 入れ方 |
| --- | --- |
| tauri-driver | `cargo install tauri-driver --locked` (`TAURI_DRIVER` で場所を指定できる) |
| WebKitWebDriver | Debian・Ubuntu の `webkit2gtk-driver` (`WEBKIT_WEBDRIVER` で場所を指定できる) |
| Xvfb | `xvfb` (`--headed` なら要らない) |
| dbus-run-session | `dbus` |
| (あれば) gnome-keyring-daemon | `gnome-keyring` (`GNOME_KEYRING_DAEMON` で場所を指定できる。空にすると使わない)。無ければ鍵の保管庫の要る筋書き (エディタの API キー) を飛ばし、理由を出す (下の「鍵の保管庫」) |
| アプリのビルドに要るもの | [apps/editor/README.md](../../apps/editor/README.md) (Tauri のシステムの依存・wasm-bindgen-cli など) |
| (あれば) xwd と ImageMagick | 失敗したときに画面全体のスクリーンショットを撮る |

足りないものがあれば、入れ方を出して止まる (終了コード 2)。gnome-keyring-daemon だけは無くても流せる (飛ばした筋書きは node:test の skip になる)。`just ci` には含めない (道具が要り、アプリのビルドが重いため)。
CI では `.github/workflows/e2e.yml` が流す ([docs/testing.md](../../docs/testing.md#実機の-e2e-テスト))。

## 流れ

1. アプリをビルドする。`tauri build --debug --no-bundle` と同じもの (フロントエンドを埋め込んだデバッグビルド。Vite の開発サーバは要らず、
   配布物と同じ読み込み方) を、アプリごとの `tauri build` ではなく次の 2 段で作る (T0059):
   1. 選んだアプリごとに、`tauri.conf.json` の `beforeBuildCommand` (`pnpm wasm && pnpm build`) をアプリのディレクトリで流し、
      wasm の生成物と `dist/` (frontendDist) を作る。
   2. 選んだアプリを 1 回の `cargo build --bins --features tauri/custom-protocol -p ugoe-editor -p ugoe-mcp -p ugoe-viewer -p ugoe-studio`
      でビルドする。エディタは同梱の `ugoe-mcp` を隣に探す (「Connect agents」の登録のコマンドと Claude Desktop の `.mcpb`) ので、
      それも同じ cargo で作る。

   できた実行ファイルは `target/e2e/bin/` に写す。1 回の cargo にするのは、feature の統合を CI の `rust (apps)` と揃えて、
   e2e.yml がそのキャッシュを読むだけで済むようにするため ([docs/testing.md](../../docs/testing.md#ci-のキャッシュ))。
   cargo の引数は tauri-cli (2.12.0) が `tauri build` の中で流すもの (アプリの `src-tauri` で `cargo build --bins --features tauri/custom-protocol`)
   と同じ。`custom-protocol` が frontendDist を実行ファイルに埋め込み、`tauri.conf.json` は tauri-build と `generate_context!` が
   コンパイルのときに読む。tauri-cli を上げたら、`pnpm --filter @ugoe/viewer tauri build --debug --no-bundle -v` の出力で
   cargo の引数が変わっていないかを確かめる (確かめ方は T0059 のメモ)。
2. Xvfb を起動する (空いている番号を選ぶ。下の「Xvfb の番号」)。利用者の画面にはウィンドウを出さない。
3. アプリごとに、新しい D-Bus のセッション (`dbus-run-session`) の中で tauri-driver を起動し、`specs/<アプリ>.test.mjs` を
   node:test で流す。エディタのセッションでは、先に使い捨ての鍵の保管庫を動かす (下の「鍵の保管庫」)。tauri-driver は WebKitWebDriver に中継し、WebKitWebDriver がアプリを起動する。
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
  エディタの API キーは、そのセッションで動かす使い捨ての保管庫に入る (下の「鍵の保管庫」)。
- 言語は `LANG=en_US.UTF-8` に固定する (OS の言語に合わせる画面を、どの環境でも英語にする)。
- テストが書くファイル (スタジオの見本のプロファイル・エディタの見本のプロジェクト) は `target/e2e/<アプリ>/work/` に置く
  (`--manual` では `/tmp/ugoe-manual/work/`。下の「マニュアルの画像」)。スタジオの見本のプロファイルは、中のパスを `assets/samples` の
  絶対パスに解き、見本の印を外した写しを work に書く (`resolveSampleProfile`。印が無いので自動の保存を確かめられる)。見本の印の扱い
  (自動で保存しない) は、印を残した写しを別に置いて確かめる。
- スタジオの操作 API は空いているポートで開く (利用者のスタジオの既定のポート 28451 とぶつけない。マニュアルの画像は既定のポートで撮る。下の「マニュアルの画像」)。見本のプロファイルの
  ホットキー (OS 全体に登録される) は外してから開く。

### 外に出さない

エディタの AI のプロバイダは、外のサービス (Claude API・OpenRouter など) に HTTPS で送る。筋書きはどれも外に送らない
(偽の AI のプロバイダはループバック、外のサービスに送る前の確認は「Cancel」で閉じる) が、誤って送らないように run.mjs が見張る (T0969)。

- run.mjs は 127.0.0.1 の空いているポートにプロキシの形の止める口を立て、エディタの `ALL_PROXY`・`HTTPS_PROXY`・`HTTP_PROXY`
  (と小文字) をそこに向ける。`NO_PROXY` はループバック (`127.0.0.1,localhost,::1`) で、偽の AI のプロバイダ・MCP のブリッジ・WebDriver は
  直に通る。AI のプロバイダの HTTP (ureq。crates/ugoe-ai) はこれらを読む。
- 止める口は、来た要求 (HTTPS の `CONNECT` と HTTP) をすべて 403 で断って覚える。終わったときに 1 つでもあれば、
  `editor tried to reach the network (blocked)` と行き先を出して、そのアプリを失敗にする。
- 送り先を `https://example.invalid/v1` (名前の引けない送り先) にして「Test connection」で送らせると、`CONNECT example.invalid:443` が
  止まって失敗になることを確かめた (T0969。外には何も出ない)。

### 鍵の保管庫

エディタは API キーを OS の資格情報の保存先 (Linux では Secret Service) に置く (apps/editor/src-tauri/src/ai.rs)。保管庫が無いと
設定の「AI provider」は「保管庫が無い」の案内になり、API キーの欄を確かめられない。そこで run.mjs は、エディタの D-Bus のセッションの中で
gnome-keyring-daemon を動かす (T0050)。

- `gnome-keyring-daemon --unlock --components=secrets` に決まったパスワード (`ugoe-e2e`。秘密ではない) を渡し、ログインの保管庫を作って解錠する。
  gnome-keyring は空のパスワードではログインの保管庫を作らないので、空にはしない。Secret Service だけを動かす (SSH・PKCS#11 は動かさない)。
- 保管庫のファイルは `XDG_DATA_HOME` (`target/e2e/editor/xdg/data/keyrings/`) に作られ、毎回消してから始める。
- `XDG_RUNTIME_DIR` を新しい一時ディレクトリ (`/tmp/ugoe-e2e-*`。終わったら消す) にする。gnome-keyring は `$XDG_RUNTIME_DIR/keyring` で
  動いている保管庫を探し、見つけるとそれを使うので、利用者の保管庫 (`/run/user/<uid>/keyring`) につながないように。
  制御のソケットのパスは 108 バイトまでなので、`target/` の下ではなく短い一時ディレクトリに置く。
- 保管庫は D-Bus のセッションが終わると止まる。
- gnome-keyring-daemon が無ければ保管庫を動かさず、保管庫の要る手順を `step(..., { skip: keyringSkip() })` で飛ばす
  (`lib/harness.mjs`。理由と入れ方が node:test の結果に出る)。動かせなかったとき (起動に失敗した) は driver.log に出て、手順が失敗する。

### 結果

```
target/e2e/
  bin/                          ビルドした実行ファイル
  <アプリ>/driver.log            tauri-driver・WebKitWebDriver・アプリの標準出力と標準エラー
  <アプリ>/<アプリ>.console.json  終わったときの、各ウィンドウのページの console (起動の直後から) と CSP が止めたもの
  <アプリ>/shots/                途中のスクリーンショット (成功したときも見返せるように)
  <アプリ>/failures/<手順>/       失敗したとき: 各ウィンドウのスクリーンショット・画面全体 (screen.png)・console.json
  studio/tracking.json          重い検出の下でのトラッキングの測定 (set_tracking の回数と応答の時間・AngleX の推移・トラッキングの待ち)
```

`studio/tracking.json` の「トラッキングの待ち」(「フレームレートと負荷」の Tracking wait) の欄:

- `trackingMs`: トラッキングを流した長さ (ミリ秒。既定 13000。下の「長く流す」)。
- `trackingWaits`: トラッキングを始めて 6 秒 (見る値が 10 個そろう約 5 秒の後) から流し終わる (既定 13 秒) まで、操作ウィンドウに届いた状態
  (`window.__UGOE_STUDIO_STATUS__` の `trackingWait`。表示の文には値の数が出ないので状態から読む) の約 1 秒ごとの集計
  (前に読んだものと同じ集計は除く)。各回は `atMs` (始めてからの時刻) と `wait` (`meanMs`・`maxMs`・`late` (刻みに遅れた割合、0〜1)・
  `values` (届いた値の数)・`lateValues` (そのうち遅れた値の数)・`marginMs` (余裕))。その約 1 秒に値が届かなければ `wait` は `null`。
- `trackingWaitSummary`: 値の有った回数 `samples`・無かった回数 `unread`・届いた値の数の合計 `values` と遅れた値の数の合計 `lateValues`・
  値の数で重みを付けた遅れた割合 `latePercent` (合計の比、%)・各回の割合の平均 `meanLatePercent` と最大 `maxLatePercent`・
  待ちの平均の平均 `meanMeanMs`・最大の最大 `maxMaxMs`。
- `trackingWait`: 手順の終わりの表示の文 (1 回分)。

1 回の集計は直近の約 1 秒 (検出 200 ms では値 2〜3 個) で、1 つ遅れるだけで 0〜67% と揺れる。`pacing.rs` を変えたときは
まず仮の時刻のテストで比べ、実機でも確かめるなら前と後で長く (下の「長く流す」) 数回ずつ流し、`trackingWaitSummary` の `latePercent` (と `values`・`lateValues`) を比べる。筋書きは表示が出ることだけを確かめ、割合や待ちの値では落とさない
(値は計算機の重さで決まり、閾値を決めると揺れで落ちる。値の良し悪しは `pacing.rs` の仮の時刻のテストで決める)。

### 長く流す

既定の 13 秒では「トラッキングの待ち」の値が 12〜20 個ほどしか集まらず、遅れた値の割合が回ごとに 0〜28% と揺れる (T0873)。
`pacing.rs` を変えた前と後を比べるときだけ、環境変数 `UGOE_E2E_TRACKING_MS` (ミリ秒) で重い検出のトラッキングを長く流す (T0876)。

```sh
UGOE_E2E_TRACKING_MS=60000 just e2e --only studio --no-build
```

- 範囲は 13000 (既定) から 120000 までの整数のミリ秒 (`lib/tracking-length.mjs`)。範囲の外や数でない値なら、ビルドの前に名前と値を出して止まる (終了コード 2)。
  120 秒で値は約 280 個 (1 個が約 0.4%) 集まり比べるのに足りる。それより長いと 1 回が数分になり、その間の計算機の重さの変化も混ざるので、回を重ねる。
- 判定の意味は既定と同じ。`set_tracking` が届く回数の下限は長さに比例させる (13 秒で 10 回、60 秒で 47 回、120 秒で 93 回)。
  下限を 10 回のままにすると、長く流したときに直す前の遅さ (50 秒で 3 回。T0812) に近くても通ってしまうため。
- 60 秒で値は約 125 個集まり、1 回の中の割合は落ち着く (前半・中ほど・後半でほぼ同じ)。ただし **回ごとの差は残る**: 同じコードで 5 回流して
  9%・11%・11%・15%・32% だった (T0876)。
- この差は、値の届く時機と刻みの時機の関係 (位相) が回の初めに決まって残るためというより、1 回の値の数が少ないためと見る (T0878)。
  `pacing.rs` の仮の時刻のテスト `late_values_spread_over_phases` で e2e に近い形 (検出 200 ms + 0〜2 ms、スリープが 0〜2 ms 遅れる混んだ計算機) を
  60 秒回すと、位相だけを振っても割合が 12〜38% に分かれ、揺れの種だけを振っても同じほど分かれる。約 110 個の値の数え方の揺れ
  (割合 20% で標準偏差 約 4 ポイント) と同じ大きさで、1 回の中で 15 秒ごとにトラッキングを始め直して位相を取り直す形を仮の時刻で
  模しても、回ごとの差は狭まらなかった (数える値が減る分もある)。そのため e2e の手順は始め直さず、60 秒の 1 回を 1 つの標本として回を重ねる。

前と後を比べる手順:

1. **仮の時刻のテストで比べる。** 前と後のコードで
   `cargo test -p ugoe-studio --lib late_values_spread_over_phases -- --nocapture` を流し、出力の行 (速さと計算機の形ごとに、位相を 32 通りに
   振った遅れた値の割合の最小・中央値・最大と、すべての位相の値 (約 3,500 個) を合わせた割合・待ちの平均の幅) を並べる。値は決まっていて
   揺れないので 1 回ずつで比べられ、1 分かからない。中央値か最大が上限 (テストのコメント) を超えると落ちる。
2. 実機でも確かめるときは e2e を長く流す。前のコード (`pacing.rs` を変える前のコミット。一時的なコミットか別の worktree で作る) で
   `just e2e --only studio` をビルドして流し、続けて `UGOE_E2E_TRACKING_MS=60000 just e2e --only studio --no-build` を 3〜5 回流す。
   毎回 `target/e2e/studio/tracking.json` を別の名前に写しておく (次の回で上書きされる)。後のコードで同じことをする。ほかのビルドが重なると
   割合が上がるので、前と後はなるべく同じ計算機の重さで流す。
3. 各回の `trackingWaitSummary` の `latePercent`・`values`・`lateValues` と `meanMeanMs` を並べ、前と後それぞれ `lateValues` と `values` を
   すべての回で足した割合も比べる。回ごとの差 (上の 9〜32%) より大きく、ほとんどの回で同じ向きに動いていれば差があるとみる。

CI では失敗したときに `target/e2e` (実行ファイルを除く) を成果物として残す。

## 筋書き

| アプリ | 手順 |
| --- | --- |
| エディタ (`specs/editor.test.mjs`) | 起動 → ダイアログで選んでいないパスを渡したコマンド (`import_psd`・`export_bundle`) が `file.not_chosen`・`file.not_chosen_for_save` で断られる → 「Open Sample…」で見本のプロジェクトを書き出して開き、ビューポートが描ける → パラメータ (AngleX) の値を入力する → メタデータの Version を変えて保存し、ファイルに書かれ未保存の印が消える → 「Import PSD…」で `assets/fixtures/character.psd` を取り込み、ツリーにレイヤーが出る → 「Export…」で `.ugoe` を書き出し、ファイルができて一時ファイルが残らない → プレビューで見本の `cat-ears.ugoemod` を読み込んで付き、チェックで外し、スロットの選択欄で付け直す → 画像からトラッキングし、MediaPipe が顔を見つけて口の開きが動く → 見本のモジュールのプロジェクト `cat-ears.ugoeproj` を開き (保存していない変更を捨てる確認を通る)、「Export Module…」で `.ugoemod` を書き出し、見本の `cat-ears.ugoemod` と同じバイト列で一時ファイルが残らない → 設定の「AI provider」で API キーを保存し (欄は空になり、キーは画面に出ない)、設定を開き直しても保存済みで、確かめてから削除する (鍵の保管庫。無ければ飛ばす) → 設定の AI の送り先を偽の AI のプロバイダにする → 見本の着せ替えの本体を開き直して「Import PSD…」で `character.psd` を取り込み、AI タブの「Propose rig」で分類とリグの提案を作り、「Preview」で提案を適用した姿 (`get_proposal_preview_bundle`) がビューポートに出て、「End preview」で戻りドキュメントが変わらず、「Adopt」で本体の物理に髪揺れのリグが足されて衝突体 `Shoulders` とスロット `Hair` の参照が残る (T1627 の回帰) → 「New from PSD…」で `character.psd` から作った物理の無いプロジェクトでも、同じく提案を作ってプレビューする → 設定で日本語に切り替え、読み込み直しても日本語のまま → CSP の違反が無い |
| ビューア (`specs/viewer.test.mjs`) | 起動 → 「Samples」のメニューから見本を開く → トラッキングの記録を流し、AngleX が動く → 画像からトラッキングし、口の開きが動く → 設定を開いて閉じる → CSP の違反が無い |
| スタジオ (`specs/studio.test.mjs`) | 起動 → 見本のプロファイルを開く → シーンをクリックで切り替える → 出力ウィンドウを出し、「いまの状態」に見本の大きさ (1920 × 1080) が出て、フレームが届く → 透過が効かない (Xvfb にコンポジタは無い) と「背景」の欄に注意が出て、注意のボタンで背景がクロマキーになり注意が消え、「シーンの編集」で透過に戻すとまた出る (T0846・T0849) → 重い検出の下でトラッキングが操作ウィンドウと出力に届く (T0812 の回帰) → 出力ウィンドウを隠してもフレームが届き続ける → 画像からトラッキングし、口の開きが動く → 操作 API を入れると「いまの状態」に「API on」の印が出て、クライアントを承認し (承認の後の「取り消す」の知らせが出る)、シーンを切り替える → ブラウザソースを入れると「Source on」の印が出て、ページとフレームが届く → シーンを足すとプロファイルに自動で保存され、開き直しても残る → 見本の印のあるプロファイルを開くと保存の印が「Sample」になり、シーンを足すと「Save a copy…」の知らせが出て、待ってもファイルは変わらない (T0866) → CSP の違反が無い |

画面の操作はできるだけ WebDriver の入力 (クリック・キー) で行う。次のものだけは OS のものなので、テストが代わりをする。

- OS のファイルダイアログ: エディタはデバッグビルドのバックエンドの差し替えを使う。`run.mjs` が環境変数
  `UGOE_EDITOR_DIALOG_ANSWERS` (`target/e2e/editor/dialog-answers/`) を渡し、バックエンドは OS のダイアログの代わりにそこの答えのファイルを
  読む (apps/editor/src-tauri/src/dialog.rs、T1616)。バックエンドが選んだパスを覚えるので、コマンドはそのパスだけを読み書きできる
  (画面の `window.__UGOE_FILE_DIALOGS__` の差し替えはバックエンドが知らないので、本物のアプリでは使えない)。
  スタジオは見本のプロファイルを最近のプロファイルに置いてから起動し、最後のプロファイルとして開かせる (T0838)。
- OS のファイルダイアログでファイルやフォルダーを選ぶところ (エディタの「Open Sample…」「Open…」「Import PSD…」「New from PSD…」
  「Export…」「Export Module…」・着せ替えの「Load .ugoemod…」): 筋書きが `answerDialog` で次のダイアログが返すパスを決めてから、ボタンを押す。
- AI のプロバイダ: 下の「偽の AI のプロバイダ」。
- カメラ: 下の「偽のカメラ」と「画像からのトラッキング」。
- 操作 API のポートとトークン・ブラウザソースの URL は、パネルの表示の代わりにバックエンドのコマンドで読む (クリップボードを使わないため)。

### 偽のカメラ

本物のカメラと MediaPipe の代わりに、決まった動き (頭を左右に振り、傾け、口を開け閉めする) の記録を
`window.__UGOE_TRACKER__.startReplay` で再生する (`lib/tracking.mjs`)。MediaPipe のモデルと顔の映像が要らず、値が決まるので判定が安定する。

重い検出は、トラッカーが刻みごとに値を書くところ (`replayTime`。Svelte の `$state` のアクセサ) をこのインスタンスだけ差し替え、
書くたびに主スレッドを 200ms 塞いで模す (WSL2 の WebKitGTK での MediaPipe の顔と体の検出 1 回の重さ)。
刻みの処理の中で重くなるので、T0812 の不具合 (刻みが溜まり `set_tracking` の応答が何秒も遅れる) がそのまま起きる。
スタジオの筋書きは、13 秒の間 (`UGOE_E2E_TRACKING_MS` で長くできる。上の「長く流す」) に `set_tracking` が 10 回以上 (長さに比例) 届き、応答の中央値が 1 秒未満で、操作ウィンドウの AngleX が動き、
「フレームレートと負荷」に「トラッキングの待ち」が出ることを確かめる (その間の表示を `tracking.json` に残す。上の「結果」)。

### 偽の AI のプロバイダ

エディタの AI の提案 (PSD のレイヤーの分類とリグの値) は、本物の AI に頼らず、テストがこの計算機 (127.0.0.1 の空いているポート) に
立てる OpenAI 互換の Chat Completions のサーバ (`lib/fake-ai.mjs`) に答えさせる (T0029)。ネットワークも API キーも要らない。
エディタは送り先がループバックなら送る前の確認を出さず、API キーも求めないので、画面の操作は本物の手元のサーバ (Ollama など) と同じ。

- 送り先は設定の「AI provider」で、「OpenAI-compatible API」を選び Base URL を `http://127.0.0.1:<ポート>/v1` にする (画面の操作で)。
- 答えは要求の JSON Schema から決める。分類 (`layers` を持つスキーマ) には、プロンプトのレイヤーの行の名前から決めたラベル
  (`character.psd` の `顔`・`右目`・`口` など。知らない名前は `other`)、リグの値には説明の範囲 (`From <min> to <max>.`) の真ん中と
  決まった理由の文を答える。筋書きは、分類とリグの要求が画像付きで届いたこと・分類が AI のものになったこと・提案の理由に
  決まった文が入ったことを確かめる。
- 提案の元は 2 つ: 見本の着せ替えの本体に「Import PSD…」で取り込んだ PSD (物理とスロットを持つ本体に提案を重ねる。リグの提案が
  本体の物理の衝突体 `Shoulders` を外し、スロット `Hair` の参照が壊れて提案が作れなかった T1627 の回帰) と、「New from PSD…」で
  作った本体の要素の無いプロジェクト (T0039)。どちらも `proposeAndPreview` (specs/editor.test.mjs) で提案とプレビューを確かめる。
- 利用者の注文 (プロンプトの `<user_instructions>`) があれば、文 (`。`・`.`・`!`・`?` で区切る) ごとに答える。髪・揺れ・帽子のように
  リグの値では変えられない文は、理由に `Not applied:` の行を足す (マニュアルの画像 `ai-revision` の「Not applied」)。ほかの文には従う:
  文が値の言葉 (振り向き・頭・turn・head → `turn_angle_scale`、目・eye → `eye_close_line`、笑・smile → `eye_close_bend`、口・mouth →
  `mouth_open_height`) と向きの言葉 (控えめ・小さ・細・弱・less・smaller・narrower など → 下げる、大き・太・広・強・more・larger・wider など →
  上げる。笑いは逆で、上げる言葉で `eye_close_bend` を下げる) を持てば、その値を範囲の 6 分の 1 だけ動かす (文ごとに足し、範囲の端で止める)。
  同じ注文なら同じ値。どちらかの言葉が無い文は、値を変えずに従ったことにする。理由には従った文と動かした値
  (`(Moved by a sixth of the range: turn_angle_scale down, ...)`) を足す。マニュアルの筋書きの作り直し (「Turn the head less. Close the eyes
  a little narrower. Make the hair sway more.」) は、振り向き 0.75 → 0.60・目の閉じる線 0.60 → 0.50 になり、筋書きがその値を確かめる (T0061)。
- MCP の経路 (エディタにつないだエージェントの提案) は筋書きでは使わない。ポートとトークンの準備が要り、画面の操作が提案の一覧から先は同じため。
  マニュアルの画像は偽のエージェントで撮る (下の「偽のエージェント」)。

### 偽のエージェント

マニュアルの MCP の画像 (`mcp-*`) は、Claude Code や Claude Desktop の代わりに、テストがエディタの中の MCP サーバ (ブリッジ) に
MCP の Streamable HTTP でつなぐ小さなクライアント (`lib/fake-agent.mjs`) で撮る。本物のモデルには何も送らない。

- 送り先とトークンは、画面の表示の代わりにバックエンドのコマンド (`mcp_bridge_status`・`get_mcp_bridge_token`) で読む。
- `initialize` の `clientInfo` で名乗る (`claude-code` 2.5.0・`claude-ai` 1.2.0)。「Connected agents」にその名前が出る。
  つないだエージェントの一覧は待ち受けを入れ直すと空になるので、画像ごとに「Allow agents to connect to this editor」を入れ直す。
- 適用の確認 (`mcp-approval`) は、エージェントが `get_psd_layers`・`set_layer_labels` (偽の AI のプロバイダと同じ、名前からのラベル)・
  `propose_rig` で提案を作り、`apply_proposal` を送って答えを待つ間に撮る。撮った後に「Apply」を押す。


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
MediaPipe が読めた後で、ページの `fetch` が MediaPipe の利用状況の送り先 (`https://odml.pa.googleapis.com/v1/log`) への要求を
ネットワークに出さずに断ることも確かめる (@ugoe/face-tracking の `usage-logging.ts`、T0943)。断らずに出すと CSP (`connect-src`) が止め、
最後の手順で落ちる。

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

止まって正しいものも数える。MediaPipe (@mediapipe/tasks-vision) の利用状況の送信 (60 秒ごとに `https://odml.pa.googleapis.com/v1/log` へ)
は、以前は CSP が止めて数えないことにしていた (T0029) が、今は @ugoe/face-tracking が送る前に断るので CSP の違反にならない (T0943)。

わざと CSP に止めさせる手順 (スタジオの出力ウィンドウで `new Function` が止まることを確かめる) は、止まったことを
`takeCspViolations` で確かめて取り除く (最後の手順で数えないように)。

`script-src` から `'wasm-unsafe-eval'` を外したビルドで、3 つのアプリとも失敗することを確かめた (T0022)。設定のファイルを書き換えずに
試すには、`{"app":{"security":{"csp":{"script-src":"'self'"}}}}` を書いた JSON を作り、
`pnpm --filter @ugoe/viewer tauri build --debug --no-bundle --config <その JSON>` でビルドして `target/debug/` の実行ファイルを
`<out>/bin/` に写し、`just e2e --no-build --only viewer --out <out>` で流す。

## マニュアルの画像

`just e2e --manual` は、筋書きの代わりに `specs/<アプリ>.manual.mjs` を流し、マニュアル (docs/manual) の画像を本物の WebView
(Linux の WebKitGTK) で撮る (T0919)。撮った画像は `<out>/<アプリ>/manual/<名前>.png` に置く。`--update-images` を付けると、
256 色に減らして (`convert -dither None -colors 256`) `docs/manual/images/<名前>.png` に写す。画面を変えたら撮り直して、差分の画像を見て確かめる。

- 窓の大きさは筋書きが決める (`lib/manual.mjs` の `resizeTo`。Xvfb は devicePixelRatio 1)。エディタ 1280×800 (スロットとグルーの画像は
  1280×1000 の窓で右の列だけを切り出す)、ビューア 1300×820・1100×760、スタジオ 1280×800 (パネルの画像は 1280×1000 の窓でパネルだけを切り出す)。
  エディタは撮る前にポインタをヘッダーの空いた所へ動かす (Tooltip とホバーの輪郭を写さない)。
- 言語: エディタは英語 (マニュアルが英語の名前で書く)、ビューアとスタジオは日本語 (localStorage の言語の設定を変えて読み込み直す)。
  スタジオは日本語の見本のプロファイル (`sample-ja.json`。ホットキーは Xvfb の画面にだけ登録する。`--headed` では外す)。
- 中身: エディタは「Open Sample…」の見本の着せ替えと `character.psd`、ライティングと検証は `ugoe sample avatar --lit` で作ったプロジェクト、
  モジュールは見本の `cat-ears`・`sailor`・`long-hair` のプロジェクト。トラッキングは偽のカメラ (記録の再生)。
- チュートリアル (docs/manual/tutorial.md) の途中の姿 (`mesh-edit`・`keyform-grid`・`mirror`・`depth-rotation`) は、`character.psd` から
  3〜10 章の手順のとおりに画面で操作して撮る (T0968)。チュートリアルに書いた ID や表示 (`WarpGroup18`・`Mirrored 1 cell …`・
  `16 control points · 0 – 35.28 px`) が出るのを確かめるので、手順の文と画面がずれると失敗する。ギズモの点・光源の取っ手・
  グルーの頂点は、位置を測ってマウスで操作する (`session.pointer`。W3C WebDriver の Perform Actions)。
- モジュールの作成と比較 (`module-*`): 設定とスロットの移し替え・アトラスでの書き出し・書き出したファイルといまの編集の比較
  (耳を消し、Band の頂点と描画順を変える・Sailor の袖のポーズの追従を外す)・グルーと組・本体の要素の選択・新しいモジュールの欄。
  本体の問題 (`module-problem`・`module-problem-drop`) は、見本の本体の写しからスロット `Headwear` を消したもの・物理を消したものを
  作業の場所に作り、その隣に写したモジュールを開いて撮る。点光源 (`lighting-point`) は、ライティングのプロジェクトで「Add point」を押し、
  取っ手を右の肩のそばへドラッグする。
- AI の設定 (`ai-settings`) は、OpenRouter を選んで適用し、送り先の欄と API キーの欄を撮る。API キーの欄は鍵の保管庫が要る
  (上の「鍵の保管庫」。無ければ「Apply」の行までを撮る)。
- AI の提案 (`ai-*`。T0969) は `character.psd` から作って保存したプロジェクトで撮る。右の列を広げる (列の区切りを 80px 左へ、
  上下の区切りを下の端へ。区切りにフォーカスしてキーで)。
  - 注文の欄 (`ai-instructions`) と送る前の確認 (`ai-consent`) は、Claude を選び、偽のキー (`sk-ant-e2e-not-a-real-key`) を鍵の保管庫に
    置いて撮る。名前で分類してから注文を書いて「Propose rig」を押し、出た確認 (リグの値) を撮って「Cancel」で閉じる (何も送らない。
    上の「外に出さない」)。鍵の保管庫が無ければ飛ばす。
  - 提案 (`ai-proposal`)・作り直し (`ai-revision`) は、送り先を偽の AI のプロバイダにして、AI で分類してから提案する。提案をプレビューし、
    提案の下のスライダーの `AngleX` と `MouthOpenY` を最大にする。作り直しは注文 (髪の揺れを含む) を書いて「Regenerate with instructions」を押す。
  - 法線マップ (`ai-normal-maps`) は `ugoe sample avatar` (法線マップの無いもの) のプロジェクトで、顔のまとまりの縁の丸みを開いて提案し、
    プレビューする (光源は最初のもの)。
- MCP (`mcp-*`) は同じ `character.psd` のプロジェクトで、偽のエージェント (上の「偽のエージェント」) をつないで撮る。
  Claude Desktop のタブ (`mcp-claude-desktop`) は「Save .mcpb…」で `.mcpb` を保存した後に、1280×1000 の窓で撮る (ダイアログの全体が入るように)。
- 画像は公開されるので、流した機械の場所 (ユーザー名・worktree・ビルドのキャッシュ) を写さない。`--manual` では run.mjs が
  作業の場所を `/tmp/ugoe-manual/work/`、実行ファイル (エディタと `ugoe-mcp`) を `/tmp/ugoe-manual/bin/` に写して起動する (T0969)。
  画面に出るパス (「Agents work on」・登録のコマンドの `ugoe-mcp`・保存した `.mcpb`・新しいモジュールの本体) は、どの機械でも
  `/tmp/ugoe-manual/…` になる。実行ファイルは写す (シンボリックリンクにしない): エディタは自分の実行ファイルの実際の場所の隣に
  `ugoe-mcp` を探し、その場所を出す。決まった場所なので、`--manual` は同じ機械で同時に流せない (`/tmp/ugoe-manual/lock` に
  動いている実行のプロセスの番号を書き、ほかの実行がいれば終了コード 2 で止まる)。
- スタジオの画像に出る、撮った機械に依るもの (T0977):
  - 操作 API とブラウザソースは既定のポート 28451 で開く (接続先・URL・ポートの欄がマニュアルの文と合い、撮るたびに変わらないように)。
    28451 をほかのもの (利用者のスタジオなど) が使っていれば、最初の手順で理由を出して止まる。
  - 「出力ウィンドウ」のパネル (`studio-output-panel`) は、Xvfb にコンポジタが無いために出る「透過が効いていません」を、
    マニュアルの説明の例として写す (出ていることを確かめる。コンポジタのある画面の `--headed` では失敗する)。
  - 「フレームレートと負荷」(`studio-load`) の数値は撮った機械の測定のまま写す。Xvfb はソフトウェアで描くので、出力ウィンドウの
    描画の遅れの注意が出たり消えたりする。読者の画面ではふつう出ないので、注意の出ていない時に撮る (撮った後も出ていないことを
    確かめ、出ていれば撮り直す。100 秒ほど出続けると失敗する)。
- スタジオの「入力」の調整 (`studio-inputs-tuning`。T0984) は、記録の再生の首が正面の近くを通る時に「キャリブレーション」を押し、
  調整の値を既定から動かして (首の左右の感度と遊び・傾きの遊び・瞼と口の開きの境目)、開いた「調整 (感度と境目)」だけを撮る
  (メーターに帯・縦の線・点線が出るように)。撮った後は「既定に戻す」を押して閉じる。
- 撮らないもの (前の画像のまま): スタジオの出力の例 (`studio-output`。Xvfb にはコンポジタが無く透過を撮れない) と仮想カメラ
  (`studio-virtual-camera`。v4l2loopback が要る)。この 2 枚は手で撮る ([docs/manual-check.md](../../docs/manual-check.md) の U06)。
- WSL の中で流しても、`--manual` ではアプリに `WSL_DISTRO_NAME` を渡さない (エディタの「Connect agents」に WSL の案内を写さない)。

### ヘッドレスの Chrome (`just ui-audit`) との違い

T0919 で、同じ画面を `just ui-audit` (偽のホストとヘッドレスの Chrome) と比べて分かった違い。

- 書体: WebKitGTK は fontconfig の既定 (DejaVu Sans・Noto Sans CJK) で描き、太字 (600) が太く出る (見出し・Section の名前)。
  Chrome は細い字面で、「…」を CJK の書体の「⋯」(中央の点) で描く。文字の幅と行の高さが違うので、折り返しの位置も違う
  (ビューアの 1100px の窓で、日本語のトラッキングの欄の「左右を入れ替える」が 2 行になる)。
- スクロールバー: WebKitGTK はスクロールバーを幅を取らずに中身の上に重ねて描く (`offsetWidth - clientWidth` が 0。`scrollbar-gutter` も
  幅を空けない)。細い印は右端から 3〜6px、指を乗せると 6〜14px に太る。ヘッドレスの Chrome は幅を取る形 (15px) で描くので、
  重なりは監査にも画像にも出ない。T0919 で右端の数 (ビューアのパラメータのグループの数) やスタジオの右の列の部品に重なっていたので、
  T0970 で送る枠の右の padding を `--ugoe-scrollbar-gap` にした ([docs/ui.md](../../docs/ui.md#レイアウト) の「送り」)。
- 偽のホストと本物のバックエンドの違いで、本物でだけ起きた不具合: スタジオの承認の後の知らせが出ない (承認すると ID が変わる)、
  出力ウィンドウの大きさが前のまま (960×540) で「OS が小さくした」の注意が出る、空の `WAYLAND_DISPLAY` で Wayland と見なして
  ホットキーが「前面でだけ」になる。どれも T0919 で直し、e2e の筋書きに足した。
- 偽のホストの見本は、T0919 の時点では本物とずれていた (スタジオの「雑談」のポーズのグループ・パラメータ・プロファイルの拡張子、
  エディタの AI の提案の見本の、実在しないコマンドの種類)。T0047 で本物にそろえた。偽のホストは指揮役やバックエンドの答えを
  作りもので返すので、見た目の確認には、本物の見本で撮るこの画像を使う。

## 筋書きを足す

- `specs/<アプリ>.test.mjs` に `step(app, "名前", async () => { ... })` を足す。`step` は失敗したときの記録を残す (`lib/harness.mjs`)。
  1 つのアプリは 1 回だけ起動し、手順は上から順に同じウィンドウで進む (前の手順が失敗すると後の手順も失敗しやすい)。
  最後の手順は `checkNoCspViolations(app)` のままにする (上の「CSP の違反」)。
- 部品は表示の文字 (英語) と `@ugoe/ui` の部品のクラス・ARIA の属性で探す (`lib/page.mjs` の `window.__e2e`)。
  表示の文字を変えたら筋書きも直す。アプリの中のクラス (`.studio__…` など) には頼らない。並びを変えると消えるので、
  リストの項目は役割と名前で探す (`__e2e.listItem("^Status$", "^API on$")`: 名前が Status のリストの中の「API on」の項目。T0847)。
  文字を見せない部品 (エディタの右の列のアイコンのタブ) は役割と読み上げの名前で探す (`__e2e.byRole("tab", "^AI")`。T1632)。
- ページの中で動かすコードは関数で書き、関数のまま `session.execute(fn, ...args)` / `session.run(fn, ...args)` に渡す
  (`lib/webdriver.mjs` が関数のソースをページに送り、`args` を引数にして呼ぶ)。関数はページで動くので、Node の側の変数は使えない。
  要るものは `args` で渡す (JSON にできる値と、`find` などで得た要素)。コードを文字列で書くと、正規表現のエスケープが二重になり
  (`/\\s+/`)、整形 (Biome) が書き換えて意味が変わることがある (T0042)。`window.__e2e` の道具を足すときは `lib/page.mjs` の
  `pageHelpers` に足す。
- 待つときは決まった時間眠らず、`waitFor` で条件を待つ。
- 操作の直後に押すボタン・閉じかけのダイアログの下の部品は、`session.clickText`・`session.clickWhen` で押す。無効なあいだ待ち、
  「element not interactable」(覆いの下・押す前に画面が組み替わった) なら探し直して押し直す。重い負荷の下 (並行のビルドなど) では、
  ただの `click` はこれで失敗することがある (T0029)。
- WebDriver のクライアントは `lib/webdriver.mjs` の小さなもの (W3C WebDriver のうち使う分だけ。npm の依存なし)。

## Windows と macOS

流さない。macOS の WKWebView には WebDriver が無く、tauri-driver も対応していない。Windows は tauri-driver が
Microsoft Edge Driver に中継して動くはずだが、確かめていない。両方とも、リリースの前に手で確かめる一覧 ([docs/manual-check.md](../../docs/manual-check.md)) で見る
([docs/testing.md](../../docs/testing.md#実機の-e2e-テスト))。
