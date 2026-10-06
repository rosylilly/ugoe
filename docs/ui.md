# UI と文言のガイドライン

エディタ・ビューア・スタジオで共有する UI の部品 (`@ugoe/ui`) と翻訳 (`@ugoe/i18n`) の約束です。
なぜこの形にしたかは [ADR 0029](adr/0029-shared-ui-and-i18n.md) にあります。部品とトークンの実装は T0902・T0903、翻訳の実装は T0901 で行います。
この文書と実装が食い違ったら、気づいた人がどちらかを直してください。

## パッケージ

| パッケージ | 置き場所 | 使う場面 |
| --- | --- | --- |
| `@ugoe/i18n` | `packages/i18n` | 画面に出す文言・数値・日付のすべて |
| `@ugoe/ui` | `packages/ui` | トークン (`tokens.css`) と部品。アプリの `main.ts` で `tokens.css` を 1 回だけ読む |
| `@ugoe/player` | `packages/player` | モデルの再生の層 (ビューアとスタジオ。WebPuppet の寿命 (`PuppetSurface`) とスクリーンショットの PNG への変換はエディタも使う) |

- 依存の向きは `apps/* → @ugoe/player → @ugoe/ui → @ugoe/i18n`。どれも GPL-3.0-only。
- アプリの中にしか無い部品 (エディタのパネルの見出しなど) はアプリに置く。2 つ以上のアプリで要るものになったら `@ugoe/ui` へ移す。
  再生の層の言葉 (ポーズのグループなど) を受け取る部品は `@ugoe/player`・`@ugoe/face-tracking` に置く (下の「パッケージの部品」)。

### WebPuppet の寿命

WebPuppet (ugoe-wasm) と、それを描く canvas の寿命 (作る・作り直す・捨てる) は、`@ugoe/player` の `PuppetSurface`
(`packages/player/src/surface.ts`) にまとめ、Player (ビューア・スタジオ) とエディタのビューポート (`ViewportController`) が使う (T0932)。
T0672・T0914 で同じ誤りが両方に入り、別々に直していたため。

- **置き場所:** `@ugoe/player` (GPL-3.0-only)。エディタは T0730 から `@ugoe/player` に依存している。MIT の `bindings/web` の
  `<ugoe-avatar>` は GPL のコードを使えないので、同じ振る舞いを自分で持つ (下の「`<ugoe-avatar>` との関係」)。
- **型の受け方:** WebPuppet の型 (wasm-bindgen の生成物) はアプリごとに違うので、`PuppetSurface<P extends SurfacePuppet>` は使うメソッド
  (`free`・`is_lost`・`resize` と、ビューを渡し直す `zoom`・`pan_x`・`pan_y`・`set_view`) だけの構造の型で受け、`created(puppet)` にはアプリの型のまま渡す。wasm の読み込みも `loadWasm` で受ける。
- **canvas:** `PuppetSurface` が作り、`attach(container)` の要素の先頭に置く。エディタは T0672 まで `generation` を増やして Svelte の
  `{#key}` で canvas を作り直していたが、canvas を作るのを `PuppetSurface` に移したので `generation` は除いた (Viewport.svelte は
  canvas を置く要素を渡すだけ。canvas の見た目は `:global(canvas)`・`canvasClass` で決める)。
- **WebGL2 への切り替え:** `"auto"` で作れなければ新しい canvas で `"webgl2"` を試す (1 つの canvas は WebGPU と WebGL2 を併用できない)。
  一度 WebGL2 に落ちたら、作り直しでも WebGL2 から始める。
- **古い作成の破棄:** `attach`・`detach` のたびに番号を進め、待った後で変わっていたら、遅れてできた WebPuppet と canvas を捨てる
  (`attach` の結果は `"superseded"`)。作り直しの途中で `detach` されたときも同じ。
- **GPU の喪失:** WebGPU のデバイスの喪失と WebGL2 のコンテキストの喪失は、どちらも `WebPuppet.is_lost()` で分かる (WebGL2 は
  ugoe-wasm が canvas の `webglcontextlost` で印を立てる。T0697 を T0932 にまとめた)。描く前に `checkLost()` で確かめ、真なら描かない。
  - **WebGPU:** `checkLost()` が気づいたらすぐ canvas ごと作り直す。
  - **WebGL2 (T0935):** `PuppetSurface` が canvas の `webglcontextlost` を `preventDefault` して、復元 (`webglcontextrestored`) を待ってから
    新しい canvas で作り直す (wgpu の資源は戻らないため)。待っている間は `checkLost()` が真 (描かない) で、作り直しは始めない。
    本物の GPU のリセットでは、復元の前に新しいコンテキストを作れないことがある (T0932 までのすぐ作り直す方式では「描画を始められない」に
    なり得た) ため、`<ugoe-avatar>` (T0425・T0439) に揃えた。復元はイベントで始めるので、要求されたときだけ描くエディタも次に操作するまで
    待たない。
  - **復元が届かないとき (T0938):** 喪失に気づいたらすぐ `waiting` で知らせ、`RESTORE_TIMEOUT_MS` (5 秒) 待っても復元が届かなければ、
    新しい canvas で作り直しを試す (ブラウザが復元を送らないまま待ち続けないため)。作り直しで作れなければ (GPU がまだ戻っていない)、
    `waiting` で知らせて同じだけ待って試し直す。試し直しも作り直しの 1 回に数えるので、続けて失敗すれば下の上限で諦める。
    遅れて届いた古い canvas の復元は無視する。
  - **作れないときの例外 (T0475):** `WebPuppet.create` は、GPU やブラウザの都合で作れないとき (canvas のコンテキストを取れない・取れた
    WebGL2 のコンテキストが既に失われているなど) に `name` が `"CreateError"` で `code` を持つ例外を投げる (panic しない。
    [crates/ugoe-wasm/README.md](../crates/ugoe-wasm/README.md#作成の失敗-t0475))。作り直しの途中なら上の「作れなければ」と同じく待って試し直し、
    回数に数える。最初の `attach` なら `failed(error)` にその例外を渡す。`<ugoe-avatar>` も同じ。
  - 作り直しは一度に 1 つで、捨てた WebPuppet の印と古い canvas のイベントは見ないので、2 つの知らせが重なっても作り直しは 1 回。
  - **読み込みの途中の喪失 (T0955):** wasm の読み込み (`load_bundle`・`load_json`) と着せ替え (`load_module`・付け外し) は、GPU を
    失った後 (`is_lost()` が真) に呼ぶと `code` が `device_lost` の例外を投げる。このときは喪失の知らせ (WebGL2 の `webglcontextlost`) は
    もう届いているか、次のフレームの `checkLost()` で気づく。使う側は、投げた後に `is_lost()` が真なら読み込めなかったとはせず
    (エラーの文を出さず)、開こうとしたものを作り直した後に開く。エラーの文は作り直しを諦めたとき (`failed`) だけ。`<ugoe-avatar>` の
    T0486 (`load()` は作り直した後に解決する) と同じ扱い。
    - **Player:** 開こうとしたモデル (と一緒に開くモジュール) を、WebPuppet が無い間に開いたものと同じく `#setUp` で開く (fit で見せ、
      パラメータの値も既定から。`#setUp` の読み直しの途中にまた失ったときは、利用者が開いたものではないのでビューと値を保つ)。
      `load` の Promise は待たずに返す (WebPuppet が無い間と同じ)。前のモデルのモジュールは除いたまま。モジュールの読み込みも同じく
      作り直した後に読む。付け外し (`attachModule` など) は WebPuppet が無い間と同じく何もせず `false` を返し、`dressUpError` を出さない
      (作り直しは付けていた組で戻す)。
    - **エディタのビューポート:** 表示するモデル (`setModel`) は読む前に覚えるので、文を出さずに `requestRender` で次のフレームの
      `checkLost()` へ回せば、作り直した WebPuppet に `#setUp` が今のモデルを読む。WebGL2 の復元を待つ間は「GPU の復元を待っています」の
      ままにする。
    - **エディタのプレビューの着せ替え (T1644):** 読み込もうとした `.ugoemod` のバイト列を取っておき、付け外しは付けたい組だけを変えて、
      作り直した WebPuppet に `PreviewMode.loaded` → `DressUp.loaded` で読み込んで付ける (文を出さない。[docs/editor.md](editor.md#プレビューの着せ替え))。
  - **続けて失ったとき (T0935):** 前の作り直しから 1 分 (`RESTART_WINDOW_MS`) 以内に失う・作れないことが続き、作り直しが `MAX_RESTARTS` (3) 回を
    超えたら (4 回目)、作り直さずに `failed(GpuLostRepeatedlyError)` で知らせる (数と時間は `<ugoe-avatar>` と同じ)。GPU が戻らない間に
    作り直しを繰り返して資源と時間を使い続けないため。次の `attach` と `restart` で数え直す。
    諦めた作り直しの中で作れなかったことがあれば、最後の失敗 (多くは `CreateError`) を `GpuLostRepeatedlyError` の `cause` に持たせる
    (T0946)。最初の作成で作れずに「作り直す」を押してまた作れなかったときなど、実際には GPU を「失い続けた」のではなく作れなかったので、
    使う側はその理由の文を出す (下の「失敗の文」)。どの作り直しも作れていた (作っては失った) なら `cause` は無い。
  - **復元の時間切れのタイマー (T0946 で確かめた):** `RESTORE_TIMEOUT_MS` のタイマー (`#restoreTimer`) を置くのは `webglcontextlost` だけで、
    復元を待っていないときだけ置く (待っている間の 2 つ目の知らせ・作り直している間・古い canvas の知らせでは置かない)。時間切れ・復元・
    WebPuppet を捨てるとき (`detach`・作り直し) に消すので、前のタイマーを消さずに置き直す経路は無い (`<ugoe-avatar>` の T0480 と同じ誤りは無い)。
    作れなかったときの試し直しの待ちは別のタイマー (`#recreate` の `sleep`) で、待った後に番号で古いものを捨てる。
  - **画面の知らせと「作り直す」(T0938):** 待つ間 (`waiting` から次の `created`・`failed` まで) は「GPU の復元を待っています」を出す
    (Player は `waitingForGpu`、エディタはビューポートの `status` を `loading` にして `viewport.gpuWaiting`)。諦めたら、Player は `status` を
    `unavailable` にして `error` にカタログの `errors.gpuLostRepeatedly` を、エディタは `status` を `error` にして `viewport.gpuLostRepeatedly`
    を出し、どちらも「作り直す」ボタンを出す (Player の `canRestart`・ViewportController の `canRestart`。ボタンはビューアの描く領域と
    エディタのビューポートの右上)。押すと `PuppetSurface.restart()` が同じ container で作り直し (回数は数え直す)、`created(puppet, true)` で
    GPU を失ったときの作り直しと同じく、開いていたモデル・付けていたモジュール・パラメータの値を戻す。作れなければ上と同じく待って試し直し、
    続けば諦めてまたボタンを出す。最初の `attach` で GPU やブラウザの都合で作れなかった (`failed` に `CreateError`) ときも、時間を置けば
    作れることがあるのでボタンを出す (T0941)。描画を始められないだけ (wasm が無い・`code` を持たない例外) のときはボタンを出さない
    (押しても変わらないため)。
    スタジオの出力ウィンドウは画面に知らせを出さないので、ボタンも置かない。
- **失敗の文 (T0941):** どの文がどこから来るか。
  | 場面 | Player (ビューア・スタジオ) | エディタのビューポート |
  | --- | --- | --- |
  | 続けて失って諦めた (`GpuLostRepeatedlyError`。`cause` が無い) | `player` の `errors.gpuLostRepeatedly` | `editor` の `viewport.gpuLostRepeatedly` |
  | 作り直しで作れずに諦めた (`GpuLostRepeatedlyError` の `cause`。T0946) | `player` の `errors.recreateFailed` に `cause` の文 (下の 2 行と同じ) を埋める | `editor` の `viewport.recreateFailed` に `cause` の `createErrorText` の文を埋める (`surfaceFailureText`) |
  | 作れなかった (`CreateError`。`code` ごと) | `player` の `createError.*` (`describeCreateError`) | `editor` の `viewport.createError.*` (`createErrorText`) |
  | 作れなかった (ほかの例外・知らないコード) | `player` の `errors.renderStart` に例外の文を埋める | 例外の文 (`errorText`) |
  | 読み込みの失敗 (`LoadError`) | `player` の `loadError.*` (`describeLoadError`)。GPU を失った `device_lost` は出さない (上の「読み込みの途中の喪失」) | 同じ。`is_lost()` が偽の `device_lost` は `editor` の `viewport.deviceLost` (`loadErrorText`) |
  | 読み戻しの失敗 (`RenderError` の `device_lost`) | `player` の `loadError.deviceLost` (`describeRenderError`) | `editor` の `viewport.deviceLost` (`screenshotErrorText`) |

  `player` のカタログの GPU が止まった文は「アプリを起動し直して (ブラウザではページを読み込み直して)」で、エディタはエディタを起動し直す
  案内にする。`CreateError` の文はどれも「描画を始められません」の理由と、できること (ほかのタブ・ウィンドウを閉じる・待って「作り直す」) を書く。
- **devicePixelRatio:** ResizeObserver は倍率だけの変化を知らせないので、`PuppetSurface` が `matchMedia("(resolution: …dppx)")` で聞き、
  描画バッファの大きさを合わせ直して `pixelRatioChanged` で知らせる。CSS ピクセルと描画バッファの画素の換算には `pixelRatio` を使う。
- **作り直した後の状態:** 背景・光源・モデル・パラメータの値を渡し直すのは使う側 (`created(puppet, recovered)`。値は `lost` で控える)。
- **作り直しでのビュー (T0942):** 作り直しは利用者の操作ではないので、見ている場所 (ズームとパン) を変えない。`PuppetSurface` が失った
  WebPuppet のビュー (`zoom`・`pan_x`・`pan_y`。ビューは CPU の側の状態なので GPU を失っても読める) を、パンを CSS ピクセルに直して覚え、
  作り直した WebPuppet に `created(puppet, true)` の前に `set_view` で渡す (作り直すまでに devicePixelRatio が変わっても、CSS ピクセルで
  同じ場所を見せる)。続けて失ったときと、諦めた後の `restart` でも同じビューを渡す。`attach` (作り直しでない作成) と `detach` で忘れる。
  - **Player:** モデルを読むたびに fit するが、作り直しの読み直し (`#setUp` の最初の読み込み) では fit せず、渡し直したビューを保つ
    (`view`・`zoom` は WebPuppet から読み直す)。作り直しを待つ間 (WebPuppet が無い) に利用者が開いたモデルと、作り直しの後に開いた
    モデルは、今までどおり fit で見せる。待つ間に開いたモデルには `lost` で控えた値も戻さず、既定から始める (前のモデルの値を同じ ID の
    パラメータに戻さない。T0945)。
  - **エディタのビューポート:** モデルを読んでもビューを変えない (fit は「全体を表示」と、別のモデルになったときの App.svelte) ので、
    `PuppetSurface` が渡し直すだけで保たれる。

`<ugoe-avatar>` との関係: GPU の喪失は同じく `is_lost()` で分かり、devicePixelRatio も同じく `matchMedia` で聞く。WebGL2 の
コンテキストの喪失で復元を待つこと、復元が 5 秒届かなければ作り直しを試すこと、作れなければ待って試し直すこと、続けて失ったときに諦める
数と時間も同じ (T0935・T0938 で揃えた)。違うのは知らせ方で、要素は待ち始めたら `gpulost` を送って `waitingForGpu` を真にし、諦めたら
`error` を送り `ready` を reject し、`restart()` で作り直せる (文言とボタンは要素を使うページが出す)。

## デザイントークン

見た目の値は、すべて `@ugoe/ui` の CSS 変数 (トークン) から取ります。**部品とアプリの CSS に色の値 (`#...`・`rgb()`) や余白の数値を直に書きません。**
例外は、描画の中身 (ビューポートの格子・ギズモ・光源の印など、モデルの上に重ねる図) の色で、これはテーマに依らない別の変数 (`--viewport-*`。下の「ビューポートの上の図」) で決めます。

### 名前と切り替え

- 名前は `--ugoe-<分類>-<名前>` (例: `--ugoe-color-text`・`--ugoe-space-3`)。アプリの古い変数 (`--bg` など) と衝突させないため、接頭辞を付けます。
- **ライトとダーク:** 既定は OS の設定 (`prefers-color-scheme`) に従います。`<html data-theme="light">` / `"dark"` で上書きできます (設定の「OS に合わせる・ライト・ダーク」)。
  3 つのアプリの設定に `@ugoe/ui` の `ThemeSetting` を置き、選んだテーマは `localStorage` のアプリのキー
  (`ugoe.editor.theme`・`ugoe.viewer.theme`・`ugoe.studio.theme`。キーはどのアプリも `ugoe.<アプリ>.<名前>` の形で、
  名前は camelCase、`.` は分野の区切りにだけ使います (`ugoe.editor.parameterSectionsClosed`・`ugoe.viewer.dressUp.autoRestore`・
  スタジオの出力ウィンドウの Player は `ugoe.studio.player.autoIdle`。T0971)。一覧は [docs/editor.md](editor.md#覚える値)・
  [apps/viewer/README.md](../apps/viewer/README.md#覚える値)・[docs/studio.md](studio.md#覚える値) の「覚える値」) に覚えます (T0909)。
  言語も同じ流儀で、`@ugoe/i18n` の `initLocale(key, OS の言語を取る関数)` にアプリの言語のキー (`ugoe.editor.locale`・`ugoe.viewer.locale` など) を渡し、
  設定の画面は `setLocalePreference()` で切り替えて覚えます (T0925。[packages/i18n/README.md の「言語」](../packages/i18n/README.md#言語))。
  localStorage の読み書きは、テーマ・言語・`@ugoe/player` の覚えるものとも `@ugoe/i18n` の `readStoredItem` などを通し、
  使えない localStorage でも例外を出しません (T0937。[packages/i18n/README.md](../packages/i18n/README.md#localstorage-の読み書き-t0937))。
  起動時のちらつきを避けるため、各アプリの `index.html` の `<head>` で同じキーを読んで先に `data-theme` を付けます。
  CSS の変数を JS で読むところ (エディタのビューポートの背景) は `currentTheme()` で変化を知ります。スタジオの出力ウィンドウには効かせません (透過のまま)。
  `color-scheme` も合わせて切り替え、スクロールバーやフォームの標準の部品の色もそろえます。
- **密度:** `<html data-density="compact">` (エディタ。基準の文字 13px。属性を省いたときもこれ) と `"comfortable"` (ビューア・スタジオ。基準の文字 15px)。
  余白と文字の大きさ・部品の高さ・アイコンの大きさのトークンは密度によって変わり、部品は密度を意識せずにトークンを使います。使い分けは下の「密度の使い分け」。

### 色

コントラスト比は WCAG 2.x の式で計算した値です (`bg` / `surface` / `sunken` に対して)。

| トークン | 用途 | ライト | ダーク | 比 (ライト) | 比 (ダーク) |
| --- | --- | --- | --- | --- | --- |
| `--ugoe-color-bg` | 画面の地 | `#f4f4f5` | `#18181b` | | |
| `--ugoe-color-surface` | パネル・ダイアログ・入力欄 | `#ffffff` | `#232327` | | |
| `--ugoe-color-sunken` | ビューポートの地・くぼんだ領域 | `#e4e4e7` | `#111113` | | |
| `--ugoe-color-border` | 区切りの線 (飾り。コントラストの基準の外) | `#d4d4d8` | `#3f3f46` | | |
| `--ugoe-color-control-border` | 入力欄・チェックボックスなどの枠 (部品の境界) | `#7a7a83` | `#7c7c86` | 3.9 / 4.3 / 3.4 | 4.3 / 3.8 / 4.6 |
| `--ugoe-color-text` | 本文 | `#18181b` | `#e4e4e7` | 16.1 / 17.7 / 14.0 | 14.0 / 12.3 / 14.9 |
| `--ugoe-color-text-muted` | 補足・単位・無効でない控えめな文字 | `#5f5f68` | `#a1a1aa` | 5.8 / 6.3 / 5.0 | 6.9 / 6.1 / 7.4 |
| `--ugoe-color-accent` | 主な操作・選択・スライダー・リンク | `#6d28d9` | `#a78bfa` | 6.5 / 7.1 / 5.6 | 6.5 / 5.8 / 6.9 |
| `--ugoe-color-on-accent` | 強調色の上の文字 | `#ffffff` | `#18181b` | 7.1 (強調色に対して) | 6.5 (強調色に対して) |
| `--ugoe-color-focus` | フォーカスの輪 | 強調色と同じ | 強調色と同じ | | |
| `--ugoe-color-danger` | エラー・取り消せない操作 | `#c81e1e` | `#f87171` | 5.2 / 5.7 / 4.5 | 6.4 / 5.7 / 6.8 |
| `--ugoe-color-warning` | 警告 | `#a14a06` | `#fbbf24` | 5.5 / 6.0 / 4.7 | 10.6 / 9.4 / 11.3 |
| `--ugoe-color-success` | 成功・接続中 | `#126c34` | `#4ade80` | 5.9 / 6.5 / 5.1 | 10.2 / 9.0 / 10.8 |
| `--ugoe-color-backdrop` | モーダルの後ろ | `rgb(0 0 0 / 40%)` | `rgb(0 0 0 / 55%)` | | |
| `--ugoe-color-hue-amber` | 分類の色: パーツ・深度からの回転・イベントの印 | `#b45309` | `#fbbf24` | 4.6 / 5.0 / 4.0 | 10.6 / 9.4 / 11.3 |
| `--ugoe-color-hue-cyan` | 分類の色: デフォーマ | `#0e7490` | `#22d3ee` | 4.9 / 5.4 / 4.2 | 9.8 / 8.7 / 10.4 |
| `--ugoe-color-hue-pink` | 分類の色: マスク | `#be185d` | `#f472b6` | 5.5 / 6.0 / 4.8 | 6.7 / 5.9 / 7.1 |
| `--ugoe-color-hue-teal` | 分類の色: 加算合成 | `#0f766e` | `#2dd4bf` | 5.0 / 5.5 / 4.3 | 9.5 / 8.4 / 10.1 |
| `--ugoe-color-hue-blue` | 分類の色: ミラー | `#1d4ed8` | `#60a5fa` | 6.1 / 6.7 / 5.3 | 7.0 / 6.2 / 7.4 |
| `--ugoe-color-hue-violet` | 分類の色: 導いたキー | `#7e22ce` | `#c084fc` | 6.4 / 7.0 / 5.5 | 6.7 / 5.9 / 7.1 |
| `--ugoe-color-hover` | hover の重ね (本文の色の 6%) | | | 本文 14.3 / 15.7 / 12.4、控えめ 5.1 / 5.6 / 4.4 | 本文 12.2 / 10.6 / 13.2、控えめ 6.0 / 5.2 / 6.5 |
| `--ugoe-color-active` | 押下の重ね (本文の色の 12%) | | | 本文 12.6 / 13.8 / 11.0 | 本文 10.3 / 8.9 / 11.2 |

- 状態の色 (`danger`・`warning`・`success`) は、文字と印 (アイコン) に使います。地に敷くときは、それぞれの `-soft` (`surface` に 12% 混ぜた色。`color-mix()`) を使い、
  文字は `--ugoe-color-text` のままにします。強調色の `--ugoe-color-accent-soft` も同じ作りです (選択した行・ドロップできる領域の地)。
  T0916 で状態の色の 3 つの `-soft` を `surface` に混ぜる形にそろえました (前は `bg` に混ぜていて、パネル (`surface`) の上の知らせだけ地が沈んでいた)。
  `-soft` の上の比 (ライト / ダーク): 本文 14.5〜14.9 / 9.5〜10.3、控えめな文字 5.2〜5.3 / 4.7〜5.1、それぞれの状態の色 4.7〜5.8 / 4.7〜7.2。
- **分類の色 (`hue-*`):** 種類を見分けるための色 (エディタのツリーの節の種類・キーの由来の凡例・タイムラインのイベントの印) で、状態の意味を持ちません
  (エラー・警告には使わない)。T0916 で、エディタに直に書いていた色 (ライトで `bg` に 2.9 しかなかった `#d97706` など) をこれに移しました。
  どれも `bg`・`surface` に 4.5 以上なので文字にも使えます (`sunken` の上は印だけ)。色だけで種類を伝えず、印の形・記号・名前を並べます。
- **hover と押下の重ね:** `--ugoe-color-hover`・`--ugoe-color-active` は半透明の色で、部品は `background-image: var(--ugoe-overlay-hover)`
  (`--ugoe-overlay-active`) で地の上に重ねます。地の色 (`surface`・`sunken`・強調色・透明) を書き換えないので、どの種類の部品でも
  「一段濃く (ダークでは明るく)」が同じに見えます。比は `bg` / `surface` / `sunken` に重ねたときの値で、強調色に重ねても
  `on-accent` は 6.8 以上です。`sunken` の上で hover した控えめな文字は 4.4 なので、控えめな文字を持つ部品 (`Tabs` の項目) は hover で文字を本文の色にします。
- 強調色は 3 つのアプリ (エディタ・ビューア・スタジオ) でエディタの紫にそろえます。ビューアの青は使いません (利用者の判断。ADR 0029 の 5)。
- 前のエディタの `--muted` (`#71717a`) は、`--bg` の上で 4.4 で基準 (4.5) に届かなかったので、`--ugoe-color-text-muted` に置き換えました。
- 文字を薄く見せるのに `opacity` を使いません (地との比が下がって 4.5 に届かない)。控えめにするなら `--ugoe-color-text-muted` にします (T0907)。

### ビューポートの上の図

エディタのビューポートの上に重ねる図 (選択の輪郭・メッシュの辺と点・ギズモ・物理・グルー・光源・はみ出し・違いの強調) の色は、
`@ugoe/ui` のトークンではなく、`apps/editor/src/lib/viewport/Viewport.svelte` の `.viewport` に置く CSS の変数 (`--viewport-*`) から取ります (T0675)。
図はモデルの色の上にも、ライトとダークのビューポートの地 (`sunken` の `#e4e4e7`・`#111113`) の上にも描くので、**テーマで変えません**。

- 線の色は、両方の地に 3:1 以上の中ほどの明るさ (相対輝度 0.12〜0.22) から選びます。モデルの上は色が決まらないので、
  点は **白の中と線の色の縁** で描きます (明るい地では縁が、暗い地とモデルの上では中が見える)。選んだ点は琥珀の中と濃い縁で、形 (塗り) も変わります。
- 黄色の光源は明るい地に弱い (1.2) ので、図の全体に暗い縁取り (`--viewport-halo` の `drop-shadow`) を付けます。
- 前は強調色 (`accent`。テーマで変わる)・`surface` の塗り (ダークでは点が暗い)・`opacity` で薄くした線と、場所ごとの直書きの色でした。

| 変数 | 用途 | 値 | 比 (ライトの地 / ダークの地 / 白) |
| --- | --- | --- | --- |
| `--viewport-edit` | 選択の輪郭・メッシュの辺と点の縁・フォーム編集・矩形選択 | `#8b5cf6` | 3.3 / 4.5 / 4.2 |
| `--viewport-warp` | ワープの格子と制御点の縁 | `#0284c7` | 3.2 / 4.6 / 4.1 |
| `--viewport-rotation` | 回転のギズモ | `#db2777` | 3.6 / 4.1 / 4.6 |
| `--viewport-base` | モジュールのプロジェクトの本体の要素の輪郭 (破線) | `#71717a` | 3.8 / 3.9 / 4.8 |
| `--viewport-point` | 点の中 | `#ffffff` | |
| `--viewport-picked` / `-picked-edge` | 選んだ点の中と縁。グルーのパネルで指している組の線と両端 (T1651) | `#f59e0b` / `#92400e` | 縁 5.6 / 2.7 (暗い地では明るい中が 8.8) |
| `--viewport-glue-a` / `-glue-b` / `-glue-pair` | グルーの 2 つのメッシュと頂点の組 | `#15803d` / `#9333ea` / `#d9480f` | 4.0 / 3.8、4.2 / 3.5、3.4 / 4.4 |
| `--viewport-collider` | 物理の衝突体 | `#2563eb` | 4.1 / 3.6 / 5.2 |
| `--viewport-chain` / `-chain-selected` | 物理の粒子の鎖と、選んだ鎖 | `#d9480f` / `#dc2626` | 3.4 / 4.4、3.8 / 3.9 |
| `--viewport-overflow` | はみ出しの印 | `#dc2626` | 3.8 / 3.9 / 4.8 |
| `--viewport-highlight` | プレビューの違いの強調 | `#d9480f` | 3.4 / 4.4 / 4.3 |
| `--viewport-light` | 光源 (暗い縁取り付き) | `#facc15` | 1.2 / 12.3 (縁取りで補う) |
| `--viewport-ink` / `--viewport-halo` | 色の付いた取っ手の縁 / 縁取り | `#1f2937` / `rgb(0 0 0 / 0.6)` | |

深度の点 (`DepthOverlay`) の色は深度の値から作る段階の色で、この表の外です。ビューアの図 (`LightMarkers`・`LightPad`) はビューアの中で決めています。

### 余白

4px を単位にします。`compact` と `comfortable` で同じ名前のまま値が変わります。

| トークン | compact | comfortable | 主な用途 |
| --- | --- | --- | --- |
| `--ugoe-space-1` | 2px | 2px | アイコンと文字の間 |
| `--ugoe-space-2` | 4px | 6px | 部品の中の詰めた余白・行の間 |
| `--ugoe-space-3` | 8px | 10px | ボタンの左右・部品の間 |
| `--ugoe-space-4` | 12px | 16px | パネルの内側・段落の間 |
| `--ugoe-space-5` | 16px | 20px | ダイアログの内側 |
| `--ugoe-space-6` | 24px | 32px | 大きな区切り |
| `--ugoe-control-height` | 24px | 30px | ボタン・入力欄・Select の高さ |
| `--ugoe-icon-size` | 16px | 18px | アイコン・チェックボックスの箱・色の見本 (下の「アイコン」) |

### 文字

| トークン | compact | comfortable | 用途 |
| --- | --- | --- | --- |
| `--ugoe-font-size-xs` | 11px | 12px | 見出しの小さな大文字・単位 |
| `--ugoe-font-size-sm` | 12px | 13px | 補足 |
| `--ugoe-font-size-md` | 13px | 15px | 本文・部品 (基準) |
| `--ugoe-font-size-lg` | 15px | 17px | ダイアログの見出し |
| `--ugoe-font-size-xl` | 18px | 21px | 画面の見出し |

- `--ugoe-font-sans`: `system-ui, -apple-system, "Segoe UI", "Hiragino Sans", "Noto Sans JP", sans-serif`
- `--ugoe-font-mono`: `ui-monospace, "Cascadia Mono", "SF Mono", "Noto Sans Mono", monospace` (数値の欄・パス・コード)
- `--ugoe-line-height`: 1.5 (本文・説明の段落・知らせ)。部品の 1 行の中は `--ugoe-line-height-control` (1.2)。
  T0916 で 1.4 から 1.5 にしました (日本語の説明が折り返すと行が詰まって見えたため。T0915 の監査)。
  エディタは根元を `--ugoe-line-height-control` にし (密な行のため)、文 (`p`・`li`・`dd`・`.ed-hint`) だけを `--ugoe-line-height` にします。
  前は根元に行の高さが無く (ブラウザの `normal`)、説明の段落が折り返すと行が重なりかけていました。ビューアとスタジオは根元から `--ugoe-line-height` です。
- 太さは `--ugoe-font-weight-normal` (400) と `--ugoe-font-weight-strong` (600) の 2 つだけ使います。数値が並ぶ欄には `font-variant-numeric: tabular-nums` を付けます。
- 日本語では `text-transform: uppercase` と字間の指定が効かない・不自然になるので、見出しの装飾は言語に依らない形 (色と大きさ) で付けます。

### 角丸・影・重なり・動き

| トークン | 値 | 用途 |
| --- | --- | --- |
| `--ugoe-radius-sm` | 3px | チェックボックス・小さなボタン |
| `--ugoe-radius-md` | 4px | ボタン・入力欄・Select |
| `--ugoe-radius-lg` | 8px | ダイアログ・メニュー・知らせ |
| `--ugoe-radius-full` | 999px | トグル・バッジ |
| `--ugoe-shadow-overlay` | `0 4px 16px rgb(0 0 0 / 20%)` (ダークは 50%) | メニュー・ツールチップ・ダイアログ |
| `--ugoe-z-popover` / `--ugoe-z-toast` | 100 / 200 | `popover` と `<dialog>` は最上位の層 (top layer) に出るので、z-index を使うのは知らせの帯などだけ |
| `--ugoe-duration-fast` / `--ugoe-duration-normal` | 100ms / 200ms | 開閉・ホバー。`prefers-reduced-motion: reduce` では 0ms |
| `--ugoe-focus-ring` / `--ugoe-focus-offset` | `2px solid var(--ugoe-color-focus)` / `2px` | `:focus-visible` の `outline` と `outline-offset` |
| `--ugoe-overlay-hover` / `--ugoe-overlay-active` | `--ugoe-color-hover` / `-active` の一色の `linear-gradient` | hover と押下の地の重ね (`background-image`) |
| `--ugoe-opacity-disabled` | 0.5 | 無効の部品の全体に掛ける |
| `--ugoe-target-min` | 24px | 押せる部品の最小の大きさ (WCAG 2.5.8)。密度に依らない |

- 枠の線は 1px (部品の枠・区切り)、選んだタブの下線と知らせの左の帯だけ太く (2px・4px) します。線の太さのトークンは作りません (値が 3 つしかなく、意味が場所で決まるため)。
- 影は重ねて出すもの (メニュー・ツールチップ・ダイアログ・Toast) の `--ugoe-shadow-overlay` だけです。パネルやカードに影を付けず、境は `--ugoe-color-border` の線で示します。
- 角丸は大きさで選びます: 部品の中の小さなもの (チェックボックス・一覧の行・メニューの項目) は `sm`、部品 (ボタン・欄) は `md`、
  重ねて出すもの・知らせ・まとまりの枠は `lg`、丸いもの (トグル・札) は `full`。

### アイコン

アイコンは `@ugoe/ui` の `Icon` (`<Icon name="close" />`) を使います (T0916)。3 つのアプリで同じ意味の印を同じ形にするためで、
閉じる・消す (`close`)・上下へ動かす (`arrowUp`・`arrowDown`)・読み直す (`reload`)・再生と停止 (`play`・`stop`)・コマ送り (`stepBack`・`stepForward`)・
キー (`key`)・開閉 (`chevronRight`・`chevronDown`)・拡大と縮小 (`plus`・`minus`)・合わせる (`fit`)・設定 (`settings`) と、知らせの種類の印があります。
`Icon` に無いもの (1 か所でしか使わない印) を描くときは、16 の格子の `viewBox="0 0 16 16"` に線で描き、`tokens.css` の `.ugoe-icon` を付けます。
2 つ以上のアプリで要るものになったら `Icon` に足します。

```svelte
<IconButton label={t("dressUp.remove.aria")} onclick={remove}><Icon name="close" /></IconButton>
<svg class="ugoe-icon" viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8h9M8 3.5v9" /></svg>
```

- **大きさ:** `--ugoe-icon-size` (compact 16px・comfortable 18px)。`IconButton` の中も同じです。領域の目印になる大きな印 (`DropZone`・空の状態) だけ 2 倍にします。
- **線:** 太さは `--ugoe-icon-stroke` (1.5。格子の単位なので、表示の大きさに比例する)、端と角は丸めます (`.ugoe-icon` が付ける)。
  `stroke-width` などを要素に直に書きません。線では形が弱いもの (再生の三角・停止の四角) は `.ugoe-icon--fill` で塗ります。
- **色:** `currentColor` (文字の色に従う)。意味を伝えるアイコンは地に 3:1 以上にします (`text-muted` はどの地にも 5.0 以上)。
- **揃え:** 文字と並べるときは `display: inline-flex; align-items: center` の並びに入れ、文字の中心の線にそろえます。アイコンと文字の間は `--ugoe-space-1`〜`--ugoe-space-2`。
- **例外:** チェックボックスの箱の中の印 (箱より 2px 小さく、線 2) と、ビューポートの上の図 (ギズモ・光源の印。描画の中身でトークンの外) です。
  エディタのツリーの節の種類の印 (`NodeIcon`) は行の高さに収めるため `--ugoe-icon-size` より小さい 14px で描き、線の太さだけそろえます。
- **エディタの右の列のタブ (`PaneIcon`、T1632):** 8 つのタブを 1 行に収めるため、名前を見せずにアイコンだけにしています (名前は読み上げの名前と
  Tooltip)。エディタでしか使わないので `Icon` には置きません。絵は「そのタブで扱うもの」をそのまま描き、記号的な意味 (歯車 = 設定など) と
  ぶつからないものを選びました。初めは Tooltip で覚えてもらう前提で、Tooltip には名前と何をするタブかを並べます。

  | タブ | 絵 | 選んだ理由 |
  | --- | --- | --- |
  | Inspector | 3 本のスライダー | 選んだ要素の値を調べて変える場所。「プロパティ」の印として広く使われる形で、歯車 (`settings` = アプリの設定) と分ける |
  | Physics | 支点から下がって左右に揺れる振り子 | ugoe の物理は振り子のリグ ([docs/spec/physics.md](spec/physics.md))。髪や服が揺れる動きを、両側の弧で示す。リンゴやバネは物理の印として分かりやすいが、このタブで作るもの (揺れるリグ) と合わない |
  | Glue | 2 つの輪をつないだ鎖 | グルーは 2 つのメッシュの継ぎ目を「つなぐ」もの。接着剤のチューブは 16 の格子では潰れて読めない。リンクの印と同じ形だが、エディタに URL のリンクを作る操作は無いのでぶつからない |
  | Lighting | 太陽 | 光源とライティング。電球は「ヒント・アイデア」の印と紛れる |
  | Slots | ハンガー | 着せ替えのスロット = 衣装や小物のモジュールを「掛ける」場所。服の絵 (T シャツ) にすると衣装そのもの (モジュール) と紛れる |
  | Poses | 片腕を上げた人 | ポーズのグループは腕の A / B のような切り替える変形。左右の腕を違う向きにして「ポーズを変える」ことを示す |
  | Verify | チェックの入った虫眼鏡 | はみ出しを「調べて確かめる」。チェックだけでは「完了・成功」(`success`) と紛れるので虫眼鏡と重ねる |
  | AI | 4 つの尖りの星と小さな + | AI の機能の印として広く使われる形 (きらめき)。提案の数はアイコンの右に数字、見てほしい変化は右上の点 |

- 文字の記号 (◆・▾・▶・×) をアイコンの代わりに使うのは、タイムラインのキーとイベントの印のような図の中と、文の中の記号 (「× 3」) だけにします。
  T0916 で、エディタのボタンの中の記号 (×・✕・▲・▼・↻・◀・▶・■・◆・▾・▸) を `Icon` に、ビューアとスタジオの手描きの SVG を `Icon` に置き換えました。

### 密度の使い分け

| 密度 | アプリ | 向く画面 | 約束 |
| --- | --- | --- | --- |
| `compact` | エディタ | 欄と行が多く、1 画面に多くを並べて見比べる (インスペクタ・ツリー・タイムライン・パラメータ) | 行の高さは 24px (`--ugoe-target-min`) を下回らない。余白は `space-2`〜`space-3` を主に使い、パネルの内側は `space-4` |
| `comfortable` | ビューア・スタジオ | 操作が少なく、配信中や見るだけのときに離れて・急いで押す | 部品は 30px。パネルの操作の並び (シーンの追加・複製・並べ替え、モジュールの追加) に `small` のボタンを使わない。`small` は一覧の行の中 (行の並べ替え・消す)・見出しの右 (「止める」「すべて解除」)・値の隣 (パスと「選ぶ…」)・知らせの中の操作だけにする |

- 1 つのアプリの中で密度を混ぜません (ダイアログ・メニューも `<html>` の密度に従う)。スタジオの出力ウィンドウは部品を持たないので密度の外です。
- 密度で変わるのはトークンの値だけです。部品やアプリの CSS で `[data-density]` を見て値を切り替えません (変えたいなら、トークンを足す)。
- `small` のボタン (`--ugoe-target-min` の高さ・`font-size-sm`) は、エディタでは行の中と見出しの右、ビューアとスタジオでは上の場所に限ります。
  T0916 で、スタジオのシーンの一覧の操作の並びと、シーンの編集の「モジュールを足す」「配置を戻す」を通常の大きさにしました (T0915 の監査)。

## 部品の一覧

`@ugoe/ui` の部品は、ふるまいの土台を [Bits UI](https://bits-ui.com/) 2.x (ヘッドレス) に任せ、見た目をトークンで付けます ([ADR 0029](adr/0029-shared-ui-and-i18n.md) の 5)。
**アプリと、`@ugoe/ui` 以外のパッケージは Bits UI を直接 import せず、`@ugoe/ui` の部品を使います。** Bits UI に無い部品は、Bits UI の部品の組み合わせか
HTML の標準の要素の上に自作し、ふるまいは [WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/) のパターンに従います。

土台の列は元にするもの (Bits UI の部品名、または自作の元にする要素)。キーボードの列は部品にフォーカスがあるときの操作で、Bits UI の部品はその実装に従います。
段は実装するタスク (T0902 = 1 段目、T0903 = 2 段目)。

| 部品 | 使う場面 | 土台 | キーボード | 段 |
| --- | --- | --- | --- | --- |
| `Button` | 操作を 1 つ実行する。種類は `primary` (画面に 1 つまで)・`secondary` (既定)・`danger`・`ghost` (ツールバーの中)。`form` (フォームの id) でフォームの外から送れる (ダイアログの `actions` に置いた決定のボタンなど) | Bits UI `Button` | Enter・Space | 1 |
| `IconButton` | 文字を出す場所が無いときの操作。**`label` (読み上げとツールチップに使う) を必ず渡す** | Bits UI `Button` と `Tooltip` | Enter・Space | 1 |
| `Checkbox` | 決定 (保存・OK) を押すまで効かない選択肢、または一覧の中の選択 | Bits UI `Checkbox` | Space | 1 |
| `Toggle` | すぐに効く入・切 (「鏡像で表示」など)。説明は `tooltip` (スイッチに Tooltip を付ける) | Bits UI `Switch` | Space・Enter | 1 |
| `Slider` | 範囲のある数値。`NumberField` とリセット (既定値に戻す) を並べて出せる (この組み合わせは自作)。説明は `tooltip` (つまみに Tooltip を付ける) | Bits UI `Slider` | 矢印・PageUp/PageDown・Home/End | 1 |
| `NumberField` | 数値を正確に入れる。上下の矢印で増減、Shift で 10 倍。単位を後ろに出す。誤りは `invalid`、説明は `tooltip`。空 (既定を使う) を持てる欄は `optional` (下の「空を持てる数値の欄」)。打つたびに読めた値を知りたいときは `oninput` | 自作 (`<input type="text" inputmode="decimal">`、APG の Spinbutton) | 上下の矢印・Enter で確定・Esc で戻す | 1 |
| `TextField` | 1 行の文字 (名前・ID・URL・検索。`type` は `text`・`search`・`url`)。見えるラベル (`label`)・補足 (`hint`)・誤りの文 (`error`) を欄と結んで出す。ID・パスは `mono`、説明は `tooltip`。複数行は `TextArea`、秘密は `SecretField` | 自作 (`<input>`) | Enter・フォーカスを外すと確定 (`onchange`)・Esc で確定前に戻す | T0917 |
| `TextArea` | 複数行の文字 (名前の一覧・注文の文など)。ラベル・補足・誤りの文・`tooltip`・確定の約束は `TextField` と同じ。綴りの印は既定で出す (`spellcheck`) | 自作 (`<textarea>`) | Enter は改行・Mod+Enter かフォーカスを外すと確定 (`onchange`)・Esc で確定前に戻す | T0949 |
| `SecretField` | API キーなどの秘密 (`TextField` と同じ見た目。秘密の欄はこの部品だけで作る)。値を Svelte の状態に持たない: 呼び手は `bind:filled` (空白を除いて空でないか) だけを受け、送るときに `bind:this` の `take()` で取り出す (取り出すと欄を空にする)。捨てるのは `clear()` | 自作 (`<input type="password">`) | Enter は標準のまま (`<form>` なら送る)・Esc で入れかけを消す | T0949 |
| `Select` | 5 個以上の選択肢から 1 つ。誤りは `invalid` (選べなくなった値など)、説明は `tooltip` | Bits UI `Select` | 上下の矢印・Enter・頭文字で移る | 1 |
| `SegmentedControl` | 2〜4 個の選択肢から 1 つ (すべて見せたいとき)。名前と選択肢は字の途中で折らず、幅が足りなければ選択肢ごと次の行へ送る (T0950) | Bits UI `RadioGroup` (または `ToggleGroup` の単一選択) | 矢印で移り、選ぶ | 1 |
| `Tabs` | 同じ場所の中身を切り替える (中身が別の画面のとき) | Bits UI `Tabs` | 左右の矢印・Home/End | 1 |
| `Section` | 折りたためるまとまり (パネルの中) | Bits UI `Collapsible` | Enter・Space | 1 |
| `Toolbar` | ボタンの並び。中は 1 つのタブ止まりにする | Bits UI `Toolbar` | 左右の矢印 (roving tabindex) | 1 |
| `Dialog` | 作業を止めて確認・入力させる (モーダル)。取り消せない操作の確認は `AlertDialog`。開いたときのフォーカスは中の最初の部品で、`initialFocus` (要素の id) で選べる (その部品の `Tooltip` は開いたときには出さない。最初の Esc で閉じるように) | Bits UI `Dialog`・`AlertDialog` | Esc で閉じる・中でフォーカスが回る・閉じたら開いた元にフォーカスを戻す | 2 |
| `Notice` | 画面に残る知らせ (読み込みの警告など)。`ondismiss` を渡すと閉じるボタンを出す | 自作 (`role="status"`。エラーは `role="alert"`) | 閉じるボタンに届く | 2 |
| `Toast` | すぐ消えてよい知らせ (「保存しました」)。操作を含むなら `Notice` にする。`showToast()` で出し、アプリの根元の `Toaster` が描く | 自作 (`role="status"`、`popover="manual"`) | フォーカスを奪わない。ポインタやフォーカスが乗っている間は消えない。`Toaster` は `announce()` の見えない領域も持つ ([状態の変化の知らせ](#状態の変化の知らせ)) | 2 |
| `Tooltip` | ボタンやアイコンの短い説明。`IconButton` は `label` を既定で出す。根元に `TooltipProvider` を置くと、続けて出すときに待たない | Bits UI `Tooltip` | フォーカスでも出る・Esc で消える | 2 |
| `Menu` / `ContextMenu` | 操作の一覧 (ファイルのメニュー・「その他」のボタン / 右クリック)。選択肢を選ぶなら `Select`。項目にショートカットを出せる (`aria-keyshortcuts` も付く)。名前の下に説明の行 (`description`。項目の説明として読む) を出せる (見本の中身・最近のファイルの場所など) | Bits UI `DropdownMenu`・`ContextMenu` (メニューバー (`Menubar`) は使う画面ができたときに足す) | 上下の矢印・Enter・Esc・頭文字で移る | 2 |
| `Popover` | 部品の近くに出す小さな編集の枠 (色の選択など) | Bits UI `Popover` | Esc で閉じ、開いた元に戻る | 2 |
| `ColorPicker` | 光源などの色 | 自作 (`Popover` の中に `<input type="color">` と `NumberField`) | 標準の操作 | 2 |
| `DropZone` | ファイルを落とす領域。**同じことができるボタン (「ファイル…」) を必ず並べる** | 自作 (`<div>` と `Button`) | ボタンで開く | 2 |
| `Icon` | 線のアイコン (閉じる・上下・再生・停止・キー・設定など、名前で選ぶ)。ボタンの中の印に文字の記号を使わずこれを使う (上の「アイコン」) | 自作 (`<svg>`) | | T0916 |
| `Kbd` | ショートカットの表記 (macOS は ⌘、ほかは Ctrl) | 自作 (`<kbd>`) | | 2 |
| `ThemeSetting` | テーマの設定 (OS に合わせる・ライト・ダーク)。選ぶとすぐに切り替えて覚える | 自作 (`SegmentedControl`) | 矢印で移り、選ぶ | T0909 |

`@ugoe/ui` の外の、パッケージの部品 (中は上の部品で作り、ugoe のデータの形を受け取る):

| 部品 | パッケージ | 使う場面 | 使うアプリ |
| --- | --- | --- | --- |
| `TrackingPanel` | `@ugoe/face-tracking` | フェイストラッキングの入力・調整・記録。高さの上限と中の送りは持たない (置く側が決める。T0951) | ビューア・スタジオ・エディタのプレビュー |
| `PosePicker` | `@ugoe/player` | ポーズのグループごとに表示する要素を選ぶ (要素が 2〜4 個なら `SegmentedControl`、5 個以上なら `Select`)。`shown` をそのまま出すので、呼び手が表示中の要素に合わせれば追従し、`onchange` が Promise を返す間は選んだ要素を先に出す (T0948)。見出しと要素の表示名は `groupLabel`・`entryLabel` で差し替えられる (T1640) | ビューア・スタジオ・エディタのプレビュー |

Bits UI を包むときの約束:

- Bits UI の部品の `class` と `data-*` 属性 (`data-state`・`data-disabled` など) にトークンを当てて装います。Bits UI の既定の見た目には頼りません (持たない)。
- 読み上げの名前・閉じるボタンの名前など、部品の中の文言は `@ugoe/i18n` から引いて渡します。日付を扱う部品を使うときは `locale` に今の言語を渡します。
- `@ugoe/ui` の部品の props は ugoe の言葉で決め、Bits UI の props をそのまま外に出しません (Bits UI の破壊的な変更を `@ugoe/ui` の中で吸収するため)。
- 読み上げの名前と説明の props は、HTML の属性と同じ名前 (`aria-label`・`aria-labelledby`・`aria-describedby`) で受けます。
  `Button`・`IconButton` はトリガーの属性を広げて受けるので、ほかの部品もこれに合わせます (T0920。`ariaLabel`・`labelledby`・`describedby` とは書かない)。
- トリガーを呼び手が描く部品 (`Tooltip`・`Popover`・`Menu`) は、そのスニペットを `trigger` と呼び、受け取った属性をフォーカスを受ける要素に広げます。
- `index.ts` は部品と、アプリやほかのパッケージが使う関数・型だけを名前で出します (`export *` は使わない)。部品の内部の関数
  (数値の欄の解釈・ファイルの選り分け・テーマの読み書き・Toast の列など) は出さず、アプリで要るものになったら名前を足します。
- Bits UI の版は `@ugoe/ui` で完全に固定し、人が更新します (ADR 0029 の 5)。

使い分けの約束:

- **`title` 属性で説明を出さない**のを基本にし、`Tooltip` を使います (`title` はキーボードで出せず、タッチでも出ない)。
  欄の説明は部品の `tooltip` (`Toggle`・`Slider`・`NumberField`・`Select`・`TextField`) で、押せない理由・読み方の説明は見える補足で出します。
  T0917 の後に残している `title` は、フォーカスを受けない要素の補い (省略した名前の全体・ID・一覧の行の印・図の印) だけです
  (一覧は下の「足りない部品と口 (T0917)」)。同じことを見える文や読み上げの名前で伝えていて、`title` が無くても操作に困らないものに限ります。
- **チェックボックスとトグル:** 押した瞬間に効くならトグル、決定のボタンを押すまで効かないならチェックボックス。
- **`Dialog` と `Notice`:** 利用者の判断が要らない知らせにダイアログを使いません。
- **取り消せない操作** (すべて忘れる・上書き保存の前の確認など) は `danger` のボタンにし、確かめのダイアログを出すか、Undo を用意します。確かめのダイアログでは、既定のフォーカスを取り消し側に置きます。
- **無効のボタン**は、なぜ押せないかを近くに書くか、`Tooltip` で出します (無効のボタンはフォーカスを受けないので、説明は `aria-describedby` で別の要素から付ける)。
- パネルの見出し・行のレイアウトなど、エディタの中の共通の形は下の「エディタの中の共通の形」にあります。

## エディタの中の共通の形

エディタのパネルとダイアログを `@ugoe/ui` に移すとき (T0663・T0664・T0665) にそろえる形です。エディタの中にしか無いものは
`apps/editor/src/lib/ui/` と `apps/editor/src/app.css` に置き、2 つ以上のアプリで要るものになったら `@ugoe/ui` へ移します。

| 形 | 使うもの | 約束 |
| --- | --- | --- |
| パネルの見出し | `<h2>` (`app.css`) | `--ugoe-font-size-sm`・太さ 600・`--ugoe-color-text-muted`、上 `--ugoe-space-4`・下 `--ugoe-space-2`。大文字化と字間は使わない (日本語で効かない)。パネルの中のまとまりは `<h3>` (`app.css`。`--ugoe-font-size-md`・600・本文の色)。折りたためるまとまりは `Section` |
| 行 (欄の名前と部品) | `.ed-field` (`app.css`) | 2 列の格子 (名前 `minmax(5em, 2fr)`・部品 `3fr`)、行の間 `--ugoe-space-2`。名前は折り返してよく、固定の幅を付けない (英語は長くなる)。名前は `<label for>`、または名前の `<span id>` と部品の `aria-labelledby`。単位は部品の後ろ (`NumberField` の `unit`)。`Select` は部品の列の幅に広げ、`NumberField` は桁の幅のまま。1 つの値の欄はパネルを問わずこの行にし、部品の `label` で名前を横に出す形 (`<Select label>`) を混ぜない。名前を見せる部品 (`Slider`・`Checkbox`・`Toggle`) と、組の値 (最小・既定・最大、行数と列数、x・y・z) は行に入れずにそのまま並べる |
| ボタンの並び | `.ed-buttons` (`app.css`) | 横に並べて折り返す (間 `--ugoe-space-2`)。決定は右端ではなく並びの最後 (パネルの中は左寄せ)。押せない理由は近くに書く |
| 補足・空の状態 | `.ed-hint` (`app.css`) | `--ugoe-font-size-sm`・`--ugoe-color-text-muted`。空の状態は「何が無いか」と「次に何をするか」を 1 文で |
| 欄の説明 | `.ed-help` (`app.css`) | 欄 (`.ed-field`) のすぐ下に置き、`aria-describedby` で欄と結ぶ。上は `space-1`、下は `space-3` (説明は上の欄の仲間に見える)。パネルの頭の説明も同じ形 |
| 注記 | `.ed-note` (`app.css`) | 本体のもの (モジュールのプロジェクト) など、パネルや欄の組の頭の `role="note"`。左に 3px の線、控えめな文字。`BaseLock` の文もこれ |
| 枠で囲むまとまり | `.ed-card` (`app.css`) | 選んだものの編集の欄 (スロット・グルー)・追加の欄 (新しいグルー・新しいグループ)・ポーズのグループのように、一緒に変える欄の組だけ。見出しは `<h3>`。枠の中に枠を重ねない (中の一覧・選ぶ欄の枠は例外の 1 段まで) |
| 一覧の行 | `.ed-item` (`app.css`) | 押すとそれを選ぶ行のボタン (スロット・リグ・衝突体・グルー・ポーズの要素・光を受けるメッシュ)。高さ 24px 以上、hover は重ね、選択は `accent-soft` の地と強調色の枠。行の中の補足は `.muted`、札は `.ed-tag`。並べ替え・消すは行の右に `small` の `IconButton` |
| 札 | `.ed-tag` (`app.css`) | 行の印 (「本体」「app」「先頭」「relay」など)。`font-size-xs`・枠。意味のある札は `.accent`・`.danger`。説明は Tooltip (ツリーの行は `title`) |
| 足すボタン | `Button`・`TipButton` と `Icon name="plus"` | 「+ Rig」のような足す操作は、文言の頭に「+ 」を書かず、`<Icon name="plus" />` を文字の前に置く (文言は名詞のまま。`Button` の中の間は部品が取る)。文書では「+ Rig」と書いてよい (画面の見た目と同じ) |
| 文のように見せるボタン | `.ed-link` (`app.css`) | 要素の ID (着せ替えの違いの一覧)・ライセンスの口など、文の中に置く操作。下線と強調色、押せる高さ 24px、フォーカスの輪 (T0676) |
| 説明付きのボタン | `TipButton` (`lib/ui/TipButton.svelte`) | `title` 属性の代わりに `Tooltip`。ショートカットがあれば `shortcut` (`Mod+Z`。実際に効くものだけ) |
| タブの並び | `TabStrip` (`lib/ui/TabStrip.svelte`) / `Tabs` | 中身を部品が持てるなら `Tabs`。右の列のように、タブの状態が別々の表示の状態に分かれているときは `TabStrip` (APG の Tabs。矢印・Home・End、説明は Tooltip、見てほしい変化は `attention` で色・太さ・印と読み上げ)。`icons` でアイコンだけのタブ (右の列。読み上げの名前は `aria-label`、名前は Tooltip にも出し、数は `badge`) |
| 失敗・知らせ | `Notice` / `showToast` | 操作の失敗はその場所に `Notice` (`danger`。閉じられる)。結果 (「保存しました」) は Toast。ヘッダーの操作の失敗はヘッダーの下の知らせの帯 (`FileMenu` の `.bands`) |
| 本体のもの (モジュールのプロジェクト) | `BaseLock` | 欄をまとめて無効にし、直す場所を `role="note"` の文で示す。`fieldset` の `disabled` は `<input>`・`<button>` にしか効かないので、`Slider` など中にそれらを持たない部品には `useBaseLock()` (`lib/ui/baseLock.ts`) の値を `disabled` に渡す |
| 確かめ | `AlertDialog` | 取り消せない操作・保存していない変更を捨てる操作。同じボタンを 2 度押させる形にしない |
| ダイアログ | `Dialog` | 見出しは `title`、説明は `description`、決定・取り消しは `actions`。Esc で閉じるときの意味 (「後で」など) を `onclose` で決める。ボタンの並び・既定のボタン・Enter と Esc は [docs/editor.md の「ダイアログ」](editor.md#ダイアログ) |
| 区切り | `Splitter` (`lib/ui/Splitter.svelte`) | 領域の大きさを変える境 (APG の Window Splitter)。ポインタのドラッグと、矢印・Home・End、Enter とダブルクリックで既定に戻す。範囲と覚えるキーは `lib/layout.ts` の `SizeRange` ([docs/editor.md の「画面の配置」](editor.md#画面の配置)) |
| 空の状態の次の操作 | `.ed-hint` と `Button` | 「何が無いか」の 1 文と、次の操作のボタンをその場に並べる (ビューポートの空のプロジェクト・パラメータ一覧) |
| 文字の入力欄 | `TextField`・`TextArea`・`SecretField` | 行の左に名前を置くときは `<label for>` と `id`、ほかは `label`。確定で送る欄は `onchange` (Enter・フォーカスを外す。`TextArea` は Mod+Enter・フォーカスを外す)。空 (既定) を持てる数値は `NumberField` の `optional`。標準の `<input>` のまま残すものは下の「足りない部品と口 (T0917)」 |
| 要素の名前 | `src/lib/labels.ts` | `顔 (Layer8)` の形 (名前と ID)。ID・パス・パラメータの名前は訳さない |

- **文言の置き場所:** エディタのカタログ (`apps/editor/locales/<言語>.json`、名前空間 `editor`) のキーの最初の段は、画面の部分 (`app`・`fileMenu`・
  `moduleSettings` など) か、パネルの名前 (`inspector`・`physics`・`timeline` など) にします。部品から作る文 (`moduleProject.ts` など) は
  関数にして、呼ぶたびに `t()` を通します (モジュールの読み込み時に 1 度だけ評価した定数は言語の切り替えに追従しない)。
- **`<dialog>` と body に出す部品:** `showModal()` の `<dialog>` は最上位の層 (top layer) に出るので、その中で `Select`・`Popover`・`Menu`
  (body に出す) を使うと一覧が後ろに隠れます。ダイアログを `@ugoe/ui` の `Dialog` に移してから使います。移すまでは `SegmentedControl` などの
  body に出さない部品にします。
- **重なるダイアログ:** `Dialog` の上に `Dialog`・`AlertDialog` を開いてよいです (設定の中の「接続を確かめる」から送る前の確認を開くなど。T0665)。
  Bits UI のフォーカスの閉じ込めと Esc・外のクリックの層は全体で重なった順に積まれるので、上のものだけが反応し、閉じると下に戻ります。
  ただし、開いた元のボタンを待つ間に無効にすると、閉じたときにフォーカスを戻せないので、押せるようになってから戻します (`SettingsDialog` の「接続を確かめる」)。

### 数値の部品と onchange

`Slider` と `NumberField` (と `Slider` の中の数値の欄) は、**利用者の操作のときだけ** `onchange` (と `Slider` の `oncommit`) を呼びます (T0910)。

- 利用者の操作は、つまみのドラッグ・キー (矢印・PageUp / PageDown・Home / End)・欄に入れて確定 (Enter・フォーカスを外す)・リセットのボタンです。
- 外から渡した `value` では呼びません。刻み (`step`) の格子や `decimals` の桁に乗らない値 (指揮役が送る呼吸の値など) を渡すと、
  つまみは格子に寄せて、欄は丸めて**表示だけ**します。渡した値はそのまま残り、寄せた値を書き戻しません。
- 数値の欄は、書き換えていなければ確定しません (フォーカスを外す・Enter だけでは呼ばない)。書き換えていない間は、フォーカスがあっても外からの値に追従し、
  書き換えの途中は外からの値で欄を上書きしません。
- そのため、毎フレーム変わる値を映すだけのスライダーでも、利用側で「変化が小さければ送らない」などの回避は要りません。
- **確定の印:** `NumberField` は矢印キー・PageUp / PageDown の 1 回ごとに `onchange` を呼びます (押し続けると何度も呼ぶ)。
  呼んでいる間だけ、`numberFieldCommit()` が欄ごとに一意な印 (文字列) を返します (それ以外では `null`)。受け手はこれで同じ欄の続けての確定を
  見分けてまとめられます。エディタは 1 つの Undo にまとめます ([editor.md#undo](editor.md#undo)、T0695)。`onchange` の中で `await` した後は
  印が無いので、読むなら同期的に読みます。ビューア・スタジオは印を読まず、振る舞いは変わりません。

### 空を持てる数値の欄

`NumberField` に `optional` を渡すと、値に空 (`null`) を持てます (T0949)。空は「既定を使う」(カーブのフェードはモーションのフェード、
推定の軸は既定の軸) や「まだ入れていない」(グルーの足すキー) を表します。`optional` を渡さない欄は今までどおり空を持てません (空にして確定すると前の値に戻す)。

- **空の表示:** 欄は空にし、`placeholder` を出します。省くと、空が表す値 (`emptyValue`) を欄と同じ書式で薄く出します (フェードの欄の `0.5` など)。
  空の間は `aria-valuenow`・`aria-valuetext` を付けません。
- **確定:** 欄を空 (空白だけも) にして確定 (Enter・フォーカスを外す) すると、`onchange(null)` を呼びます。もとから空なら呼びません。
  読めない文字は、ほかの欄と同じく確定前の値 (空なら空) に戻します。
- **矢印キー:** 空から上下の矢印・PageUp / PageDown で動かすときは、`emptyValue` から動かします (`0.5` から上で `0.6`)。
  `emptyValue` が無ければ、範囲に収めた 0 から動かします (値が数でないときと同じ)。
- **打つたびの値:** `oninput` は打つたびに、読めた値を範囲に収めて (丸めずに) 渡します。`optional` で空なら `null`。読めない文字の間は呼びません。
  確定を待たずにボタンを押せるかを決める欄 (グルーの「Key」、推定の「Set partner」) が使います。
- 型は `optional` で分かれます (`optional` なら `value`・`onchange`・`oninput` が `number | null`)。

## 文字の選択

デスクトップアプリらしく、画面の文字は**既定で選択できません** (T0911)。ドラッグのはずみに文字が選択されて操作の邪魔をしないためです。
`@ugoe/ui` の `tokens.css` の末尾で、`<html>` に `user-select: none` (WebKitGTK・WKWebView 向けの `-webkit-user-select` も) を付けています。
スタジオの出力ウィンドウ (`apps/studio/output.html`) は `tokens.css` を読まないので、同じ指定を `<style>` に書いています。

- **入力の欄の中は選択できます:** 文字の `<input>` (ボタン・チェックボックス・スライダーなどの型を除く)・`<textarea>`・`contenteditable`。
  `NumberField` や `Slider` の数値の欄も中身は `<input>` なので、そのまま選択・コピーできます。
- **選択を許すときは `.ugoe-selectable` を付けます。** 付けた要素とその中が選択できます。CSS に `user-select: text` を直に書かず、このクラスを使います
  (印を探せば例外の一覧が分かるように)。許すのは、利用者が写したい文字だけにします。**エラーの文は選択できるようにします**
  (利用者の判断。報告や検索に写せるように)。新しくエラーの文を出すときは、`Notice` / Toast の `danger` を使うか、`.ugoe-selectable` を付けます。いま許している所:

| アプリ | 場所 | 理由 |
| --- | --- | --- |
| `@ugoe/ui` | `Notice` と Toast のうちエラー (`danger`) の見出しと本文 (部品の側で付ける) | エラーの文を報告に貼る |
| エディタ・ビューア・スタジオ | エラーの文 (`.error`・ビューアの `player.error`・スタジオの `.studio__error` など、失敗を出す文のすべて) | エラーの文を報告に貼る |
| エディタ・ビューア・スタジオ | ライセンスのダイアログの全文 (`LicenseDialog` の `<pre>`) | 引用・転記 |
| エディタ | モジュールの問題の帯の「詳細」(`ModuleProblemBar`)・着せ替えの問題の詳細 (`DressUpSection`)・アバターの検査のエラーの一覧 (`AvatarSection`) | エラーの詳細を報告に貼る |
| エディタ | マニュアルを開けなかったときの知らせ (`HelpButton`) | URL を写してブラウザで開く |
| エディタ | エージェントの接続の手順の設定の例 (`ConnectAgents` の `<pre>`) | 設定のファイルに貼る |
| スタジオ | 操作の API の URL (`ApiPanel`) | 外のツールに貼る (コピーのボタンもある) |
| スタジオ | 仮想カメラの失敗の文と詳細、入れ方のコマンド (`VirtualCameraPanel`) | エラーの詳細を報告に貼る・端末に貼る |
| スタジオ | ホットキーのエラー (`warning` の `Notice` の本文) | エラーの文を報告に貼る |
| ビューア・スタジオ | トラッキングのパネルのエラーの文 (`@ugoe/face-tracking` の `TrackingPanel`) | エラーの文を報告に貼る |

- **画像とリンクはドラッグで持ち出せません** (`-webkit-user-drag: none`)。ファイルのドロップ (`DropZone`)、
  ポインタのイベントで作るドラッグ (ギズモ・スライダー・タイムライン・ツリーとポーズの並べ替え) には影響しません。
  エディタは Tauri のファイルのドロップを有効にしているので、Windows (WebView2) では HTML5 のドラッグ & ドロップ (`draggable`) が
  届きません。要素の並べ替えはポインタのイベントで作ります (T0696)。
- 指定は `:where()` で詳細度を 0 にしているので、部品やアプリの CSS で上書きできます。

## UI の品質の基準

M10 (UI の品質) のタスクが同じ物差しで画面を見直すための約束です (T0915)。上のトークン・部品・「エディタの中の共通の形」は「何を使うか」、
ここは「画面をどう組むか」と「どう確かめるか」です。機械で見つけられるものは `just ui-audit` ([docs/testing.md](testing.md#ui-の監査)) が数え、
残りは撮った画像を人が見ます。各項の **確かめ方** に、どちらで見るかを書きます。

### 配置

- **情報の優先度:** 画面ごとに「いちばんよく使う操作」を決め、それを最も目に入る位置 (左上・各パネルの頭) に置きます。`primary` のボタンは画面に 1 つまで。
  まれな操作 (書き出しの設定・取り消せない操作・開発者向けの情報) は、メニュー・設定・折りたたんだ `Section` の奥に置きます。
  利用者が知っているべき状態 (保存していない・配信中・トラッキング中・エラー) は、送らずに見える位置に、文と印で出します。
  確かめ方: 人 (広い幅と狭い幅の画像で、送らずに見えるものを並べ、優先度と合っているか)。
- **揃え:** 文字は左揃え、数値は右揃えで `tabular-nums`。1 つのパネルの中では、欄の名前の列と部品の列の左端をそろえます (`.ed-field` の格子)。
  ボタンの並びの決定は並びの最後 (「エディタの中の共通の形」)。アイコンと文字は中心の線でそろえます。
  確かめ方: 人 (画像に縦の線を当てて見る)。
- **余白の段:** 余白はトークン (`--ugoe-space-*`) の段だけを使い、関係の近いもの同士ほど狭くします: 部品の中・ラベルと部品は `space-1`〜`space-2`、
  部品の間は `space-2`〜`space-3`、まとまりの間は `space-4`、パネルの内側とダイアログは `space-4`〜`space-5`。まとまりの中の余白を、
  まとまりの間の余白より広くしません (どれとどれが仲間か分からなくなる)。
  確かめ方: 人と、直書きの検索 (`grep` で `padding`・`margin`・`gap` の px を探す。残してよいものは「トークンと部品の見直し (T0916)」の表)。
- **グループの見せ方:** まとまりは、まず見出しと余白で示します。枠 (`fieldset`・カード) は、意味のある組 (位置・大きさ・回転のように一緒に変える欄、
  ラジオの組) だけに使い、枠の中に枠を重ねません。区切りの線は、余白だけでは分からない所 (ツールバーと中身の境など) に 1 本だけ引きます。
  折りたためるまとまり (`Section`) は、閉じても何が入っているかが見出しで分かるようにし、閉じた状態の要約 (数・状態) を見出しの右に出してよいです。
  確かめ方: 人。

### レイアウト

- **幅の変化:** 対象の幅は、エディタ 1024px〜、ビューアとスタジオ 360px〜です (「アクセシビリティの基準」の「大きさの変化」)。主な領域 (ビューポート・
  シーンの一覧) を最も広くし、横のパネルには最小と最大の幅を付けます。幅が足りないときは、まず折り返し (ボタンの並び)、次に列ごと縦に送る並び
  (ビューア・スタジオの狭い幅)、最後に省略記号 (名前。全体は Tooltip か `title`) の順で逃がします。ボタン・見出し・欄の名前に固定の幅を付けません
  (英語は日本語より長い)。
  確かめ方: `just ui-audit` (横のはみ出し・文字の重なりと切れ。広い幅と狭い幅) と人。
- **高さの変化:** 低い画面 (エディタ 720px) でも主な操作が見えるようにします。送るのはパネルの中で、パネルの頭 (見出し・タブ・道具の並び) は送らずに残します。
  ビューアとスタジオの狭い幅だけは、画面の全体を縦に送ります。
  確かめ方: 人 (`-bottom` の場面で、送った先が見えること)。
- **送り:** 1 つの領域に縦の送りは 1 つにします (送る領域の中に、また縦に送る領域を入れない)。横の送りは表とタイムラインだけにします。
  送れることが分かるように、領域の端で行を途中で切って見せるか、見出しを固定 (`position: sticky`) にします。選んだもの・フォーカスしたものは、見える所まで送ります。
  送る枠の中の `position: absolute` の要素 (見せずに読ませる文字 `.ugoe-visually-hidden` など) は、送る枠かその内側を位置の基準にします
  (`position` の無い送る枠の外の祖先を基準にすると、枠で切り取られずに外の祖先を送れるようにしてしまう)。`Notice` は自分を基準にする (`position: relative`) ので、
  どこに置いてもよいです。アプリが自分で置く見せずに読ませる文字 (表の見出し・欄の名前など) は、送る枠に `position: relative` を付けるか、
  文字を包む要素を基準にします (エディタはパネルの送る枠に付けている。T0954)。
  縦に送る枠は、行末 (右) の padding を `--ugoe-scrollbar-gap` (16px) 以上にして、スクロールバーの分を中身から空けます。
  WebKitGTK (Linux)・macOS のスクロールバーは幅を取らずに中身の上に重ねて描き (WebKitGTK は細い印が 3px、指を乗せると 8px に太り、
  右端から 14px まで掛かる)、`scrollbar-gutter` はこの形のスクロールバーには幅を空けないためです (T0970)。中身が右の端まで届く一覧
  (ビューアのパラメータの一覧) は、置く側のパネルの右の padding まで一覧を広げ (`margin-inline-end` を負に)、その padding の中に描かせます。
  幅を取る形のスクロールバー (ヘッドレスの Chrome・Windows) では、空けた幅はスクロールバーの左に余白として残るだけです。
  確かめ方: 人。重ねて描くスクロールバーは本物の WebKitGTK の画像 (`just e2e --manual`) で見ます。ビューアとスタジオの送る枠の規則が
  `--ugoe-scrollbar-gap` を使うか、使わない理由をスクロールバーの言葉で書いていることは packages/ui の `tokens.test.ts`。Tab で送り先に隠れたまま止まらないことは `just ui-audit` (Tab の止まりの「画面の外」)。送る枠の外を基準にする absolute の要素は
  `just ui-audit` の `overflow` (`… escapes …`) と、部品の見本 (`just ui-shots`) の `position` の無い送る枠の中の `Notice`。
- **分割:** 大きさを変えられる境 (`role="separator"`) は、ポインタとキーボード (矢印・Home・End) の両方で動かせ、フォーカスの印を出し、
  変えた大きさを覚えます。最小の大きさで中身が潰れない (見出しと 1 行が見える) ようにします。
  確かめ方: `just ui-audit` (Tab の止まりの印) と人。

### デザイン

- **トークン:** 色・余白・文字・角丸・影は上の「デザイントークン」から取ります。文字を薄く見せるのに `opacity` を使いません。新しい色はコントラスト比を計算して表に足します。
  確かめ方: `just ui-audit` (コントラスト)、`packages/ui/src/tokens.test.ts` (宣言と使い方の食い違い)、直書きの色の検索 (`#`・`rgb(`。残してよいものは「トークンと部品の見直し (T0916)」の表)。
- **アイコン:** アイコンだけのボタンは `IconButton` (読み上げの名前とツールチップ) にします。意味を伝えるアイコンは地に 3:1 以上にし、色だけで違いを伝えません。
  文字の記号 (◆・▾・▶) と絵のアイコンを、同じ並びの中で混ぜません。大きさ・線の太さ・揃えは上の「アイコン」(T0916)。
  確かめ方: `just ui-audit` (名前) と人。
- **状態の見せ方:** 部品の状態は次の形で示し、どれも色だけに頼りません。

  | 状態 | 見せ方 | トークン・部品 (T0916) |
  | --- | --- | --- |
  | 通常 | トークンの地と枠 | `surface`・`control-border` |
  | hover | 地に本文の色を薄く重ねる (枠と文字は変えない。控えめな文字だけ本文の色に)。押せないもの・無効のものには付けない | `background-image: var(--ugoe-overlay-hover)` |
  | 押下 | hover よりもう一段濃い重ね | `var(--ugoe-overlay-active)` (`:active`) |
  | フォーカス | `--ugoe-focus-ring` (`:focus-visible`)。外側の枠に出す部品は `:focus-within` | `outline: var(--ugoe-focus-ring)`・`outline-offset: var(--ugoe-focus-offset)` |
  | 選択 | 強調色の地か枠に加えて、形 (チェックの印・下線・枠・太字) | `accent`・`accent-soft`。一覧の行の選択は `accent-soft` の地に、形 (印・太字・枠) を足す |
  | 無効 | 部品の全体を薄くする (コントラストの基準の外)。理由を近くに書くか Tooltip で出す | `opacity: var(--ugoe-opacity-disabled)`。一覧の中の 1 項目だけ無効なときもその項目に同じ値 |
  | エラー | 危険の色の枠と印と文 (`aria-invalid`・`aria-describedby` で文を結ぶ) | `TextField` の `error` (文を欄の下に出して結ぶ)・`invalid`、`NumberField`・`Select` の `invalid`。標準の文字の欄は `[aria-invalid="true"]` に `danger` の枠 |
  | 読み込み中・実行中 | その場所に文 (「読み込んでいます…」) と、長いものは進み具合と取り消し。押すと二重になる操作は無効にする | `Button`・`IconButton` の `busy` (無効にし `aria-busy`。文言は呼び手が「保存しています…」に変える) |

  キーボードで選んでいる候補 (メニュー・`Select` の一覧の項目) は、hover と同じ扱いでなく `accent-soft` の地と強調色の 1px の枠で示します
  (矢印で移る位置がフォーカスの代わりなので、hover より強く見せる)。

  確かめ方: 見本の画面 (`pnpm --filter @ugoe/ui gallery`・`just ui-shots`) で部品ごとに、アプリの画面は人。
- **空の状態:** 何も無い領域 (プロジェクトなし・パーツなし・モデルなし・シーンなし) には、「何が無いか」と「次に何をするか」を 1 文で書き、
  次の操作のボタンをその場に置きます (文でメニューの場所を教えるだけにしない)。
  確かめ方: `just ui-audit` の `-empty` の場面を人が見る。
- **読み込み中:** 0.5 秒より長くかかりそうな処理は、始めた場所に「何をしているか」を出します。数秒を超えるもの (AI の送信・書き出し・PSD の取り込み) は
  進み具合か経った時間と、取り消しを出します。待っている間も、ほかの領域の操作を止めません (止めるならモーダルで理由を出す)。回る印は
  `prefers-reduced-motion: reduce` で止め、文だけでも分かるようにします。
  確かめ方: 人 (偽のホストでは待ちを作れない。実機か e2e)。
- **エラー:** 失敗は起きた場所に出します (操作の失敗は `Notice` の `danger`、欄の入力の誤りは欄の下の文)。「何が起きたか」と「どうすればよいか」を
  利用者の言葉で書き、内部のメッセージ (英語のコード・パス) は詳細に回します。エラーを Toast だけで出しません (消えて読み返せない)。
  エラーの文は選択できるようにします (「文字の選択」)。
  確かめ方: `just ui-audit` (画面に出たエラーを数える。見本で出るはずのないエラーを見つける) と人。

### アクセシビリティ

基準は下の「アクセシビリティの基準」です。T0915 で、その確かめ方 (スクリーンリーダー・200% の拡大・reduced motion) を [確かめ方](#確かめ方) に足しました。
T0918 で、[骨組み (ランドマークと見出し)](#骨組み-ランドマークと見出し)・[状態の変化の知らせ](#状態の変化の知らせ)・[強制色](#強制色) を足しました。

## アクセシビリティの基準

目標は [WCAG 2.2](https://www.w3.org/TR/WCAG22/) の AA に当たる水準です。ビューポート (モデルの描画) そのものは対象の外ですが、ビューポートで行う操作には、パネルやキーボードでの代わりの手段を用意します。

### キーボード

- **マウスでできる主な操作は、キーボードだけでもできる**ようにします。Tab の順は見た目の順 (左から右、上から下) に合わせ、`tabindex` に正の値を使いません。
- 複数の部分からなる部品 (`Tabs`・`Toolbar`・`Menu`・`SegmentedControl`) は、部品の全体で 1 つのタブ止まりにし、中は矢印で移ります (roving tabindex)。
- フォーカスを閉じ込めるのはモーダルの `Dialog` だけです。それ以外でキーボードが抜け出せなくなる場所を作りません。
- Esc は、いちばん上に重なったもの (メニュー → ツールチップ → ダイアログの順) を 1 つだけ閉じ、フォーカスを開いた元に戻します。
- 1 文字のショートカット (修飾キー無し) は、文字の入力欄・数値の欄・`Select` にフォーカスがあるときは効かせません。ショートカットの一覧はヘルプとメニューに出します (`Kbd`)。
- ビューポートの操作ごとのキーボード (とパネル) での代わりは、[docs/editor.md の表](editor.md#ビューポートの操作と代わりの手段) にまとめます。
  ビューポートのキー (矢印・Alt + 矢印・`]` `[`) は、フォーカスがどこにも無いかビューポートの中にあるときだけ効かせ、ツリー・スライダー・
  タイムラインの矢印を奪いません。点を 1 つずつ選ぶのに Tab を使わない (フォーカスの移動に残す) のも同じ理由です。

### フォーカス

- フォーカスは常に見えるようにします。`:focus-visible` に `--ugoe-focus-ring` を付け、`outline: none` を代わりの表示なしに使いません。
- フォーカスの輪は、隣り合う色に対して 3:1 以上にします (強調色はどちらのテーマの地にも 5.6 以上)。
- 項目を消したり、ダイアログを閉じたりしてフォーカスしていた要素が無くなったら、近くの意味のある要素 (次の行・一覧・開いた元のボタン) にフォーカスを移します。

### コントラストと色

- 文字は 4.5:1 以上 (24px 以上、または 18.66px 以上の太字は 3:1 以上)。無効の部品の文字は基準の外ですが、読めることを確かめます。
- ビューポート (canvas) の上に重ねる文字は、テーマの地の札に載せます (地はモデルの背景色で、テーマと関わりなく決まるため。T0725)。
  `just ui-audit` は canvas の上の文字を画素で測ります (T0952)。
- 部品の境界 (入力欄の枠・チェックボックス・スライダーのつまみと溝) とアイコンは 3:1 以上。上の色のトークンはこれを満たします。新しい色を足すときは比を計算してこの表に書きます。
- **色だけで状態を伝えません。** エラーは色に加えて印と文で、選択は色に加えて形 (枠・チェックの印) で示します。

### そのほか

- **名前:** すべての操作できる部品に、読み上げの名前を付けます (見えるラベル、`<label>`、または `aria-label`)。`aria-label` などの文言も翻訳を通します。
- **見せずに読ませる文字:** 読み上げだけに渡す文字 (表の見出しの無い列の名前・印の意味など) は、`@ugoe/ui` の `tokens.css` の
  `.ugoe-visually-hidden` を付けます。アプリの CSS に同じ指定 (`.visually-hidden`) を書き直しません。
- **押せる大きさ:** 押せる部品は 24×24 CSS px 以上 (WCAG 2.5.8)。`compact` でもこれより小さくしません。
  エディタの密な行 (タイムラインの行とキー・ツリーの行と開閉・パラメータの名前と値の欄) もこれに従います (2026-10-02、利用者の判断。T0907)。
  見た目の印 (◆・▸・グラフの点) は小さいままでよく、押せる範囲 (透明な枠) を 24px に広げます。行が減っても、選んだ行は見える所まで送ります。
- **動き:** `prefers-reduced-motion: reduce` では、部品の開閉の動きを止めます。1 秒に 3 回より多く点滅させません。
- **言語:** `<html lang>` を今の言語にします (読み上げとフォントの選択に効く)。
- **大きさの変化:** WebView の拡大 (200%) と高 DPI、狭い幅 (ビューアとスタジオは 360px、エディタは 1024px) で、文字が切れたり重なったりしないようにします。
  英語は日本語より長くなりやすいので、ボタンや見出しに固定の幅を付けません。
- **部品の作りの注意:** Bits UI に任せた部品も、包んだ後の名前・ラベルの付き方は自分で確かめます。
  例: Bits UI 2.19.4 の `Select` の項目は、無効でも `aria-disabled` を付けないので、`@ugoe/ui` で付けています。
  同じく `Tooltip` の中身に `role="tooltip"` を、`DropdownMenu`・`ContextMenu` の一覧に読み上げの名前 (`aria-label`) を付けています。
  `Toolbar` は、描いた時点で中の項目がすべて無効だと、押せるようになってもタブ止まりが無いまま (Tab で届かない) なので、
  `@ugoe/ui` で「押せる項目のうち 1 つだけを `tabindex="0"`」に保っています (T0907)。

### 骨組み (ランドマークと見出し)

スクリーンリーダーの利用者は、見出しとランドマークの一覧で画面を飛んで回ります。画面の構造と同じ骨組みにします (T0918)。

- **ランドマーク:** 上のツールバーは `<header>` (banner)、主な領域 (ビューポート・シーンの一覧) は `<main>`、横の列は `<aside>` (complementary)、
  足元の状態の行は `<footer>` (contentinfo)。同じ種類のランドマークが 2 つ以上あれば `aria-label` で名前を付けます (エディタの左右の列)。
  まとまり (`<section>`) は、名前 (`aria-label`・`aria-labelledby`) を付けたものだけが region になります。メニュー・Popover・ダイアログ・Toast は
  開いた元の外に描くので、ランドマークの外でよいです。
- **見出し:** 各アプリに 1 つだけ `<h1>` (アプリの名前。見た目にはツールバーがあるので `.ugoe-visually-hidden`) を置き、パネルは `h2`、
  その中のまとまりは `h3` と、段を飛ばさずに下げます。`Section` は `level` で段を合わせます (既定は 3)。置く場所で段が変わる共有の部品
  (`@ugoe/face-tracking` の `TrackingPanel`) は `level` を受け取ります。タブと同じ名前の見出しは、見た目では隠して読み上げに残します (エディタの右の列)。
- **見出しの中にボタンを入れません** (見出しの名前にボタンの文が混ざる)。見出しと操作は横に並べた別の要素にします。

| アプリ | banner | main | complementary | contentinfo |
| --- | --- | --- | --- | --- |
| エディタ | ヘッダー (h1・「File」の nav・保存の状態) | ビューポートとタイムライン (region「Timeline」) | 「パーツとデフォーマ」(ツリー)、「プロパティとパラメータ」(右の列。h2 はタブの名前と「パラメータ」)、プレビュー中は右の列の代わりにプレビューのパネル (h2「プレビュー」) | なし |
| ビューア | ツールバー (h1) | ビューポートと右の列 | 右の列 (h2 はフェイストラッキング・ライティング・モーションと表情・ポーズ・パラメータ) | 状態の行 |
| スタジオ | ツールバー (h1) | シーンの一覧 (h2)・いまの状態 (h2)・表情とモーション・ポーズ (h2) と右の列 | 右の列 (パネルごとに h2) | なし |

### 状態の変化の知らせ

画面の別の場所で起きた状態の変化は、フォーカスを動かさずに読み上げで知らせます (WCAG 4.1.3。T0918)。うるさくならないよう、利用者が
知るべき変化だけを、1 回の変化につき 1 回知らせます。

| 道具 | 使う所 | 読み方 |
| --- | --- | --- |
| `Notice` (`role="status"`。`danger` は `role="alert"`) | 画面に残る知らせ・エラー (起きた場所に出す) | 出たときに読む。エラーは割り込んで読む |
| `showToast()` (`Toaster` の `role="status"` の領域) | すぐ消えてよい知らせ (保存した・書き出した・画像を保存した) | 出たときに読む |
| `announce()` (`@ugoe/ui`。`Toaster` が見えない `role="status"` と `aria-live="assertive"` の領域を持つ) | 画面には文が出ない・離れた所で文が変わるだけの変化 | 渡した文を読む。同じ文が続いても読む。既定は読み終わるのを待つ (`assertive` は割り込む。失敗だけに使う) |
| その場の `role="status"` | 操作の近くの進み具合 (「保存しています…」・「分類しています…」・選んだ点の番号) | 文が変わったら読む |

- 今 `announce()` で知らせるもの: エディタの AI の提案が届いた (AI・つないだエージェント・作り直し。開いたときからある提案と、Undo で戻った提案は読まない)、
  ビューアのモデルを開き終えた (着せ替えなどの読み直しは読まない)、スタジオの出しているシーンが変わった (クリック・ホットキー・操作 API のどれでも。
  開いたときは読まない) と、自分で保存したプロファイル (自動の保存は読まない)。
- 今その場の `role="status"` で知らせるもの: フェイストラッキングのパネル (`TrackingPanel`) の見出しの横の状態 (準備中・トラッキング中・
  記録を再生中・顔が見つからない・体が見つからない・止まった。T0958)。顔の短い出入りで読み上げが続かないよう、状態が変わってから約 1 秒
  続いたときだけ文を変えます (`@ugoe/face-tracking` の `status.ts`)。止めたときは待たずに文を消します (空の文は読まれない)。fps と再生の位置は
  その隣の、live region の外の文に分けています。見出しの幅を空けるため、トラッキング中・記録を再生中の文は見せずに読ませます
  (`.ugoe-visually-hidden`。見た目は fps・再生の位置だけ)。
- **live region は先に置きます。** 中身と一緒に足した `role="status"` を読まない読み上げソフトがあるので、知らせの領域は描いたときから置いて中身だけを
  変えます (`Toaster`・`announce()`)。条件で出す `Notice` は「出たときに読む」に頼っているので、読まれないと分かったものは `announce()` に移します。
- **毎秒変わる数は live region の外に置きます** (AI に送っている間の経った秒数・トラッキングの fps・再生の位置)。読みたいときに読める所にあればよいです。

### 強制色

Windows の高コントラスト (コントラスト テーマ) は、ブラウザの強制色 (`forced-colors: active`) になります。地の色・影 (`box-shadow`)・グラデーションが消え、
文字と枠は OS の色に置き換わり、透明の枠も線になります (T0918)。

- **地の色や影だけで描いたもの・状態を地の色だけで示すものを、系統の色 (`Canvas`・`CanvasText`・`ButtonText`・`Highlight`・`HighlightText`・
  `GrayText`) で描き直します。** `@ugoe/ui` の部品の分は `tokens.css` の `@media (forced-colors: active)` の節にまとめています:
  押したボタン・選んだ区切りの項目・入れたチェックボックスとスイッチは `Highlight` の地、スイッチのつまみとスライダーの溝と範囲は系統の色、
  選んでいないタブの下線と区切りの項目の枠は地と同じ色 (選んだものだけに線が残る)、入力の誤りは太い破線の枠。
- フォーカスの輪は `outline` なので強制色でも残ります (`box-shadow` だけで描いた輪は消える)。アプリで部品を描くときも、印は `outline`・`border`・
  文字・形 (◆・チェックの印) で描きます。
- ビューポート (canvas) は強制色の外です (モデルの色のまま)。その上の文字は、テーマの地の札に載せる決まり (上の「コントラストと色」) のままで、
  強制色では札が OS の地の色になります。

### 確かめ方

| 何を | 道具 | 見るもの |
| --- | --- | --- |
| 部品 | Vitest (キーボード操作と aria の属性)、見本の画面 (`pnpm --filter @ugoe/ui gallery`)、`just ui-shots <出力先>` (両方のテーマ・言語・密度で撮り、名前を検査。[packages/ui/README.md](../packages/ui/README.md)) | 部品を足す・変えたとき (T0902・T0903) |
| 3 つのアプリの画面 | `just ui-audit` ([docs/testing.md](testing.md#ui-の監査)・[tests/ui-audit/README.md](../tests/ui-audit/README.md)) | コントラスト・横のはみ出し・文字の重なりと切れ・押せる大きさ・名前 (axe-core)・ARIA の規則・ランドマークと見出し・reduced motion の下の動き・強制色の状態・画面のエラー・Tab の順。画面を変える PR の前と後で流して比べる (`--compare`) |
| ランドマークと見出し | `just ui-audit` の `structure` (axe-core の landmark・heading・region の規則) と、場面ごとの `<場面>.outline.txt` (ランドマーク・見出し・live region を文書の順に並べたもの) | 上の「骨組み」の表と同じか。見出しの段が飛ばないか |
| 状態の変化の知らせ | 部品のテスト (`announce()` と `Toaster` の領域。`packages/ui/src/overlays.test.ts`)・アプリのテスト (いつ知らせるか: `proposalsView.test.ts`・`liveScene.test.ts`・`loadedModel.test.ts`)・トラッキングの状態の揺れの抑え (`packages/face-tracking/src/status.test.ts`)・`.outline.txt` の `[status]` の行 | 領域が先に置かれ、知らせるべき変化で 1 回だけ文が入るか |
| スクリーンリーダー | 下の手順と [docs/manual-check.md の U03](manual-check.md#u03-スクリーンリーダーで主な流れを読む-b-wml) | 名前・役割・状態が読まれるか、読む順が見た目の順か、知らせが読まれるか |
| 強制色 | `just ui-audit` の `en-dark-forced-wide` と `ja-light-forced-narrow` (Chrome の強制色のエミュレーション。暗い組では Tab もたどる) と、[U04](manual-check.md#u04-os-の高コントラスト動きを減らす拡大-b-wml) | 入・押した・選んだ状態が見分けられる (`forced`。同じ種類の部品の入と切の見た目を比べる)、フォーカスの輪が残る |
| 200% の拡大 | `just ui-audit` の `en-light-zoom200` と `ja-light-zoom200` (CSS の窓の幅を半分にし、devicePixelRatio 2) と、実物の WebView | 文字が切れない・重ならない・横に送らずに読める |
| 動きを減らす | `just ui-audit` (すべての場面を `prefers-reduced-motion: reduce` で開き、動いている animation・transition を数える) と、OS の設定 | 開閉・回る印が止まる |

- **スクリーンリーダー:** 機械の検査は名前の有無までで、読まれ方は人が聞いて確かめます。`just ui-audit` が残す `<場面>.aria.yml`
  (アクセシビリティの木。役割・名前・状態) は読まれる内容の近似で、先に目で見て足りないものを探せます。実物では、Linux は Orca
  (`orca` を起動し、Tauri のアプリを開く)、Windows は NVDA、macOS は VoiceOver (Cmd + F5) を使い、次をたどります:
  Tab と Shift + Tab で主な操作を一巡する (名前と役割が読まれ、見た目の順か)・メニューとダイアログを開いて閉じる (開いたことと、閉じた後の
  フォーカスの戻り先が分かるか)・状態を変える (トグル・選択・保存。新しい状態が読まれるか)・エラーを起こす (知らせが読まれるか)。
  アプリごとの流れは [docs/manual-check.md の U03](manual-check.md#u03-スクリーンリーダーで主な流れを読む-b-wml) です。
- **強制色:** ヘッドレスの Chrome では、ページの開き方に `forcedColors: "active"` を渡すと強制色のエミュレーションになります (CDP の `Emulation`)。
  `getComputedStyle` が強制した後の色を返すので、`just ui-audit` は部品の入と切の見た目をそのまま比べられます。実物では Windows の
  コントラスト テーマで確かめます ([U04](manual-check.md#u04-os-の高コントラスト動きを減らす拡大-b-wml))。
- **200% の拡大:** ヘッドレスの Chrome では、窓の CSS の幅を半分にして devicePixelRatio を 2 にすると、200% に拡大した WebView と同じ
  レイアウトになります (`en-light-zoom200`)。実物では OS の拡大 (Windows の「拡大縮小」200%・GNOME の「大きな文字」・macOS の
  「ディスプレイ」の拡大) で確かめます。
- **動きを減らす:** `prefers-reduced-motion: reduce` は、Windows の「アニメーション効果」を切る・macOS の「視差効果を減らす」・GNOME の
  「アニメーション」を切る、で入ります。ヘッドレスの Chrome では `--force-prefers-reduced-motion` で入ります。

### 3 つのアプリの見直し (T0907)

エディタ・ビューア・スタジオの操作ウィンドウを、ビルドした画面をヘッドレスの Chrome で開いて見直しました
(ライト・ダーク × 英語・日本語 × 広い幅 (エディタ 1440px・ほか 1280px) と狭い幅 (エディタ 1024px・ほか 360px)、と devicePixelRatio 2)。

- **コントラスト:** 文字を持つ要素ごとに、文字の色と祖先を重ねた地の色の比を WCAG 2.x の式で計算しました (自前の小さなスクリプト。T0915 で `just ui-audit` にした)。
  無効の部品を除いて、すべて基準を満たします (非表示のパーツ名は `opacity` で 3.4 だったので控えめな文字の色に直した)。
- **キーボード:** Tab でたどってフォーカスの印と読み上げの名前を記録しました。スタジオのツールバーに届かなかったもの (上の `Toolbar`) と、
  エディタのツリーの薄い印を直しました。
- **押せる大きさ:** 24px 未満のものは 0 件です (`Toggle` のスイッチ・`Checkbox` の箱は見えるラベルまで、`NumberField` は枠の全体が押せる範囲)。
  エディタのタイムラインの行とキー・グラフの点と取っ手・ツリーの行と開閉・パラメータの名前と値の欄・標準の文字の欄を 24px にしました。

### 3 つのアプリの基準線 (T0915)

`just ui-audit` を入れたときの結果を、M10 のタスクの出発点にしました (詳細と振り分けは T0915 のタスクファイルのメモ)。
T0907 で見た場面ではコントラスト・はみ出し・押せる大きさ・名前が 0 件のままでしたが、T0907 で見ていなかった場面で次が見つかりました。

- エディタの右の列のタブ: 24px 未満の押せるもの (物理のリグと衝突体の一覧・スロットの一覧と描く順の端・ライティングのメッシュの一覧と
  チェックボックス・光の向きの印)。T0674 で直します。
- エディタの 200% の拡大: ビューポートの右上の `webgl2` が左上の道具の並びに重なる (T0675)。
- エディタの空のプロジェクト: パーツのツリーが `role="tree"` なのに項目が無い (axe の `aria-required-children`)。分割の境にフォーカスの印が無い (T0674・T0673)。
- ビューア: ビューポート (`role="img"`) の中に光の向きの印 (`role="slider"`) がある (axe の `nested-interactive`)。光の向きの印にフォーカスの印が無い (T0725)。
  どちらも T0725 で直し、ビューアは 0 件になった (下の「ビューアの見直し (T0725)」)。

### トークンと部品の見直し (T0916)

トークン・部品の状態・アイコン・密度を見直し、アプリの CSS の直書きをトークンに置き換えました。値と理由は上の各表にあります。

- **足したトークン:** hover と押下の重ね (`--ugoe-color-hover`・`-active`・`--ugoe-overlay-*`)、無効の薄さ (`--ugoe-opacity-disabled`)、
  フォーカスの輪の離れ (`--ugoe-focus-offset`)、アイコン (`--ugoe-icon-size`・`--ugoe-icon-stroke`)、分類の色 (`--ugoe-color-hue-*`)。
  部品ごとに違っていた hover (地を `sunken` にする・`surface` にする・文字の色だけ変える・何もしない) と、無効 (薄くする・控えめな文字の色) を
  これらにそろえました。
- **変えた値:** `-soft` の 3 色を `surface` に混ぜる形に (強調色とそろえた)、本文の行の高さを 1.4 から 1.5 に。
- **整理:** 使われていないトークンはありませんでした。宣言の重複 (ダークの値を `@media` と `[data-theme="dark"]` の 2 か所に書く) は
  CSS の作りの上で避けられないので、`packages/ui/src/tokens.test.ts` で 2 か所の値がそろっていること・ライトとダークの名前がそろっていること・
  宣言したトークンがどこかで使われていること・使っているトークンが宣言されていることを確かめます。
  移行のための変数の対応表 (前のアプリの `--bg` など) は、もう使われていないので消しました。
- **部品に足したもの:** `Button`・`IconButton` の `busy` (実行中)、`NumberField` の `invalid` (入力の誤り)、公開の `Icon`。
  見本の画面 (`pnpm --filter @ugoe/ui gallery`) に、実行中のボタン・誤りのある数値の欄・`hover` と `active` の色を足しました。

直書きのまま残したもの (どれもトークンの外にしてよい理由があるもの):

| 場所 | 値 | 理由 |
| --- | --- | --- |
| ビューポートの上の図 (エディタの `viewport/tools/**`・`ToolOverlay`・`DifferenceOverlay`、ビューアの `LightMarkers`・`LightPad`、`Viewport` の格子) | 色・線の太さ・透明度 | 描画の中身 (「デザイントークン」の例外)。モデルの色の上で見えるように、テーマに依らない色で描く。エディタの色は T0675 で `--viewport-*` にまとめた ([ビューポートの上の図](#ビューポートの上の図)) |
| 道具の並びの位置 (`ToolOverlay`・`GlueOverlay` などの `top`・`left`) | 8px・40px など | ビューポートの中の位置。余白の段でなく、重ねる図の場所 |
| 1px の隙間と内側の余白 (`RestPoseWarnings`・`VerifyPanel`・`SlotsPanel` など) | 1px | 線の太さと同じ髪の毛の幅。余白の段 (最小 2px) より細い |
| 形に結び付いた値 (`ParametersPanel` の印の -3px、`TreeItem` の開閉の -5px、`MotionEditor` の `scroll-margin-top`) | 小さな px | 印の大きさ・行の高さから決まる |
| 箇条書きの字下げ | `1.5em` | 文字の大きさに比例させる |
| 透過の市松模様 (`NormalMapSection`) | `#8884` | 画像の透明な所の印。地のテーマに依らず見えるように半透明 |
| トラッキングの映像の地 (`TrackingPanel`) | `#000` | 映像の外の帯 (映像と同じく暗くする) |
| 並べ替え中の行 (`PosesPanel`) | `opacity: 0.5` | 持ち上げている行の印 (無効ではない) |

### 足りない部品と口 (T0917)

T0907 の後続の候補だった部品の不足を埋め、アプリに残る自前の欄と `title` を減らしました。

- **部品に足したもの:** `TextField` (ラベル・補足・誤りの文・無効)、`Slider`・`NumberField`・`Select` の `tooltip`、`Select` の `invalid`、
  `Slider` の `aria-describedby`、`Menu` の項目の説明の行 (`description`)。
- **置き換えたもの:** 3 つのアプリの標準の文字の欄 (名前・ID・URL・モデル・ポート・検索・イベントの値) を `TextField` に、AI の設定の
  時間の上限 (`type="number"`) を `NumberField` にしました。ビューアの「サンプル」のメニューに見本の中身を、スタジオの最近のプロファイルに
  ファイルの場所 (名前はファイル名) を、説明の行で出します。
- **トラッキングの平滑化の説明:** T0907 で見える補足にしていた説明を、`Slider` の `tooltip` に戻しました。同じパネルの入・切 (`Toggle`) の
  説明が Tooltip なのでそろえるのと、広い幅のビューアでトラッキングのパネルが長く、パラメータが送らないと見えないため (T0915 の振り分け)。
- **見える補足・Tooltip にした `title`:** 物理のリグの「基準」(`Select` の `tooltip`)、テンプレートの「合成」(`Select` の `tooltip`) と
  「足りないパラメータを作る」を押せない理由 (見える補足)、カーブのフェードの欄 (Tooltip)、タイムラインのキーを打つ ◆ とキーの印 (Tooltip。◆ には
  読み上げの名前も付けた)、静止姿勢の警告の行・検証のセル (Tooltip)、AI の提案の注文の文と依存 (見える補足)、ビューアのイベントの読み方 (見える補足)。

T0917 で部品が無くて残した 3 種類の欄は、T0949 で部品を足して移しました: 空にできる数値の欄 (カーブのフェード・グルーのキーと新しい軸の
最初のキー・推定のミラーの軸と許す差・左右の対の軸) は `NumberField` の `optional` に、複数行の欄 (アバターのメタデータの作者・参照・
第三者のライセンスと、AI の注文) は `TextArea` に、API キー (`SettingsDialog`) は `SecretField` に。

標準の `<input>` のまま残したもの:

| 場所 | 理由 |
| --- | --- |
| パラメータの値の欄 (`ParametersPanel`) | 範囲の外を丸めず、誤りの文を出す作り (誤りの文は `aria-describedby`・`aria-invalid` で結んだ) |
| チェックボックス・ラジオ・範囲・ファイルの `<input>` | 文字の欄ではない。ライティングの「光を受けるメッシュ」の行のチェックボックスは、名前を選ぶボタンと分けるため見えるラベルを持たない (`Checkbox` は見えるラベルが要る)。押せる範囲は箱を包む 24px の `<label>` (T0674) |

残した `title` (フォーカスを受けない要素の補い):

| 場所 | 中身 |
| --- | --- |
| エディタのツリーの行 (`TreeItem`) | 名前の ID・行の印 (本体・マスク・非表示・描く順)。行はフォーカスを受けない (木が `aria-activedescendant` で受ける) |
| 物理のリグの一覧の「中継」の印 (`PhysicsPanel`) | 読むパラメータの一覧。ボタンの中の印なので Tooltip を入れ子にできない |
| タイムラインの行の名前・目盛り・イベントの印 (`MotionEditor`・`CurveGraph`) | 省略した名前の全体・目盛りの操作 (コマ送りのボタンで同じことができる)・図の印 |
| AI の提案の面の数 (`ProposalsPanel`) | テクスチャの名前の一覧 (省略した全体) |
| プレビューの着せ替えの行のファイル名 (`DressUpSection`) | ファイルの全体のパス (行には名前だけを出す。T0676) |
| ビューアのモーション・表情・着せ替え・覚えたものの行と状態の帯 | 名前と動かすパラメータ・重なりの印・警告の全体 (始めの 1 件は見える)・場所の全体 |
| ビューアのパラメータの行 (`ParameterPanel`) | 省略した名前の全体と ID (T0725)。行はフォーカスを受けない (中のスライダーと欄が受ける) |

### エディタのパネル (T0674)

「配置」「レイアウト」「デザイン」の基準で、エディタのパネルの中 (ツリー・インスペクタ・パラメータ・物理・グルー・ライティング・スロット・ポーズ・検証) を
見直しました (詳細は [docs/editor.md の「選択とホバーの見せ方」](editor.md#選択とホバーの見せ方)・[「インスペクタの並び」](editor.md#インスペクタの並び))。

- **欄の並び:** パネルごとに違っていた行の作り (名前と部品を両端に寄せて部品の幅を 12em に固定する・部品の `label` で名前を横に出す・格子) を、
  `.ed-field` の格子にそろえました (スロット・ポーズ・アバタープロファイル・深度・描画用の複製・光源・物理のテンプレート・新しいグルー)。
  欄の説明 (`.ed-help`)・注記 (`.ed-note`)・枠のまとまり (`.ed-card`)・一覧の行 (`.ed-item`)・札 (`.ed-tag`) を `app.css` に置き、
  パネルごとに書いていた `.help`・`.base-note`・`.tag`・`.badge`・`.item` を除きました (上の「エディタの中の共通の形」)。
- **押せる大きさ:** スロットの一覧・描く順の行・ポーズの要素の名前・光を受けるメッシュの行とチェックボックスを 24px に、光源の印には見える印 (半径 9) より広い
  透明な円 (半径 12) を重ねました。
- **重なり:** ポーズのタブの重なりは、閉じた「Links」の選ぶ欄 (`PickList` の `<details>`) の中身が、閉じていても位置を持って数えられていたもの。
  閉じている間は中身を描かないようにしました (パーツ 200 個の一覧を、閉じた欄の数だけ持たない)。
- **足すボタン:** パネルのボタンの文言の頭の「+ 」をやめ、`Icon name="plus"` にしました (上の表の「足すボタン」。タイムラインの「+ Event at …」は T0675)。
- **ツリー:** 行が無いときは `role="tree"` を出さず、何が無いかと次の操作 (「PSD から新規…」「PSD の取り込み…」) を出します (T0915 の ARIA の 1 件)。
  名前の変更は `TextField` (開いたら全体を選ぶ・押下を行に渡さないのは呼び手が持つ。部品に口は足していない)。表示の切り替えと道具の並びは、木を送っても上に残します。
- **状態:** ツリーの行の hover・選択を「状態の見せ方」のトークンにそろえ、ビューポートとホバーを共有しました (ツリーの行に乗るとメッシュに細い破線の輪郭、
  メッシュに乗るとその行に hover の地)。主な選択は太字 (ビューポートの太い輪郭に当たる)、本体の要素の選択は灰色の破線の枠 (ビューポートと同じ)。
  T1638 で、パーツ・デフォーマの行に乗るとその中のメッシュすべてに輪郭を出し、乗ったメッシュの行が畳んだ祖先の下にあるときは、
  いちばん近い見えている祖先の行に hover の地と左端の強調色の縦線 (3px。行そのものの hover と見分ける) を付けるようにしました。
- **送り:** パネルの中の見せずに読ませる文字 (`Notice` の中の `.ugoe-visually-hidden` など。`position: absolute`) が送る枠の外を位置の基準にして、
  文書の全体が縦に 300px ほど送れていた (`-bottom` の場面でヘッダーごと上にずれていた)。送る枠 (左の列・右の列の区画) を `position: relative` にしました。
  T0954 で `Notice` は自分を位置の基準にしましたが、この打ち消しは残します。パネルが自分で置く見せずに読ませる文字 (描く順の欄の名前・
  プレビューの表の見出し・光の欄の色の値とスライダーの名前) も同じ形で外へ出るためです (外すと `just ui-audit` の `overflow` に 4 種類出る)。
- **インスペクタ:** キーフォーム (と推定) をデフォーマの概要のすぐ後に上げ、メッシュの属性・デフォーマで包む操作・描画用の複製・深度を後ろにしました。
- **`just ui-audit --only editor`:** 前 (T0673 の後) の 12 種類 (重なり 3・24px 未満 8・ARIA 1) が消えて、すべての検査で 0 件です。
  パーツを 200 個・パラメータを 100 個に増やし、長い名前を混ぜた見本でも 0 件でした (見本の model.json を書き換えて `--no-build` で流した。場面の追加はしていない)。

残したもの:

| 場所 | 理由 |
| --- | --- |
| 光を受けるメッシュの一覧・選ぶ欄 (`PickList`) の中の縦の送り | 「1 つの領域に縦の送りは 1 つ」の例外。パーツ・メッシュが 200 個あると、パネルが一覧だけで埋まって後ろの欄に届かないため。行の途中で切って (9.5 行・7.5 行)、送れることを見せる |
| パラメータの追加・編集の欄 (`ParametersPanel`・`ParameterGroups`) | 部品の `label` を上に出す縦の形。ID・名前・最小・既定・最大を 1 つの組として見せる小さな入力の枠で、パネルの欄の並び (`.ed-field`) とは別の形にしている |

### ビューアの見直し (T0725)

ビューアの画面を「UI の品質の基準」で見直しました。`just ui-audit --only viewer` は、T0915 の基準線の 2 種類 (上) が消えて、すべての検査で 0 件です。

- **右の列:** 機能のパネル (トラッキング・着せ替え・ライティング・モーションと表情・ポーズ) を中身の高さのまま上の段に並べて段ごと送り、
  パラメータを下の段 (残りの高さ、最低 4 割) に置きました。前はパネルごとに高さの上限と中の送りがあり、トラッキングを開くとライティングなどが
  見出しの高さまで潰れていました (送りが 1 つの領域に重なる形でもあった)。トラッキングのパネル (`TrackingPanel`) が自分で持っていた
  高さの上限 (45%) と中の送りは、T0951 で部品から外しました (置く側が決める)。幅が 1200px 以上なら 2 つの段を横に並べ、どちらも上から下まで使います
  (モデルは縦長が多く、ビューポートの幅を減らしても表示の大きさはほぼ変わらない)。
- **ビューポート:** 拡大・縮小・全体の表示の道具をツールバーからビューポートの右下に移しました (狭い幅でツールバーの行を減らすため)。
  モデルの名前を持つ `role="img"` は、操作 (ドラッグ・ホイール) を受ける要素の中の、何も持たない別の要素に付けます
  (光源の印・空の状態のボタンを `role="img"` の中に入れない)。空の状態・準備中・失敗の知らせはテーマの地の札に載せます
  (ビューポートの地はモデルの背景の色でテーマに従わないので、ダークのテーマの文字を直に載せると読めなかった)。
- **空の状態:** 「モデルを開いていません」と、ドロップか選ぶかの 1 文、「ファイルを選ぶ…」「サンプルを開く」のボタンを出します。
- **狭い幅:** モデルの名前をツールバーから下の帯に移し、右の列を高さの 45% (前は 55%) にしました。360px のモデルの表示は高さ 250px ほどから
  300px ほどになります。
- **背景 (T0737):** 背景の色と「透明」を、ツールバーの 1 つの「背景」ボタン (今の背景の見本を出す。透明なら市松) の `Popover` にまとめました
  (どちらもまれな操作なので奥に置く)。360px のツールバーは 2 行 (開く・サンプル・覚えたもの / 背景・画像を保存・設定) になります。
- **パラメータ:** 絞り込みは名前・ID・グループの名前を、空白で区切った語のすべてで引きます (`@ugoe/player` の `visibleSections`)。
  合うものが無ければそう出し、グループの見出しは送っても上に残します (`position: sticky`)。
- **光源の印のフォーカス:** 見える輪は印に合わせた円で描き (四角い `outline` は丸い印に合わない)、`:focus-visible` に透明の `outline` を付けます。
  透明の `outline` は強制色のモード (forced-colors) で色が付いて輪になり、`just ui-audit` の印の判定 (outline・box-shadow を見る) にも数えられます。

残したものはありません。

### スタジオの操作ウィンドウ (T0817)

「配置」と「レイアウト」の基準で、スタジオの操作ウィンドウを見直しました (詳細は [docs/studio.md の「操作ウィンドウ」](studio.md#操作ウィンドウ))。
`just ui-audit --only studio` は前も後も 0 件で、残したものはありません。

- **情報の優先度:** 配信中に触るもの (表情とモーション・ポーズ) を右の列の奥から中央の列へ移し、中央の列の頭に状態の印を並べました。
  中央の列は前は広いのに中身が少なく、右の列が長く送っていた (T0915 の振り分け)。設定 (シーンの編集・入力・ホットキー・出力・API) は右の列。
- **状態の印:** ツールバーの「API」「ソース」の札 (文だけ・入のときだけ) と負荷の文を、「いまの状態」の印 (`StatusBadge.svelte`) にまとめ、
  トラッキング・口パク・ホットキーを足しました。形 (丸にチェック・中の空いた丸・三角・八角) と文で示し、意味は Tooltip。切っているものも出します
  (切っていることも知っているべき状態のため)。
- **幅:** 列は 13.5〜17.5rem・20rem〜・20〜28rem。960px 以下は 2 列 + 下に設定、640px 以下は 1 列 (画面の半分 (960px) に置いても、シーンと表情が送らずに見える)。
  ツールバーはプロファイルの名前を省略して、360px でも 2 行。
- **シーンの操作:** 5 つの文字のボタン (2 行に折り返していた) を、「追加」と 4 つの `IconButton` (複製・上へ・下へ・削除) の 1 行に。
  各シーンに切り替えのホットキー (`Kbd`) を並べました。
- **誤操作:** シーンの削除は前から確かめのダイアログ。操作 API の承認は、承認の後に「取り消す」の知らせを出します。上の帯のエラーは閉じられる `Notice` にしました。
- **自前のアイコン:** 複製 (重ねた 2 枚)・削除 (ごみ箱)・切の印 (中の空いた丸に横線) は、スタジオの 1 か所だけなので `Icon` に足さず、16 の格子で描きました。

### スタジオの設定の流れと出力ウィンドウ (T0818)

初めて起動してから配信ソフトに取り込むまで (モデルを選ぶ → 出力ウィンドウ → 大きさ → 保存 → 入力 → 取り込み) を画面でたどり、
「空の状態」「状態の見せ方」「エラー」の基準で直しました (詳細は [docs/studio.md の「操作ウィンドウ」](studio.md#操作ウィンドウ))。
`just ui-audit --only studio` は前も後も 0 件で、残したものはありません。偽のホストに初めて起動した画面・承認を待つアプリ・
登録できないホットキー・負荷の注意の場面と、960px の組 (`en-light-half`) を足しました ([tests/ui-audit/README.md](../tests/ui-audit/README.md))。

- **行き止まり:** 新しいプロファイル (どのシーンも配信していない) で「シーンの編集」からモデルを選んでも、何も出なかった
  (「このシーンに切り替える」の知らせを押す必要があった) のを、そのシーンに切り替えて出すようにしました。モデルの無いシーンには切り替えの知らせを出しません。
  一度も保存していないプロファイルが「保存済み」と出ていたのを「未保存」に、名前の空のシーンの名前の欄に表示名 (「シーン 1」) を placeholder で出します。
- **並び:** 右の列を準備の順 (シーンの編集・入力・出力ウィンドウ・ブラウザソース・仮想カメラ) にし、一度決めれば触らないもの (ホットキー・
  モーションの設定・操作 API・負荷・パラメータ) を後ろに。「表情とモーション」の中の設定 (待機モーション・トラッキング中の優先) は配信中に触らないので、
  右の列の「モーションの設定」に分けました (T0817 の候補)。
- **出力ウィンドウの様子:** 出力ウィンドウには何も重ねないので、操作ウィンドウに出します。「いまの状態」に出力の印 (表示中なら大きさ、
  隠していれば「非表示」、OS が小さくしたら三角。Tooltip に枠と背景)、「出力ウィンドウ」のパネルに「いまの大きさ」「枠」「背景」(配信しているシーンの背景) と
  取り込み方の案内。OS が小さくしたときの注意は、補足の文から `Notice` (warning) にしました。
- **ホットキーの問題:** 登録できない数をパネルの頭に `Notice` で、その行に左の線と三角の印と理由を出します (前は危険の色の文だけ)。
  ぶつかりの理由は、行に番号を出していないのに「ホットキー n と同じキー」と書いていたので、相手を操作の名前 (「シーンの切り替え: 雑談」) で示します。
- **ダイアログと知らせのボタン:** エディタのダイアログ (`NewModuleDialog`・`BaseChangeDialog` など) の「取り消し・保留が先、決定 (`primary`) が並びの最後」に
  そろえ、操作 API の承認の知らせを「後で」「拒否」「承認」の順に。設定のダイアログの下にエディタの設定と同じ「閉じる」を置きました。
  確かめ (シーンの削除・プロファイルを捨てる・トークンと URL の作り直し・許可の取り消し) は前から `AlertDialog` (取り消しが先で、既定のフォーカスも取り消し)。
- **狭い幅:** 承認の知らせの `SegmentedControl` の名前 (「Permission」) が 360px で字の途中で折れていたので、名前を折らずに選択肢を次の行へ送ります
  (T0950 で部品の側で直し、スタジオの CSS を外しました)。
- **ホットキーの削除の取り消しと、印から設定へ (T0845):** 行の「削除」はすぐ消していた (戻すには足し直す) ので、パネルの頭に「削除しました」の
  `Notice` と「元に戻す」を出し、同じ位置に戻します (Toast は操作を持たないので `Notice`。押したボタンが行ごと消えるので、フォーカスは「元に戻す」へ)。
  「いまの状態」の印は Tooltip で設定の場所を言うだけだったので、ボタンにして (枠を操作の部品の色に)、押すとそのパネルを開いて送り、見出しにフォーカスを移します。
  `just ui-audit --only studio` は前も後も 0 件 (場面 `-hotkey-removed` を足した)。

### エディタの外枠 (T0673)

「配置」「レイアウト」の基準で、エディタのヘッダー・列の配置・ダイアログ・空の状態・読み込み中と失敗の見せ方を見直しました
(詳細は [docs/editor.md の「画面の配置」](editor.md#画面の配置)・[「ダイアログ」](editor.md#ダイアログ)・[「読み込み中と失敗の見せ方」](editor.md#読み込み中と失敗の見せ方))。

- **ヘッダー:** 13 個の文字のボタンが平らに並び、1024px で 2 行・200% の拡大で 3 行に折り返していた (T0915 の振り分け)。ファイルの操作を「File」の
  `Menu` にまとめ (新しいモジュールと見本の説明は項目の説明の行)、よく使う保存・書き出し・Undo・Redo だけをヘッダーに残し、版は設定の「一般」へ
  移しました。1024px・200% の拡大でも 1 行です。保存・別名で保存・開くのショートカット (Ctrl / Cmd + S・Shift + S・O) を足しました。
- **列の大きさ:** 左右の列の幅とタイムラインの高さも、右の列の上下と同じく区切り (`Splitter`) で変えて覚えます。区切りは 1px の線で、押せる範囲を前後に
  広げ、フォーカスの輪を出します (前の右の列の区切りは輪の代わりに地の色を変えていた)。
- **右の列のタブ:** パネルの見出し (`<h2>`) がタブと同じ名前で重なっていたので、見た目では隠しました (読み上げと見出しの移動には残す)。
  8 つのタブは既定の幅 (320px) では 2 行です。英語で 1 行に並べると 470px ほど要り、1024px の窓ではビューポートが狭くなりすぎるので、既定の幅は変えず、
  列を広げれば 1 行になる形にしました (タブを減らす・「その他」のメニューに送るかは、使い方を見て決める)。
  T1632 でアイコンだけのタブにして、既定の幅でも 200% の拡大でも 1 行にしました ([「アイコン」](#アイコン)・下の「エディタの右の列のタブ (T1632)」)。
- **空の状態:** 空のプロジェクトのビューポートは文だけ (「New from PSD… で始める」) だったのを、PSD から新規・開く・見本を開くのボタンにしました
  (編集の道具は出さない)。パラメータの無いモデルのパラメータ一覧に「パラメータを追加」を足しました。
- **ダイアログ:** 並び (取り消し・後で が先、決定が最後)・`primary` は 1 つまで・取り消せない決定は `danger` で `primary` なし、は前からそろっていました。
  文字の欄の Enter で決定 (新しいモジュール・モジュールの設定)、ライセンスの閉じるボタン、設定の「一般」の見出しの重なりを直しました。
  ライセンスは、ビューア・スタジオと同じく設定 (「一般」の「ugoe のエディタについて」) から開けます (前はプレビューの中だけ)。
- **読み込み中と失敗:** ファイルの読み書きの間はヘッダーに「保存しています…」などの文 (`role="status"`)、編集の失敗は右の列の上の文だけだったのを
  閉じられる `Notice` の `danger` (見出しと直し方) にしました。
- **`just ui-audit --only editor`:** 見直しの前と後で、範囲 (ヘッダー・列・ダイアログ・空の状態) のコントラスト・はみ出し・押せる大きさ・名前・Tab の止まりは
  0 件のままです。200% の拡大の `webgl2` の重なり (T0675 の範囲) は、左の列が窓の幅で抑えられて広くなったビューポートで消えました。
  残る重なり (ポーズのタブ)・24px 未満 (スロット・ライティング・ポーズの一覧)・ARIA (空のツリー) はパネルの中で、T0674 で直します。

### エディタのビューポートとタイムライン (T0675)

「配置」「レイアウト」「状態の見せ方」の基準で、エディタのビューポート (道具の並び・HUD・説明の帯・ギズモ) とタイムライン
(ドープシート・グラフ・再生の操作) を見直しました (詳細は [docs/editor.md の「ビューポートの並び」](editor.md#ビューポートの並び)・
[「ビューポートの操作と代わりの手段」](editor.md#ビューポートの操作と代わりの手段)・[「タイムライン」](editor.md#タイムライン))。

- **道具の並びと説明の帯:** フォーム編集とギズモの帯が道具の並びの下に `top: 40px` で置かれていて、並びが折り返す (狭い幅・200% の拡大) と重なり、
  文字 (「キーの上にいない」の注意を含む) がモデルの上に直に載って読めなかった。道具のボタンを 1 行目に集め (「Form edit」も帯から並びへ)、
  その下に帯を縦に積んでテーマの地と枠に載せました。注意は三角の印と `warning` の色。
- **HUD:** 右上は「Screenshot」と GPU の知らせだけにし、描画の方式 (`webgl2`) は設定の「一般」へ (開発者向けの情報)。拡大・縮小・全体を表示は
  ビューアと同じ右下の道具にしました (前は「Fit」だけで、ホイールの代わりが無かった)。T0915 の基準線の `webgl2` と道具の並びの重なりは、元から無くなります。
- **図の色:** 上の「ビューポートの上の図」。ダークの地で点が暗く (中が `surface`)、ワープの格子 (`#0ea5e9`) はライトの地に 2.2 でした。
  T0674 で足したホバーの輪郭 (ツリーの行と共有) も `--viewport-edit` の 1px の破線にし、`opacity` で薄くするのをやめました (地に 3:1 を割るため。選択とは太さと破線で分ける)。
  ライティングの今の光源の取っ手の縁も、強調色から `--viewport-edit` にしました。
- **代わりの手段:** ビューポートの操作ごとに一覧にして確かめ、点のドラッグに矢印キー (1px・Shift で 10px) を足しました (docs/editor.md の表)。
  移動・頂点を足す・回転と拡大率の取っ手・グルーの 1 組ずつは代わりが無く、後続の候補にしました。
  移動 (Alt + 矢印) と点を 1 つずつ選ぶこと (`]` `[`) は T1635 で足しました。
  回転と拡大率の取っ手の代わりは T1636 でインスペクタの数値の欄を足しました。
  原点のドラッグには矢印キーに加えて、T1649 で同じ節に原点の X・Y の数値の欄を足しました (「Origin: all cells」に従う)。
- **タイムライン:** モーションの上の段の 9 つの欄が 1440px でも 2 行、1024px で 3 行に折り返していた (T0915 の振り分け)。名前・長さ・ループ・フェードを
  「Settings…」の `Popover` に移し (まれに変える。長さは選択の名前と「/ 4 s」にも出ている)、再生と繰り返しをアイコンのボタン、コマ送りと時刻を 1 組にして、
  上の段と再生の段を 1024px でも 1 行ずつにしました。表情の上の段も同じ形です。タイムラインのボタンの「+ Event at …」は、パネル (T0674) と同じく
  文言の「+ 」をやめて `Icon name="plus"` にしました。
- **キーの形とグラフ:** キーの印の形を入る区間の種類 (直線は菱形・ベジェは円・ステップは四角) にし、グラフの左端に上端と下端の値を出しました。
  グラフの左の欄 (前は 6 行。区間の種類の 4 つのボタンが 2 行に折り返していた) を 4 行 (区間の種類は `Select`) にし、
  1024×720 でタイムラインとグラフを出しても、2 つの段・表の 2 行・グラフが送らずに見えます (前はグラフが画面の下に隠れた)。
  頭の段は送らずに残し、表だけを送ります (それでも収まらない 200% の拡大ではパネルの全体を送る)。
- **`just ui-audit --only editor`:** 見直しの前と後で、範囲 (ビューポート・タイムライン) のコントラスト・はみ出し・重なり・押せる大きさ・名前・Tab の止まりは 0 件です。
  タイムラインのグラフ (`-timeline-graph`)・モーションの設定 (`-motion-settings`)・ワープのギズモ (`-gizmo`)・メッシュの編集 (`-mesh-edit`) の場面を足しました。
  T0674 の後 (main に rebase して) は、ほかの 12 種類も 0 件です。足した `-gizmo` の場面で 24px 未満として新しく見えるようになった 4 種類
  (インスペクタのキーフォームの軸の「キーへ移る」「キーを消す」(高さ 16px) とパラメータの下のキーの印 (8px)。パネルの中。T0674 の場面では
  デフォーマを選んでいなかった) は、下の「キーフォームの軸とキーの印 (T1634)」で直しました。

### キーフォームの軸とキーの印 (T1634)

- **キーフォームの軸:** インスペクタの軸のキーの「キーへ移る」(数) と「キーを消す」(×) を、高さと幅 24px 以上にしました (札は 2px 高くなる)。
- **キーの印:** パラメータのスライダーの下のキーの印 (◆ 6px) は見た目をそのままに、押せる範囲を 24×24 の透明な枠にしました。印はスライダーの
  下の帯に並べ、帯の上の 4px はスライダーの下の余白に、下の 8px は行の間の余白に重ねます (帯そのものは押せず、重なったスライダーを押せる)。
  キーが近いと隣の印と押せる範囲が重なります。重なった所では、上に描かれた印ではなく、押した所に中心が近い方の印のキーへ移ります
  (`parameterValue.ts` の `nearestMarkerKey`)。キーボードで押したときはフォーカスのある印のキーです。
- **`just ui-audit --only editor`:** 前の 4 種類 (`button.go`・`button.remove`・`button.marker`・`button.marker.current`) が消えて、すべての検査で 0 件です。

残したものはありません。

### エディタの空のプロジェクトと狭い窓のタイムライン (T1637)

- **前:** 空のプロジェクトでも、使わないタイムラインが中央の下の高さ (`clamp(280px, 42vh, 520px)`) を取り、200% の拡大 (720×450) では
  ビューポートの空の状態の 3 つ目のボタン (「Open Sample…」) がタイムラインに隠れていた。200% の拡大の中央の列は 260px ほどで、
  タイムラインを開いても表のキーの列は 100px ほどだった。
- **後:** 空のプロジェクトと幅 900px 未満の窓ではタイムラインを畳んで始め、ビューポートに高さを渡します。1024px 以上で中身のある
  プロジェクトは前のまま。利用者が開閉を押した後は押した開閉に従い、自動で畳んだことは覚えません ([docs/editor.md の「画面の配置」](editor.md#画面の配置))。
  左右の列を狭める案 (A) も比べましたが、200% の拡大では右の列が窓の幅の 36% で既に下限より狭く、表は約 22px しか広がらず、
  1024px ではツリーの名前が省略されるので採りませんでした。
- **`just ui-audit --only editor`:** 前 (main 376f1cd1) と後で +0 -0 ~0。監査の場面はタイムラインのグラフ (`-timeline-graph`) の前に、畳まれていれば開きます。

### canvas の上の文字と 1600px の組 (T0952)

T0725 で、ビューアのビューポートの知らせ (空・準備中・失敗) がダークのテーマで読めなかったのを、`just ui-audit` は見つけられませんでした。
知らせの祖先の不透明な地 (ページの地) をたどって測り、間に挟まる canvas (地はモデルの背景色) を見ていなかったためです。

- **canvas の上の文字:** 文字の不透明な地の祖先の中に、文字に重なる canvas・video・img があるもの (と、祖先に画像の地・グラデーション・hover の重ねが
  あるもの) は、画面のすべての文字を透明にして撮った画素を地として測ります。矩形の中の地の画素ごとに比を求め、下から 1 割の比を文字の比にします
  (縁や模様の数画素で落とさないため)。モーダルのダイアログの後ろの文字は操作できない (WCAG の inactive) ので数えません。
  T0725 の前 (main e9ef06f9) のビューアでは、ダークの 10 の場面で知らせ (`p.hint`) が 2.07 (text-muted が明るい背景色の上) と出ます。
- **1600px の組:** ビューアとスタジオに `en-light-xwide`・`ja-dark-xwide` (1600×900) を足しました。ビューアは 1200px 以上で右の列を 2 列に並べ、
  スタジオは中央の列だけが伸びるので、広い窓での伸び方を撮ります。エディタの広い幅は 1440px で、列の幅は区切りで変えて覚えるので足していません。
  ビューアとスタジオの 1600px の組は、すべての検査で 0 件です。

エディタは、画素で測るようにした当初、ビューポートの右上の `webgl2` が地の札なしでモデルの髪に重なり 4.31 でした (狭い幅と 200% の拡大のライト)。
T0675 で描画の方式の表示が設定の「一般」へ移り、今はコントラストの指摘が 0 件です。

canvas と hover の重ねの上の文字を測るようにして、スタジオのシーンの一覧の選んだ行の hover が見つかりました
([スタジオのシーンの一覧の選んだ行 (T0851)](#スタジオのシーンの一覧の選んだ行-t0851) で直しました)。

### スタジオのシーンの一覧の選んだ行 (T0851)

- **前:** ダークで、選んだ行 (地は `--ugoe-color-accent-soft`) に `--ugoe-overlay-hover` が重なると、text-muted (モデルの名前・ホットキーの `+`) が
  4.29 でした (`ja-dark-narrow-approved`。承認のボタンを押した後のポインタの位置が行の上に来る)。押下の重ねでは 3.63 でした。
- **後:** 選んだ行の中の控えめな文字 (モデルの名前・ホットキーの `+`・「読み込み中」) を、text-muted に本文の色を 30% 混ぜた色にします
  (`ScenesPanel.svelte` の `--scenes-muted`)。ダークで地だけ 6.33・hover 5.39・押下 4.56、ライトで 7.24・6.44・5.70 です。
  選んでいない行の文字の色は前のままで、選んだ行の方が少し明るい (ライトでは濃い) だけなので、本文との段の差は残ります。
  hover の重ねのトークン (T0916) を `accent-soft` の上だけ弱める案は採りませんでした。重ねは地の色によらず同じ段で濃くなる決めで、
  `@ugoe/ui` の部品の accent-soft は、`Select`・`MenuEntries` の強調した項目には hover の重ねが乗らず、押された `Button` は重ねが乗っても
  文字が本文の色です。ビューアの記憶の一覧の今の行も重ねが無い (text-muted は accent-soft の上で 5.03) ので、困るのは選んだ行に
  hover を重ねてその中に text-muted を置く所だけだからです。エディタのツリーの、非表示の要素を選んだ行に hover したときの名前も同じ組み合わせ
  (ダークで 4.29) ですが、監査に場面が無く、範囲の外なので残しました。
- **`just ui-audit --only studio`:** 前 (main cf09669d) と後で +0 -2 ~0 (この行の 2 件が消えて、コントラストの指摘は 0 件)。トークンは変えていません。

### エディタの AI・モジュール・着せ替え (T0676)

「配置」「デザイン」の基準で、AI のタブ (提案と分類)・AI の設定と送る前の確認・プレビューの着せ替え (モジュールの違いの一覧) を
見直しました (詳細は [docs/editor.md の「AI の提案」](editor.md#ai-の提案)・[「AI の設定と送信の同意」](editor.md#ai-の設定と送信の同意)・
[「プレビューの着せ替え」](editor.md#プレビューの着せ替え))。

- **偽のホストの場面:** `just ui-audit` の偽のホストに、AI の提案と分類・外部に送れる設定・送る前の確認・答えない送信 (取り消すと
  `ai.cancelled`)・失敗 (`ai.time_limit`)・分類のキャッシュを戻している最中・着せ替えのモジュール (見本の `.ugoemod` と読めないファイル)・
  モジュールのプロジェクト (今の編集は見本の `.ugoemod` の名前を書き換えて作る) と違いの一覧の状態を足し、10 の場面を撮ります
  ([tests/ui-audit/README.md](../tests/ui-audit/README.md))。失敗の出し方を撮る場面は、画面のエラーの文を数えません (`errors`)。
- **提案のカード:** 何が変わるか (「採用で変わるもの」: 選んだコマンドの種類ごとの数を、利用者の言葉で「パラメータを足す × 5」。T1643) と、採用するとどうなるか (1 つの編集・1 回の Undo で戻る)
  を文で出し、ボタンを「却下・作り直し・プレビュー・採用」(決定が最後) に。`primary` は画面に 1 つ (提案が無ければ「Propose rig」、あれば
  採用のどれか 1 つ)。古い提案は札でなく `Notice` で理由と戻し方を出します。理由の文の中の縦の送り (10 行で切っていた) とコマンドの
  一覧の高さの上限をやめました (送りを重ねない)。
- **待ちの見せ方:** 実行中・送信中 (経った秒数と取り消し)・確認待ち・取り消した・失敗を、始めた場所に同じ形で出します (取り消しは
  `Notice` の info、失敗は `Notice` の danger。前は文の色だけで、カードの操作の失敗もパネルの頭に出ていた)。分類のキャッシュを戻している
  間の文を足しました (バックエンドに `ProposalState.restoring` と、戻せなくても送る終わりのイベント)。
- **秘密の扱い:** API キーの欄に見えるラベルと、保存先・送り先・書かない所の文を付け、削除を確かめるようにしました。送る前の確認に、
  送るのは表のものだけでキーは認証にだけ使うことを書きました。
- **着せ替え:** 失敗・警告を `Notice` (danger・warning) にそろえ (前は危険の色の文と、補足の色の警告が混ざっていた)、行のファイルを名前だけに。
  違いの一覧は要素の表・絞り込み・「さらに N 件」にしました。
- **`just ui-audit --only editor`:** 足した場面で見つかった 12 種類の重なり (閉じた `<details>` の中身が位置を持って数えられていたもの。
  閉じている間は中を描かない)・4 種類の 24px 未満 (コマンドの一覧のチェックボックス・設定のラジオ・規約のリンク・要素の ID と
  ライセンスの口)・ARIA の 2 種類 (送れる理由の文にキーボードが届かない) が消え、すべての検査で 0 件です。

残したものはありません。

### エディタの右の列のタブ (T1632)

T0673 で残した右の列の 8 つのタブ (既定の幅 320px で 2 行、200% の拡大で 3 行) を、利用者の判断でアイコンと Tooltip にしました
(案は「よく使うタブを並べて残りを『その他』の `Menu` に送る」「アイコンと Tooltip」「1 行のまま横に送る」の 3 つを試作して比べた。
経緯は docs/tasks/T1632-editor-right-column-tabs-overflow.md)。

- 8 つのタブ (1 つ 24px と間 2px) は既定の幅でも、200% の拡大の列 (中は約 235px) でも 1 行で全部が見えます。名前は読み上げの名前と Tooltip
  (ポインタとキーボードのフォーカスの両方で出る) で、Tooltip には「名前: 何をするタブか」を出します。絵の選び方は [「アイコン」](#アイコン)。
- AI の提案の数 (「AI (3)」) はアイコンの右の数字、見てほしい変化は右上の点と強調色で、読み上げの名前にも数と「要確認」を含めます。
- **`just ui-audit --only editor`:** 前と後で +0 -0 (前も後も 0 件)。

### スクリーンリーダーと OS の設定 (T0918)

3 つのアプリの読み上げの骨組み・状態の変化の知らせ・強制色・200% の拡大を見直しました。基準は上の [骨組み](#骨組み-ランドマークと見出し)・
[状態の変化の知らせ](#状態の変化の知らせ)・[強制色](#強制色)。

- **確かめ方:** `just ui-audit` に、ランドマークと見出しの検査 (`structure`)・強制色の組 (`en-dark-forced-wide`・`ja-light-forced-narrow`) と
  そこでの状態の見分けの検査 (`forced`)・日本語の 200% の拡大 (`ja-light-zoom200`)・場面ごとの骨組み (`.outline.txt`) を足しました
  (T0915 の候補)。強制色は Chrome の CDP の Emulation (`forcedColors: "active"`) で、`getComputedStyle` が強制した後の色を返すので、
  見た目の比べ方は自前で書けます。強制色の暗い組では Tab もたどり、`outline` で描いたフォーカスの輪が残ることを確かめました。
- **見つけて直したもの:**
  - 3 つのアプリに `<h1>` が無かった (見出しの移動の起点が無い)。ツールバーの中に見た目で隠したアプリの名前を置いた。
  - エディタの左右の列が名前の無い 2 つの complementary だった。「パーツとデフォーマ」「プロパティとパラメータ」の名前を付けた。
  - エディタの「検証」と「AI」のタブに `h2` が無く `h3` から始まっていた。ほかのタブと同じく、タブの名前の `h2` を見た目で隠して置いた。
  - スタジオの右の列のパネルが `h3` で `h2` が無く、入力のパネルの `h4` の中に共有のトラッキングのパネルの `h2` があった。パネルを `h2`、
    入力のまとまりを `h3` にし、`TrackingPanel` に `level` を足した (エディタのプレビューでは 3、スタジオでは 4)。
  - ビューアの「モーション」「表情」の見出しの中に「止める」のボタンがあり、見出しの名前に混ざっていた。見出しの外に出した。
  - 状態の変化を知らせる口が無かった。`announce()` を足し、AI の提案が届いた・スタジオのシーンが変わった・プロファイルを保存した・
    ビューアのモデルを開き終えた、を知らせるようにした。AI に送っている間の経った秒数が `role="status"` の中で毎秒変わっていたので、外に出した。
  - 強制色で、押したボタン・選んだ区切りの項目 (ビューアのライティングの「標準」など)・スイッチの入と切・スライダーの溝が見分けられなかった
    (地の色だけで描いていた)。`tokens.css` の強制色の節で系統の色にした。
  - 日本語の 200% の拡大で、エディタのヘッダーのモジュールのボタンが「プレビュー」に重なっていた。名前の欄が縮みきったらヘッダーを折り返すようにした。
- **`just ui-audit`:** 前と後で +0 -0。新しい検査と組を含めて、`structure`・`forced` は 3 つのアプリで 0 件 (残るのは前からあるスタジオの
  コントラストの 2 種類だけ)。`just e2e` (3 つのアプリ) も通りました。
- **確かめていないこと:** 実際のスクリーンリーダー (作業の環境の WSL2 に Orca が無い) と、実物の WebView の強制色・拡大。手順を
  [docs/manual-check.md の U03・U04](manual-check.md#u03-スクリーンリーダーで主な流れを読む-b-wml) に足し、利用者の確認に回しました。
  `.aria.yml` と `.outline.txt` は、読み上げの名前・役割・状態と骨組みの近似として見ました。

## 文言の書き方

### 置き場所と流れ

- 文言はパッケージ・アプリごとのカタログ `locales/<言語>.json` ([i18next JSON v4](https://www.i18next.com/misc/json-format)) に置きます。
  名前空間はパッケージの短い名前 (`ui`・`player`・`face-tracking`・`editor`・`viewer`・`studio`) です。
- 置き場所の一覧・訳す人の約束・言語の足し方は [docs/translation.md](translation.md) にあります。
- **原文は英語 (`en.json`)** です。文言を足す・変える PR では、日本語 (`ja.json`) の訳も同じ PR で足します。`just i18n` が欠け・余り・未使用のキーを見つけます。
- 画面の `.svelte` と画面の `.ts` に、表示する文言を直書きしません (`just i18n` が検査します。見つけるものと対象の外は
  [docs/translation.md の「直書きの文言の検査」](translation.md#直書きの文言の検査))。ログとテストの中の文字列は対象の外です。
- **画面に出る例外の文は `t()` で作ります** (`throw new Error(t("errors.modelMissing"))`)。画面は例外を `errorText(e)` でそのまま
  出すので、例外の文も直書きの文言として検査します。画面に出ない開発者向けの例外 (起こらないはずの状態など) は英語のまま、
  `i18n-ignore` と理由を書きます。
- OS のファイルダイアログに出るフィルタの名前 (`{ name, extensions }` の `name`) も文言です。言語を切り替えても追うように、
  定数にするときは読むたびに `t()` を引く getter にします (エディタの `lib/filePaths.ts` の `PSD_FILTER` など)。
- バックエンドのエラーはコードと引数で届くので、カタログの `errors.<code>` に訳を置きます (ADR 0029 の 4)。

### キーの名前

- キーは入れ子のオブジェクトで、`<画面または部品>.<要素>.<種類>` の順に書きます。各段は lowerCamelCase の英語です。

  ```json
  {
    "tracking": {
      "calibrate": {
        "label": "Calibrate",
        "tooltip": "Use the current face as neutral (facing forward, no expression)"
      },
      "recording": {
        "stop_one": "Stop recording ({{count}} frame)",
        "stop_other": "Stop recording ({{count}} frames)"
      }
    },
    "errors": {
      "camera": { "notAllowed": "The camera was not allowed (check the OS and app settings)" }
    }
  }
  ```

- 最後の段 (種類) は次から選びます: `label` (ボタン・欄の名前)・`tooltip`・`aria` (見えない読み上げの名前)・`placeholder`・`description` (補足の文)・`title` (ダイアログや画面の見出し)・`empty` (空の状態の案内)。
  1 つの要素に文言が 1 つしか無いときは、種類を省いてよいです。
- **キーは意味で付け、英語の文をキーにしません。** 英語の文を直しても、キーは変えません (訳が失われるため)。意味が変わったら新しいキーにします。
- **同じ英語でも、使う場所が違えばキーを分けます** (「Open」は「ファイルを開く」と「パネルを開く」で訳が違いうる)。共通の語 (`ok`・`cancel`・`close` など) だけを `ui` の `common.*` に置きます。
- 文を部品のつなぎ合わせで作りません (`t("a") + name + t("b")` は語順が言語で違うので訳せない)。引数を持つ 1 つの文にします。

### 引数

- 引数は名前付きの `{{name}}` です。位置 (`{{0}}`) は使いません。名前は訳す人が意味を推せるものにします (`{{path}}`・`{{model}}`)。
- カタログに HTML を書きません。Svelte が文字をエスケープするので、i18next のエスケープ (`escapeValue`) は切ります。
  文の一部だけを強調・リンクにしたいときは、文を分けずに済む形 (前後で別の要素にする・リンクを文の後ろに置く) を先に考え、必要なら `@ugoe/i18n` の差し込みの部品 `RichText` (訳文の `<link>…</link>` をスニペットで描く) を使います。

### 複数形

- 数で語形が変わる文は、引数 `count` を渡し、[CLDR の複数形の区分](https://www.unicode.org/cldr/charts/latest/supplemental/language_plural_rules.html) の接尾辞で書き分けます
  ([Plurals](https://www.i18next.com/translation-function/plurals))。英語は `_one` と `_other`、日本語は `_other` だけです。
- 0 のときに別の言い方をしたい場合 (「記録はありません」) は、複数形ではなく別のキー (`empty`) にして、コードで分けます。
- 数でない値 (名前・パス) に `count` を使いません。

### 数値・日付・単位

- 数値・日付・一覧は `@ugoe/i18n` の書式の関数 (`Intl` を今の言語で使う) を通します。`toFixed()` や文字列の連結で書式を作りません。
  - 数値: 桁の区切りと小数の桁数は関数に任せます。パラメータの値のような、編集のための数値は桁の区切りを付けず、小数点は `.` に固定します (入力と表示を揃えるため)。
  - 日付と時刻: `Intl.DateTimeFormat` の `dateStyle`・`timeStyle` を使い、並び順を自分で決めません。「3 分前」は `Intl.RelativeTimeFormat` を使います。
  - 単位: 秒・度・ピクセルは、`Intl.NumberFormat` の `unit` か、単位を含めたカタログの文 (`"{{value}} s"` / `"{{value}} 秒"`) で付けます。
- 一覧 (「A・B・C」と「A, B, and C」) は `Intl.ListFormat` を使います。

### 文体

| | 英語 | 日本語 |
| --- | --- | --- |
| ボタン・メニュー | 動詞で始める sentence case (`Save recording`)。末尾にピリオドを付けない | 短い名詞か動詞 (「保存」「忘れる」)。末尾に句点を付けない |
| 欄の名前・見出し | sentence case の名詞 (`Smoothing`) | 名詞 (「平滑化」) |
| 説明・エラー | 文。ピリオドで終える。何が起きたかと、次に何をすればよいか | 「です・ます」の文。句点で終える。何が起きたかと、次に何をすればよいか |
| 続きの入力がある操作 | 末尾に `…` (U+2026。`File…`) | 末尾に「…」(「ファイル…」) |

- 括弧は半角の `( )` で、前に空白を入れます (「カメラ (既定)」)。日本語の文の中の英数字の前後にも空白を入れます (「`.ugoe` を開く」)。
- 利用者を責める言い方 (「不正な」「間違った」) を避け、状態と直し方を書きます (「ファイルを読めません。ugoe のバンドル (.ugoe) かどうか確かめてください」)。
- 製品名・形式の名前 (ugoe・MediaPipe・`.ugoe`・OBS) は訳しません。
- 言語の切り替えの選択肢は、その言語自身の名前で出します (「日本語」「English」)。
