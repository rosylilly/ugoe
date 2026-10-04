# apps/viewer

ugoe のビューア (Tauri v2 + Svelte 5)。`.ugoe` バンドルか JSON のモデル (`model.json`) を開いて表示し、
パラメータを動かして確かめるだけの軽いアプリ。着せ替えのモジュール (`.ugoemod`) を付け外しして試せる (T0706)。ランタイムをアプリに組み込むときの見本と、
フェイストラッキング (T0703) の確認の場を兼ねる。

**ライセンスは GPL-3.0-only** (エディタと同じ。[ADR 0019](../../docs/adr/0019-viewer-license.md))。
エディタのコードは写してよいが、アプリ同士で import はしない。

画面の部品は `@ugoe/ui`、文言は `@ugoe/i18n` (名前空間 `viewer`。カタログは `locales/<言語>.json`、原文は英語) を使う
([ADR 0029](../../docs/adr/0029-shared-ui-and-i18n.md)・[docs/ui.md](../../docs/ui.md)。T0723)。強調色は 3 つのアプリで共通の紫、
密度は `comfortable` (`index.html` の `data-density`)。ライト・ダークは OS の設定に従う。

```
locales/              画面の文言 (en.json が原文、ja.json が日本語の訳。docs/translation.md)
src/                  フロントエンド (Svelte 5 + TS。npm パッケージ @ugoe/viewer)
  main.ts               トークン (@ugoe/ui/tokens.css) を読み、覚えた言語と OS の言語を決めてから App を描く
  App.svelte            ツールバー・ビューポート・右のパネル・ステータスバー、ドロップとショートカット
  lib/i18n.ts           文言の `t()` (名前空間 `viewer`)
  lib/settings.ts       モデルによらない設定 (画面の言語・テーマのキー) を localStorage に覚える
  lib/SettingsDialog.svelte  設定の画面 (言語: OS に合わせる・日本語・English)
  lib/LicenseDialog.svelte   ライセンスの画面 (public/third-party-notices.txt)
  lib/host.ts           ファイルの選択と読み込み・OS の言語 (Tauri / ブラウザ / 差し替え)
  lib/errors.ts         バックエンドのコマンドのエラー (コード) を今の言語の文にする (T0729。下の「コマンドのエラー」)
  lib/Viewport.svelte   描画領域 (ズーム・パン・フィット)。LightMarkers.svelte は重ねる光源の印 (ドラッグ・キーボードで光源を動かす、T0708・T0714)
  lib/ParameterPanel.svelte  パラメータのスライダー (グループごと)
  lib/LightingPanel.svelte   光源のパネル (ライティングを使うモデルのときだけ出す)。LightPad.svelte は向き・位置の面
  lib/DressUpPanel.svelte  着せ替えのパネル (スロットのあるモデルか、モジュールを読み込んだときだけ出す)
  lib/MotionPanel.svelte  モーションと表情のパネル (バンドルにモーションか表情があるときだけ出す。T0720)
  lib/MemoryDialog.svelte  モデルごとに覚えたもの (光源・着せ替えの組・待機モーション) の一覧の画面 (T0714)
  lib/drop.ts           ドロップしたファイルの開き方 (モデルとモジュールを一緒に落としたとき。T0713)
  lib/samples.ts        同梱の見本の一覧と開き方 (ツールバーの「サンプル」)
  lib/screenshot.ts     画像の保存の既定のファイル名 (T0420。画素の検査と PNG への変換は @ugoe/player の screenshot.ts、T0730)
  lib/wasm/             ugoe-wasm の読み込み (`Player` に `loadWasm` として渡す)。generated/ は `pnpm wasm` の出力 (gitignore 済み)
  global.d.ts           `window.__UGOE_VIEWER__` の型
public/               そのまま配る静的ファイル。third-party-notices.txt (同梱物のライセンス。`node scripts/third-party-notices.mjs` が生成する) だけコミットする
  mediapipe/            MediaPipe の wasm とモデル。`pnpm mediapipe` が用意する (gitignore 済み)
src-tauri/            バックエンド (Rust クレート `ugoe-viewer`)
```

モデルの再生の層 (`Player`。読み込み・パラメータ・モーションと表情・トラッキング・ライティング・着せ替え・モデルごとに
覚えるもの) は、スタジオと共有するパッケージ `@ugoe/player` ([packages/player/](../../packages/player/)) にある (T0904)。
このファイルで `player.svelte.ts`・`drivers.ts`・`lighting.ts`・`dressUp.ts`・`dressUpMemory.ts`・`animation.ts`・
`loadWarnings.ts`・`modelMemory.ts`・`rememberedModels.ts`・`modelFile.ts` と書いているものは、そのパッケージの `src/` にある。
ビューアは `App.svelte` で `new Player({ loadWasm, readFile: readFileAt })` を作る。

フェイストラッキング (T0703) の部品 (MediaPipe の結果の変換・キャリブレーション・平滑化・記録・`FaceTracker`・
`TrackingPanel`) と、MediaPipe の wasm を写しモデルを取得して SHA-256 を検証するスクリプト (`ugoe-mediapipe-assets`) は、
エディタのプレビューと共有するパッケージ `@ugoe/face-tracking` ([packages/face-tracking/](../../packages/face-tracking/)) にある (T0636)。
ビューアは `Player` を `TrackingTarget` として渡し、記録の保存は `host.ts` の `saveJsonFile`。文言はそのパッケージの名前空間 `face-tracking` で、
ビューアの言語に従う。そのパネルはまだ `@ugoe/ui` の部品に移していないので、`app.css` で古い変数 (`--muted` など) をトークンへの別名にし、
標準のボタンを装っている (パネルを移したら消す)。

## 仕組み

- 描画もパラメータの操作も、webview の中の `ugoe-wasm` (`WebPuppet`) で行う (エディタのビューポートと同じ、
  [ADR 0003](../../docs/adr/0003-editor-viewport-in-webview.md))。WebGPU が無ければ WebGL2 で描く。
- バックエンドは webview だけではできないことに限る: ファイルのダイアログを開く (`open_file_dialog`・`save_file_dialog`)、
  モデルとモジュールのファイルをバイト列で読む (`read_model_file`。拡張子が `.ugoe`・`.json`・`.ugoemod` のものだけ) こと、
  トラッキングの記録を JSON で書く (`write_json_file`。`.json` だけ) こと、
  画像 (スクリーンショット) を PNG で書く (`write_png_file`。本文が PNG のバイト列、パスはヘッダー `x-ugoe-path`。`.png` で、中身が PNG のものだけ) こと、
  起動時の引数のパスを返す (`initial_file`) こと、webview からのカメラの許可の要求に応じること、OS の言語を返すこと
  (tauri-plugin-os の `locale()`。権限は `os:allow-locale` だけ)。どのパスを読み書きできるかは下の「[CSP とコマンドの許可](#csp-とコマンドの許可-t0727)」。
  依存してよいのはランタイム層のクレートだけ (`scripts/check-layers.mjs`)。
  ファイルを読み書きする 3 つのコマンド (`read_model_file`・`write_json_file`・`write_png_file`) とダイアログの 2 つのコマンドは `async` のコマンドで、
  読み書きとダイアログは `tauri::async_runtime::spawn_blocking` の別のスレッドで行う (`lib.rs` の `blocking`)。同期のコマンドは Tauri の
  メインスレッドで動き、大きなバンドル (上限 512 MiB) やネットワークのドライブを読む間ウィンドウが止まるため (T0726)。
  `read_model_file` は大きさを見てから上限 + 1 バイトまでに限って読むので、読む間にファイルが大きくなっても上限を超えて読まない。
  通常のファイル (リンクは辿る) だけを読み、装置 (`/dev/zero`)・FIFO・ディレクトリは開かずに `file.read_failed` (`not a regular file`) にする
  (`read_file.rs`、T0011。エディタ・スタジオと同じ形の写し。読み続けてメモリを使い果たす・開くだけで止まるため)。
  いまは内部のクレートに依存していない。
- 形式は中身で判別する (先頭が `UGOE` ならチャンク表を見て、`MODL` があればバンドル、`MODU` があれば着せ替えのモジュール。
  `{` なら JSON)。決まらなければ拡張子で決める (`modelFile.ts`)。
- 見本はリポジトリの `assets/samples/` の `.ugoe` と `.ugoemod` を写さずに使う。`vite.config.ts` の `sampleModels` が開発サーバーで
  `samples/<パス>` として配り、ビルドの出力に入れる (合わせて約 3 MB)。メニューに出す見本と一緒に読み込むモジュールは `samples.ts` の `SAMPLES`。
  ブラウザでは `?model=samples/avatar.ugoe` でも開ける。
- パラメータの値域・既定値・グループは `WebPuppet.parameters_json()` から得る。

## 使い方

| 操作 | 内容 |
| --- | --- |
| 開く (Ctrl+O / ⌘O) | OS のダイアログで `.ugoe`・`.json` か `.ugoemod` を選ぶ (`.ugoemod` は読み込んで付ける) |
| サンプル | 同梱の見本 (`assets/samples/`、MIT) をメニューから選んで開く (メニューには名前だけを出す)。「アバター」はアバタープロファイルのレベル 3 の見本で、フェイストラッキングをすぐ試せる。「アバター (ライティング)」は同じ見本に法線マップを付けた版で、ライティングのパネルが出る。「着せ替え」はスロットを持つ本体と衣装のモジュール 4 つを読み込み、`LongHair`・`CatEars`・`Sailor` を付ける (`TwinTails` はパネルで差し替えて試す) |
| ファイルをウィンドウにドロップ | そのファイルを開く。`.ugoemod` は読み込んで付ける。モデルとモジュールを一緒に落とすと、モデルを先に開く。フォルダや開けないファイルが混ざっていたら、開けるものだけを開き、飛ばしたものをステータスバーに出す。落とす前 (ドラッグ中) に、開けるかを枠の色と文で示す (アプリはパスから。T0717。ブラウザは名前が分からないので種類と数から。T0718) |
| `ugoe-viewer path/to/model.ugoe` | 起動時に開く |
| ホイール | カーソルの位置を中心にズーム |
| ドラッグ (左・中ボタン) | パン。光源の印 (ライティングを使うモデル) を掴んだときは光源を動かす |
| ダブルクリック・F・Fit | 全体を表示 (光源の取っ手にフォーカスがあるときも F はフィット。T0717) |
| 覚えたもの | モデルごとに覚えた光源・着せ替えの組・待機モーションの一覧。モデルごとに・すべて忘れる (下の「[覚えたものの一覧](#覚えたものの一覧-t0714)」) |
| 背景・透明 | 背景色。透明にすると市松模様の上に描く |
| 画像を保存 | 今の表示を PNG で保存する (下の「[画像の保存](#画像の保存-t0420)」) |
| 設定 (ツールバーの歯車) | 画面の言語を「OS に合わせる」(既定)・「日本語」・「English」から選ぶ。すぐに切り替わり、`ugoe-viewer.locale` に覚える。OS の言語は Tauri では tauri-plugin-os の `locale()` で取り (WebView の `navigator.languages` は OS の言語と違うことがあるため)、ブラウザでは `navigator.languages` を使う。対応していない言語なら English。テーマ (「OS に合わせる」(既定)・ライト・ダーク) も選べ、`ugoe-viewer.theme` に覚える (`@ugoe/ui` の `ThemeSetting`) |
| 着せ替え | スロットのあるモデルか、モジュールを読み込んだときに右のパネルに出る。スロットごとにモジュールを付ける・外す・差し替える (下の「着せ替え」) |
| ライティング | ライティングを使うモデルのときだけ右のパネルに出る。光源の向き・位置・色・強さと既定の光源の組 (下の「ライティング」) |
| モーションと表情 | バンドルにモーションか表情があるときに右のパネルに出る。モーションを優先度を選んで再生・停止し、表情を付け外しする (下の「モーションと表情」) |
| スライダー | パラメータ (`@ugoe/ui` の `Slider`。数値の欄で値を入れられる)。ダブルクリックか ↺ で既定値、「すべて戻す」で全部を既定値に戻す。中継用のパラメータ (`internal`、物理のリグの間の受け渡し用) は既定で隠し、「中継用のパラメータも表示」で出す |

## CSP とコマンドの許可 (T0727)

webview で思わぬスクリプトが動いても、外へ送れず、利用者が選んでいないファイルを読み書きできないようにする。
エディタ (T0686)・スタジオ (T0820) も同じ形で、CSP は 3 つのアプリで同じ値にしている (T0024。3 つの違いの表と理由は
[docs/architecture.md](../../docs/architecture.md#webview-の-csp-とコマンドの許可))。

### CSP

`src-tauri/tauri.conf.json` の `app.security.csp`:

| 指示 | 値 | 理由 |
| --- | --- | --- |
| `default-src` | `'self'` | 同梱のものだけ (`samples/`・`mediapipe/`・`third-party-notices.txt` もここ) |
| `script-src` | `'self' 'wasm-unsafe-eval'` | `ugoe-wasm` と MediaPipe の wasm をコンパイルするため (`'unsafe-eval'` は要らない)。MediaPipe のローダー (`mediapipe/wasm/*.js`) は同じ場所から `<script>` で読む。`index.html` のテーマのインラインのスクリプトは、Tauri がビルドのときに sha256 を足す |
| `style-src` | `'self' 'unsafe-inline'` | Svelte の `style:` と部品の `style` 属性。Tauri が style-src に nonce を足すと `'unsafe-inline'` が効かなくなるので、`dangerousDisableAssetCspModification: ["style-src"]` で style-src だけは書き換えさせない |
| `img-src` | `'self' blob: data:` | トラッキングに画像のファイルを使うとき (`URL.createObjectURL`)・CSS の `data:` |
| `media-src` | `'self' blob:` | トラッキングに動画のファイルを使うとき。カメラ (`srcObject` の MediaStream) は CSP の対象外 |
| `connect-src` | `ipc: http://ipc.localhost 'self'` | 同梱のファイルの `fetch` と Tauri の IPC (Linux・macOS は `ipc:`、Windows は `http://ipc.localhost`) |
| `object-src` | `'none'` | |
| `base-uri` | `'self'` | |
| `form-action` | `'none'` | フォームを送らない |
| `frame-ancestors` | `'none'` | |

- `worker-src`・`font-src` は書かない (Worker を作らず、フォントを同梱しない)。

- WebGPU と WebGL2 は CSP の指示を要らない。
- 開発 (`just viewer`) では devUrl (Vite) のページを直接開くので、デスクトップでは CSP が掛からない (Tauri は自分のプロトコルで配る
  ページにだけ付ける)。CSP を確かめるときはビルドしたもの (`cargo build -p ugoe-viewer --features tauri/custom-protocol` で
  `dist/` を埋め込んだもの) で確かめる。

### コマンドの許可

- `src-tauri/src/acl.rs` にアプリのコマンドを並べ (`COMMANDS`。`generate_handler!` と同じものを同じ順に)、`src-tauri/build.rs` が
  `include!` して `tauri_build::AppManifest` に渡す。`capabilities/default.json` で `main` のウィンドウに `allow-<コマンド名>` を与える。
  並べていない・許していないコマンドは webview から呼べない。コマンドを足したら `generate_handler!`・`COMMANDS`・capability の 3 か所に書く
  (`acl.rs` のテストが食い違いと CSP の要の値・3 つのアプリの CSP が同じことを検査する。T0024)。
- ファイルのダイアログはバックエンドが開くので、`dialog:allow-open`・`dialog:allow-save` は与えない (webview は tauri-plugin-dialog を直接呼べない)。
  文言と拡張子のフィルタは画面がコマンドの引数で渡す。
- `src-tauri/permissions/autogenerated/` はビルドで作られるので、ルートの `.gitignore` で無視している (3 つのアプリとも同じ)。

### ファイルの読み書きの範囲

`lib.rs` の `FileAccess` が、読み書きしてよいパスを覚える。

- **書き込み:** `save_file_dialog` で利用者が選んだパス (拡張子が無ければバックエンドが `.json`・`.png` を付けたもの) にだけ、
  1 回書ける。書いたら忘れる (同じ場所にもう 1 回保存するときは、またダイアログで選ぶ)。保存できる形式は `json` と `png` だけ。
  - 書くときは、同じディレクトリの一時ファイル `.<名前>.<プロセス ID>.<連番>.tmp` に書いて `sync` してから、名前を変えて
    置き換える (`src-tauri/src/atomic_write.rs`、T0728)。途中で失敗したら (ディスクがいっぱい・ネットワークのドライブが切れる・
    置き換えられない) 一時ファイルを消してエラーを返し、上書きしようとした前のファイルはそのまま残る。Windows でも
    `std::fs::rename` が既にあるファイルを置き換える。アプリが落ちたときは一時ファイルが残ることがある (前のファイルは壊れない)。
    覚えた読めるパスの `readable-files.json` も同じ形で書く。
- **読み込み:** 開くダイアログで選んだ・ウィンドウにドロップした・起動時の引数のファイルだけを読む (拡張子の検査は今までどおり)。
  同じディレクトリのほかのファイルも読めない。
  - 覚えた場所 (`path:`。着せ替えの組・覚えたものの一覧) から次の起動の後にも読み直せるよう、読めるパスはアプリのデータの
    ディレクトリの `readable-files.json` に覚える (新しいものから 1000 件。古いものは忘れ、忘れた場所は読み直せない)。
    このファイルはバックエンドだけが書く (webview の localStorage の場所の一覧を信じて読むことはしない)。
    起動時に読めない (権限が無い・通常のファイルでないなど。無いときを除く) ときは、空から始めてそのセッションでは書かない
    (読めなかった一覧を、次に選んだファイルだけの短い一覧で上書きしないため。T0011)。壊れた JSON は直せないので、次に選んだときに書き直す。
  - ドロップのパスは、Tauri がドロップのイベントを webview へ送った直後に、同じメインスレッドで `on_webview_event` が加える。
    webview がそのイベントを受けて `read_model_file` を呼ぶのはその後になる。
- 同梱の見本 (`sample:`) と `?model=` の URL (ブラウザ) は `fetch` で読むので、この範囲とは関わらない。

### コマンドのエラー (T0729)

コマンドは失敗すると、[ADR 0029](../../docs/adr/0029-shared-ui-and-i18n.md) の形 `{ code, args, message }` で reject する
(`src-tauri/src/error.rs` の `CommandError`)。`message` は英語の文 (ログ・訳の無い画面の向け)。画面は `host.ts` でそれを受けて
`CommandError` (`Error`。`message` はカタログの `errors.<code>` を `args` で引いた今の言語の文) にして投げ直す (`errors.ts`)。
カタログに無いコード (新しいバックエンドと古い画面など) は `message` を出す。数の引数 (バイト数) は今の言語の書式にする。
例外を受け取る側 (`@ugoe/player`・`@ugoe/face-tracking`・`drop.ts`) は、ほかの例外と同じく `Error` の文を出す。
エディタ (`apps/editor/src/lib/errors.ts`) と同じく訳すのは画面で、Rust に訳のカタログは置かない。

コードは公開の約束として扱い、名前を変えない (意味が変わるなら新しいコードにする)。

| コード | 引数 | いつ |
| --- | --- | --- |
| `file.not_model` | `path` | 読もうとしたファイルが `.ugoe`・`.json`・`.ugoemod` でない |
| `file.too_large` | `path`・`size`・`max` (バイト) | 読み込みの上限 (512 MiB) より大きい |
| `file.not_found` | `path` | 読もうとしたファイルが無い |
| `file.read_failed` | `path`・`reason` (OS の英語の文) | ほかの理由で読めない (通常のファイルでないときは `not a regular file`) |
| `file.write_failed` | `path`・`reason` | 書けない (一時ファイル・置き換えの失敗を含む) |
| `file.not_json` / `file.not_png` | `path` | JSON・PNG を書くパスの拡張子が `.json`・`.png` でない |
| `file.not_chosen` | `path` | 利用者が選んでいないファイルを読もうとした (「ファイルの読み書きの範囲」) |
| `file.not_chosen_for_save` | `path` | 保存のダイアログで選んでいないパスに書こうとした |
| `file.unsupported_save_type` | `extension` | 保存のダイアログで `json`・`png` 以外を求めた |
| `request.not_png_data` | なし | `write_png_file` の本文が PNG のバイト列でない |
| `request.missing_path` | `header` | `write_png_file` にパスのヘッダー (`x-ugoe-path`) が無い |
| `request.invalid_path` | `reason` | パスのヘッダーが読めない (符号化の誤り) |
| `dialog.failed` | `reason` | ダイアログで選ばれたものをパスにできない |
| `internal` | `reason` | 読み書きのスレッドの処理が終わらなかった (パニック。スタジオと同じコード) |

モデルの読み込みの失敗 (wasm の `LoadError`。読み込みの上限を超えたものを含む) は、コマンドのエラーではなく `@ugoe/player` が
`code` から訳す (`packages/player/src/loadError.ts`。T0434)。ビューアのカタログには置かない (スタジオと同じ訳を使うため)。

## 画像の保存 (T0420)

ツールバーの「画像を保存」で、今の表示を PNG で保存する (モデルを開いているときだけ押せる)。

- 画像は canvas の読み戻しではなく `WebPuppet.read_pixels()` で取る (`Player.capture`)。canvas と同じ大きさ (描画バッファの画素 =
  ビューポートの CSS ピクセル × devicePixelRatio)・ビュー (ズーム・パン)・背景・光源で canvas とは別のテクスチャに描いた、ストレートアルファの
  RGBA。WebGL2・WebGPU のどちらでも同じに取れる。
- 写るのはモデルと背景だけ。光源の印・ドロップの枠などビューポートに重ねる HTML は写らない。「透明」のときは市松模様ではなく
  透明 (アルファ 0) のまま書く。
- 押したときに読み戻してから保存のダイアログ (Tauri) を開く。既定の名前は `<モデルの名前>-<年月日>-<時分秒>.png` (拡張子と
  「 (サンプル)」を除き、ファイル名に使えない文字は `_`)。拡張子が無ければ `.png` を付ける。保存した場所は知らせ (`@ugoe/ui` の Toast) で出す。
- PNG への変換はフロントエンドの 2D canvas (`toBlob`) で行う。ブラウザ (`pnpm dev`) ではダウンロードで保存する。
  `ViewerHost` を差し替えるときは `savePngFile(name, bytes)` を置ける (無ければダウンロード)。
- **エディタと共有するもの (T0730):** 画素の検査と PNG への変換 (`checkScreenshot`・`encodePng`)・既定の名前の日時の部分
  (`timestampedPngName`)・パスのヘッダーの名前 (`PNG_PATH_HEADER`) は `@ugoe/player` の `src/screenshot.ts` にあり、エディタの
  ビューポートも同じものを使う。例外の文はそのパッケージのカタログ (名前空間 `player` の `screenshot.*`) で訳す。
  既定の名前の元 (ビューアはモデルのファイル名、エディタはプロジェクト名) と保存の流れは、それぞれのアプリが持つ。
- **バックエンドの `write_png_file` は共有しない (T0730):** 同じ処理 (ヘッダーのパスの `%XX` を戻す・拡張子 `.png` と PNG の署名を
  確かめる・一時ファイルを経て書く) がエディタの `src-tauri/src/screenshot.rs` にもある。共有しないのは、(1) アプリのバックエンドで
  共有する GPL のクレートがまだ無く、数十行のために新しいクレート (ワークスペースの登録・依存方向・CI のグループ・ライセンスの検査) を
  足すほどではない、(2) 失敗の形がアプリで違う (ビューアは画面で訳すコード付きの `CommandError` (T0729)、エディタは `EditorError`)、
  (3) 書けるパスの決め方が違う (ビューアは保存のダイアログで選んだパスを 1 回だけ (T0727)、エディタは絶対パス) ため。
  一時ファイルを経て書く `atomic_write.rs` (T0728・T0687) も同じ形で 2 つある。アプリのバックエンドで共有するクレートを作るときは、
  これらをまとめて移す。直すときは両方を同じに直す (テストも両方にある)。

## 着せ替え (T0706)

スロットを持つモデル (本体) に、着せ替えのモジュール (`.ugoemod`) を付け外しして試す。仕組みは
[docs/spec/modules.md](../../docs/spec/modules.md) と [ADR 0024](../../docs/adr/0024-module-authoring.md)、wasm の口は
[crates/ugoe-wasm/README.md#着せ替え-t0123](../../crates/ugoe-wasm/README.md#着せ替え-t0123)。

- 「着せ替え」のパネルの「モジュールを開く…」(複数選べる)、ツールバーの「開く」、ウィンドウへのドロップで `.ugoemod` を読み込む。
  モデルを開いていれば、読み込んだモジュールをそのまま付ける (同じスロットに付いているモジュールとは差し替わる)。
  同じ名前のモジュールを読み込み直すと新しい版に置き換える (付けていれば付け替える)。
- パネルはモデルのスロットを定義の順に並べ、スロットごとに付けているモジュール (無ければ「なし」) と、読み込んだそのスロットの
  モジュールを出す。モデルに無いスロットのモジュールは「モデルに無い」の印を付けて後ろに並べ、付けられない。
- **スロットの公開 (T0715):** モデルがアバタープロファイルを持つとき、`avatar.slots` に書いたスロットだけを利用者に見せる
  (`docs/spec/avatar-profile.md#アプリ向けの情報` の「書いていないスロットは作者用で、アプリは利用者に見せないことを推奨」に従う)。
  WebPuppet の `avatar_slots_json()` (本体のプロファイルの `slots`。プロファイルが無ければ `null`) を読み、`dressUp.ts` の `visibleSlots` で絞る。
  - 公開していないスロットは、パネルの一覧・「n / m スロット」の数・パネルを出すかの判定に入れない。
  - そのスロットのモジュールを読み込んだときだけ、「公開されていない」の印を付けて後ろに並べ、付けられない (付いていれば「外す」は使える)。
    モデルを開いた後にそのモジュールを開いても自動では付けず、「「X」のスロット「S」は、このモデルが公開していないため付けませんでした」と出す。
  - アバタープロファイルの無いモデル (公開の区別が無い) は、今までどおりすべてのスロットを見せる。プロファイルがあって `slots` が空なら、
    どのスロットも見せず「このモデルは着せ替えのスロットを公開していません」と出す。
  - 付け外しそのもの (wasm) は公開の有無を見ない (作者の確認用の経路を残す)。隠すのは画面だけ。

| 操作 | 内容 |
| --- | --- |
| 付ける | そのスロットに何も付いていないときに付ける |
| 差し替える | そのスロットに付いている別のモジュールと入れ替える |
| 外す | 外して本体だけの姿に戻す (`Hair` のように本体の要素を置き換えるスロットでは、本体の要素がまた見える) |
| × | 読み込んだモジュールを一覧から除く (付けていれば外す) |

- 付け外しでは、パラメータの値・トラッキング・目パチは続きから動き、物理 (揺れ) はリセットされる
  ([Puppet での着せ替え](../../docs/spec/modules.md#puppet-での着せ替え))。光源の設定も保つ。モジュールのメッシュがライティングを受けるなら、
  付けたときにライティングのパネルが出る。
- 付けられないとき (合成のエラー) は何も変えず、パネルに理由を出す。wasm の `DressUpError` の `code` ごとの文は `dressUp.ts` の
  `describeDressUpError`。たとえば:

  | `code` | 表示 |
  | --- | --- |
  | `unknown_slot` | 「Hat」を付けるスロット「Hat」が、このモデルにありません |
  | `missing_parameter` | 「Cape」に必要なパラメータ「Tail」が、このモデルにありません |
  | `parameter_not_exposed` | 「X」が使うパラメータ「P」を、スロット「S」が公開していません |
  | `avatar_level` | 「X」が求めるアバタープロファイルのレベルを、このモデルが満たしていません |
  | `no_model` | 先にモデル (.ugoe か .json) を開いてください |
  | `invalid_file` | 着せ替えのモジュール (.ugoemod) として読めません (原因) |

  合成・コンパイル・物理の検証など原因が細かいものは、wasm の文 (英語) を括弧で添える。知らないコードは wasm の文をそのまま出す。
- **同じモデル** ([モデルの見分け方](#モデルごとに覚える-t0711)) を開き直すと、読み込んだモジュールは残り、付けていたモジュールを
  wasm が付け直す。合わなくなったものは外してステータスバーに「このモデルに合わないため、着せ替えの「Dress」を外しました (原因)」と
  警告する。原因は wasm の `load_warnings_json()` の `module_detached` の要素の `error.code` から、付けられないときと同じ文にする
  (`describeLoadWarnings`。wasm の文の警告は読まない。T0124・T0125)。
- 読み込みのほかの警告 (T0125) も `load_warnings_json()` の `code` から今の言語の文にする: テクスチャを白で代用した (`texture_ktx2`・
  `texture_missing`)、法線マップを使わない (`normal_map_ktx2`)、物理の設定が合わないので外した (`physics_removed`。理由は英語のまま括弧で添える)、
  新しい版のファイルの知らない機能・値をフォールバックで読んだ (`unknown_optional_feature`・`unknown_value`。T0412)。
  モジュールの警告 (パネルの「警告 N」) も `modules_json()` の `warning_details` から同じ文にする。知らない `code` と構造の無い文は wasm の文 (英語) のまま出す。
- **別のモデル** を開くと、読み込んでいたモジュールを一覧から除く (`unload_module`)。前のモデルに付けていた組はモデルごとに
  覚えてある (下の「付けていた組を覚える」)。T0706 では一覧に残ったまま外れていたが、見本を切り替えたときに前の見本の衣装が
  「モデルに無い」で並ぶだけになるので、組ごとモデルに結び付けた (T0711)。モデルが開けなかったときは除いたモジュールを戻す。モデルを開く前に読み込んだモジュールは残す。
- モデルとモジュールを一緒に開く (見本の `openSample`・モデルとモジュールのドロップ) ときは `Player.load(model, modules)` で、
  モデルを開いた後でモジュールを読み込み、それから覚えていた組と比べる。
- GPU を失って作り直したときも、モジュールを読み直して付けていたものを付け直す (組は覚え直さない)。
- ヘッドレスのブラウザからは `window.__UGOE_VIEWER__` の `modules`・`slots`・`dressUpError`・`loadModule(file)`・`attachModule(name)`・
  `detachModule(name)`・`unloadModule(name)`・`setSlotModule(slot, name)`・`restoreOffer`・`restoreDressUp()`・`dismissRestore()`・
  `setAutoRestore(on)` を触れる。`ViewerHost` を差し替えるときは `pickModuleFiles` も置く (覚えたパスから読み直すなら `readPath` も)。

### 付けていた組を覚える (T0711)

モデルごとに、読み込んだモジュール (名前とファイルの鍵) と付けていたもの (合成の順) を `localStorage` の
`ugoe-viewer.dress-up` に覚える (`dressUpMemory.ts`。入れ物は下の `ModelMemory`)。

- 着せ替えを操作する (付ける・外す・差し替える・×・モジュールを開く) たびに、開いているモデルの組を覚え直す。
  覚えた組があれば (`dressUpRemembered`) パネルに「このモデル用に覚えた組があります」と「覚えた組を忘れる」(`forgetDressUp`) を出す。
  忘れても付けているモジュールはそのままで、尋ねている組は取り消す。次に着せ替えを操作すると、そのときの組をまた覚える
  (T0709)。覚えた組だけがあってスロットもモジュールも無いモデルでも、忘れられるようにパネルを出す。
  モジュールを 1 つも読み込んでいなければ、そのモデルの組を忘れる。覚えるのは最近開いた 20 体まで (`MAX_MODELS`)、1 体に
  64 個まで (`MAX_MODULES`)。ファイルの中身は覚えない (容量のため)。
- モデルを開いたとき (別のモデルか、同じモデルをモジュールと一緒に開き直したとき)、覚えていた組と違えば (付けているもの・その順・
  読み込んでいないモジュール。`needsRestore`) パネルで「戻す」か尋ねる (`restoreOffer`)。「前の組を自動で戻す」
  (`ugoe-viewer.dress-up.auto-restore`) なら尋ねずに戻す。尋ねている間は覚え直さず、「今回は戻さない」でも覚えている組は
  変えない (次に着せ替えを操作したときに覚え直す)。
- 戻すときは、覚えていたモジュールを **読み込んでいるもの (名前) → このセッションで読んだ中身 (指紋。32 個まで手元に置く) →
  覚えていた場所** の順に探し (`planRestore`)、付けているものを外してから覚えていた順に付ける。
- **ファイルが見つからないとき:** 場所から読めない (動かした・消した) か、場所が無い (ブラウザでのドロップ・`<input type="file">`) で
  このセッションで読んでいないモジュールは戻さず、パネルに理由を出す (`describeRestoreFailure`)。このとき覚えている組は変えないので、
  モジュールを開き直してから「戻す」ことも、そのまま付け直すこともできる。
- **ブラウザ版と Tauri 版の違い:** Tauri の「開く」「モジュールを開く…」で選んだファイルと、ウィンドウにドロップしたファイルは
  パス (`path:`) を覚え、起動し直してもバックエンドの `read_model_file` で読み直す (`ViewerHost.readPath`)。ドロップは OS からの
  ドロップを Tauri が受けてパスを渡す (`ViewerHost.listenFileDrop` = webview の `onDragDropEvent`。T0713)。このとき HTML のドロップは
  扱わない。ブラウザで選んだファイルとドロップしたファイル (File API はパスを教えない) は場所が無く、読み直せるのは開いている間だけ。
  見本 (`sample:`) と `?model=` の URL (`url:`) はどちらでも読み直せる。
- **開けないものが混ざったドロップ (T0717):** `drop.ts` の `openDroppedItems` が、拡張子 (`.ugoe`・`.json`・`.ugoemod`。
  `read_model_file` と同じ) で開けるものだけを読み、読めたものを開く。拡張子の無いもの (フォルダ) ・ほかの拡張子は読まずに飛ばし、
  読めなかったもの (拡張子の付いたフォルダ・消えたファイル。バックエンドのエラーは英語) も飛ばして、飛ばしたものの名前を
  ステータスバーに出す (`skippedMessage`。次に開くまで出す)。ブラウザの File API のドロップも同じ経路。
  Tauri ではドラッグ中の `enter` にもパスが来るので (`FileDropEvent` の `hover` の `paths`)、`describeDropPaths` で見立て、
  落とす枠の色 (すべて開ける = 青、一部を飛ばす = 黄、開けるものが無い = 赤) と上の文で示す。落とす枠はビューポートの canvas の
  上にも見えるように、`main` に重ねた要素で描く。
- **ブラウザのドラッグ中の見立て (T0718):** ブラウザはドラッグ中 (`dragover`) にファイルの名前を渡さない (落とすまで `File` を
  読めない) が、`DataTransfer.items` の数と `kind`・`type` (MIME) は分かる。`describeDropTypes` で種類から分かる範囲を見立てる
  (`classifyDragType`)。JSON (`application/json` など) は開ける、画像・文書など種類の付いたほかのものは開けない。`.ugoe`・`.ugoemod`・
  フォルダは種類が空 (OS に種類を登録していない) で区別できないので、「落としたときに名前で確かめます」と出し、枠は青
  (`unknown`)。開けない種類だけなら赤、混ざれば黄。一緒に来る文字列 (`kind` が `string`) は数えない。`items` を渡さないブラウザでは
  見立てず、枠だけを出す。

## モデルごとに覚える (T0711)

モデルごとの設定 (着せ替えの組と、光源の設定 (T0709)、待機モーションの選択 (T0722)) は、`modelMemory.ts` の部品で覚える。

| 覚えるもの | localStorage のキー | 上限 | 忘れる操作 |
| --- | --- | --- | --- |
| 着せ替えの組 (`dressUpMemory.ts`) | `ugoe-viewer.dress-up` | 20 体 | 着せ替えのパネルの「覚えた組を忘れる」、「覚えたもの」の一覧 |
| 光源の設定 (`lighting.ts` の `lightingMemory`) | `ugoe-viewer.lighting.models` | 50 体 | 「覚えたもの」の一覧 (T0714。T0709 ではパネルの「既定に戻す」だった) |
| 待機モーションの選択 (`animation.ts` の `idleMemory`) | `ugoe-viewer.idle-motion.models` | 50 体 | モーションのパネルの「覚えた選択を忘れる」、「覚えたもの」の一覧 (T0722) |

キーの接頭辞 `ugoe-viewer` は @ugoe/player の既定 (`PlayerHost.storagePrefix` を省いたとき。`packages/player/src/storage.ts`、T0930)。
ビューアは接頭辞を渡さない (前から覚えたものを読み続けるため)。

- **モデルの鍵 (`ModelIdentity`):** `{ name, fingerprint, location? }`。
  - `location` (`FileLocation`) は後から読み直せる場所: `path:<絶対パス>` (Tauri のダイアログ・ドロップ・起動時の引数)、
    `sample:<samples/ からのパス>` (同梱の見本)、`url:<絶対 URL>` (ブラウザの `?model=`)。ブラウザでドロップした・選んだファイルには無い。
    `host.ts` がファイルを読むときに `ModelFile.location` に入れ、`readFileAt(location)` で読み直す。
  - `fingerprint` はバイト列の指紋 (長さと 2 つの 32 ビットのハッシュ。暗号のハッシュではない。`fingerprint()`)。
  - `name` は表示だけに使い、比べない (`model.ugoe` のような同じ名前の別のモデルがよくあるため)。
- **同じモデル (`sameModel`):** 場所が同じか、指紋が同じ。場所が同じなら中身が変わっても (書き出し直し) 同じモデル。ブラウザでの
  ドロップで場所が分からなくても、中身が同じなら同じモデル。`ModelMemory.get` は場所が同じ項目を優先し、無ければ指紋が同じ項目を返す。
  `set` は同じモデルの古い項目を消して新しい鍵で覚える (ブラウザでドロップして覚えたモデルを、場所の分かる開き方で開くと、場所が付く)。
- **入れ物 (`ModelMemory<T>`):** `new ModelMemory(store, name, parse, { limit })`。`store` (`PlayerStorage`) の接頭辞と `name` の localStorage の 1 つのキーに
  `{"version": 1, "entries": [{"model": ModelIdentity, "value": T}]}` を最近使った順に置き、`limit` を超えたら古いものから消す。
  `get`・`set`・`delete`・`models`・`entries`・`clear` (後の 2 つは覚えたものの一覧、T0714)。値は `parse` (unknown → `T | null`) が確かめ、読めない項目・例外を出した項目・
  壊れた JSON・違う版・使えない localStorage では、何も覚えていないものとして動き、例外を出さない。
- 光源の設定 (T0709) も `Player.load` で作る鍵 (`modelIdentity(file)`) で `ModelMemory<LightingSettings>` に覚える
  (`parse` は `parseModelLighting`。オブジェクトでない値は読み飛ばし、中の壊れた値は `sanitizeLighting` で直す)。
- 確かめるための本体とモジュールはリポジトリに無い。`ugoe_model::fixtures` の `dress_up_base`・`long_hair_module`・`dress_module` を
  `ugoe_format` の `write_bundle` / `module_file::write_module` で書き出して作る (T0706 のメモ)。

### 覚えたものの一覧 (T0714)

ツールバーの「覚えたもの」(とライティングのパネルの「覚えたもの…」) で、モデルごとに覚えたもの (光源の設定・着せ替えの組・待機モーションの選択 (T0722)) を
一覧にする (`MemoryDialog.svelte`)。3 つの入れ物の項目を同じモデル (`sameModel`) ごとに 1 行にまとめ (`rememberedModels.ts` の
`mergeRemembered`)、開いているモデルを先頭に、あとは「並び」で選んだ順に並べる。既定は覚えた日時の新しい順で、名前の順も選べる (T0717)。選んだ並びは localStorage の
`ugoe-viewer.remembered-order` に覚え (`loadRememberedOrder`・`saveRememberedOrder` に `player.storage` を渡す)、次に一覧を開いたときも使う
(T0718。知らない値・読めないときは新しい順)。

- **覚えた日時 (T0717):** `ModelMemory` の項目に `at` (最後に覚えた日時、ミリ秒) を持たせた。`set` のたびに今の時刻にする
  (`ModelMemoryOptions.now` で差し替えられる)。保存の形の版 (1) は変えない: `at` の無い前の保存値もそのまま読め、その項目は
  「日時なし」として新しい順の最後 (その中は名前の順) に並ぶ。覚え直すと日時が付く。行の日時は覚えたものの中で最も新しいもの。

| 列 | 内容 |
| --- | --- |
| モデル | 名前と、覚えた場所 (見本・パス・URL。場所の無いものは「場所なし (中身で見分ける)」)。開いているモデルには「開いている」 |
| 光源 | 既定の組の名前か「カスタム」「切」と「忘れる」。覚えていなければ「—」 |
| 着せ替えの組 | 付けていたモジュールの名前と「忘れる」。覚えていなければ「—」 |
| 覚えた日時 | 光源と組のどちらかを最後に覚えた日時 (`2026/10/01 09:05`)。前の保存値だけなら「日時なし」 |
| (操作) | 光源と組の両方を覚えていれば「両方を忘れる」 |

- 「すべて忘れる…」は確かめのダイアログ (`AlertDialog`。既定のフォーカスは「キャンセル」) で確かめてから、入れ物のキーを消す (`ModelMemory.clear`)。
  最後に使った光源 (`ugoe-viewer.lighting`)・光源の印・前の組を自動で戻す、の設定は残す (モデルごとのものではないため)。
- 忘れても、いま表示している光源と付けているモジュールは変えない。開いているモデルの分を忘れたときは、パネルの「覚えた」の表示を消し、
  尋ねている組は取り消す。次に光源や着せ替えを変えると、そのときのものをまた覚える (`Player.forgetRemembered`・`forgetAllRemembered`)。
- ライティングを受けるのがモジュールだけのモデルでモジュールを外すとライティングのパネルは消えるが、覚えた光源は一覧から忘れられる
  (T0709 の後続の候補)。

## ライティング (T0705)

法線マップで陰影を付けるモデル (モデルの `lighting` と、メッシュの `lit` の両方が有効なメッシュがあるもの。
`WebPuppet.uses_lighting()`) を開くと、右のパネルに「ライティング」が出る。使わないモデルでは出さない。
光源の意味 (座標・向き・減衰・陰影の式) は [docs/spec/rendering.md#ライティング](../../docs/spec/rendering.md#ライティング)。

| 項目 | 内容 |
| --- | --- |
| 光源を当てる | 切ると光源を渡さない (`set_lighting_json(undefined)`)。ライティングを受けない場合と同じ色で描く |
| 標準・夕日・夜のランプ | 既定の光源の組。いまの設定がどれかと同じなら、その名前がパネルの見出しの右に出る (違えば「カスタム」) |
| 環境光 | 色と強さ。どの向きの面にも同じだけ当たる |
| 平行光源 | 光が来る向き。丸い面をドラッグ (中心が正面、縁が真横) するか、「方位」(0° = 右、90° = 上、180° = 左、−90° = 下) と「高さ」(0° = 面に沿う、90° = 正面) で選ぶ。色と強さ |
| 点光源 | 位置 (「横」「縦」はキャンバスの幅・高さに対する割合、四角い面のドラッグでも選べる)・「手前」(キャンバスからの距離)・「範囲」(光が届く距離。遠いほど弱まり、範囲の外には届かない)。手前と範囲はキャンバスの長い辺に対する割合。色と強さ |
| ビューポートに光源の印を出す | ビューポートに光源の印を重ね、印のドラッグで光源を動かす (下の「ビューポートの光源の印」)。既定は出す。`ugoe-viewer.lighting.markers` に覚える |

- 渡す色は「色 × 強さ」(1 を超えてよい)。位置と範囲を割合で持つので、キャンバスの大きさが違うモデルでも同じように当たる。
  JSON への変換は `lighting.ts` の `toLightingJson` (平行光源の `direction` は光が進む向きなので、来る向きの逆にする)。
- 設定は **モデルごとに覚える** (T0709。`ugoe-viewer.lighting.models`、最近開いた 50 体まで。鍵は [モデルごとに覚える](#モデルごとに覚える-t0711))。
  - 光源を変える (`setLighting`・`updateLighting`) と、開いているモデルの設定として覚え、最近使ったものにする。
  - モデルを開くと、覚えた設定があればそれにする (覚えた項目を最近使ったものにし、鍵も新しくする)。無ければ **最後に使った設定** を保つ
    (最後に使った設定は T0705 と同じ `ugoe-viewer.lighting` にも覚え、起動し直した後に覚えていないモデルで使う)。開くだけでは覚えない。
  - パネルに「このモデル用に覚えた設定」か「このモデル用の設定はまだありません」を出す (`lightingRemembered`)。
  - 「既定に戻す」(`resetLighting`) は光源を「標準」にし、**「標準」をそのモデルの設定として覚える** (T0714。最後に使った設定も
    「標準」になる)。次にそのモデルを開いたときも、最後に使った設定によらず「標準」で始まる。「標準」を覚えていれば押せない。
    T0709 では「そのモデルの設定を忘れて標準にする」だったが、次に開くと (覚えていないモデルの規則で) 最後に使った光源で始まり、
    「このモデルを標準に戻した」つもりの利用者を驚かせるので変えた。覚えた設定を忘れる操作は「覚えたもの」の一覧に分けた
    (忘れたモデルは、次に開くと最後に使った設定で始まる)。パネルの「覚えたもの…」からも一覧を開ける。
  - 既定を「最後に使った設定」にしたのは、T0705 までの「ビューアで 1 つ」の使い方 (好みの光源で見本を次々に見る) を保つため。
- 壊れた値・範囲の外の値は既定値で埋めるか範囲に収め、読めなければ (壊れた JSON・違う版) 覚えていないものとして扱う。
  最後に使った設定が読めなければ「標準」から始める。
- ヘッドレスのブラウザからは `window.__UGOE_VIEWER__` の `lighting`・`setLighting`・`updateLighting`・`commitLighting`・`usesLighting`・
  `lightingRemembered`・`resetLighting()`・`lightMarkers`・`setLightMarkers`・`view`・`canvasSize`・`viewportSize`・`dressUpRemembered`・`forgetDressUp()`・
  `rememberedModels()`・`forgetRemembered(model, { lighting, dressUp })`・`forgetAllRemembered()` を触れる。
  印の取っ手は `[data-light=directional]`・`[data-light=point]`、点光源の範囲の円の縁は `[data-light=range]` で引ける。
- スライダー・色・面をドラッグしている途中は描画だけを変え (`updateLighting(edit, { preview: true })`)、離したときに
  覚える (`commitLighting()`。最後に使った設定とモデルごとの設定を 1 回だけ書く)。キーボードで動かしたときはすぐ覚える。
  覚える前に別のモデルを開いたときは、開く前に前のモデルの設定として覚える。
- 確かめるには「サンプル」→「アバター (ライティング)」(`assets/samples/avatar-lit.ugoe`。見本のアバターに法線マップを付けた版、T0707) を開く。

### ビューポートの光源の印 (T0708)

光源を当てていて、パネルの「ビューポートに光源の印を出す」が入っているとき、ビューポートに印を重ねる (`LightMarkers.svelte`)。
形はエディタの光源の取っ手 ([docs/manual/lighting.md](../../docs/manual/lighting.md#光源-preview-lights)) に合わせた。

| 印 | 内容 |
| --- | --- |
| 平行光源 | 表示しているキャンバスの中央に黄色い破線の円と、光源の色の丸い取っ手。**取っ手のある向きから光が来る** (円の中心 = 正面から、縁 = 真横から)。取っ手をドラッグすると「方位」と「高さ」が変わる (円の外へ引くと縁、つまり真横から)。円の半径はキャンバスの短い辺の 35% を 60〜220 CSS ピクセルに収めたもの |
| 点光源 | キャンバスの上の光源の位置に、放射線の付いた取っ手。ドラッグで「横」「縦」が変わる。取っ手の上で **Shift + ホイール** を回すと「手前」が変わる (奥へ回すと手前へ離れる)。光が面の上で届く範囲 (`√(範囲² − 手前²)`) を破線の円で描き、**円の縁をドラッグ** すると「範囲」が変わる (手前は保つ。T0714) |

- **パンとの切り分け:** 取っ手と範囲の円の縁 (幅 10 CSS ピクセルの見えない線) だけがポインタを受け (`pointer-events`)、そこで
  押したときはパンを始めない。円の内側・外側を含むほかの場所のドラッグは今までどおりパン。ホイールは取っ手の上でもズームで、
  Shift を押しているときだけ点光源の手前を変える (Shift + ホイールはビューポートでは使っていなかった)。取っ手の上のダブルクリックではフィットしない。
  ホイールで変えた手前は、回し終えて 0.4 秒たったときに覚える (回している間は覚えない)。
- **キーボード (T0714):** 取っ手は Tab でフォーカスできる (`role="slider"`、読み上げ用に `aria-valuetext` で方位・高さ、横・縦・手前・範囲を出す)。
  キーボードで来たフォーカスでは取っ手の周りに輪を描く (`:focus-visible`)。キーボードで動かすとすぐ覚える (パネルと同じ)。

  | 取っ手 | キー |
  | --- | --- |
  | 平行光源 | 矢印キーで取っ手を上下左右に動かす (パネルの丸い面と同じ量。Shift で大きく)。円の外へは出ない |
  | 点光源 | 矢印キーで横・縦 (1%、Shift で 10%)、PageUp / PageDown で手前 (2%、Shift で 10%)、`+` / `-` で範囲 (5%) |

  範囲の円の縁はフォーカスを受けない (キーボードでは点光源の取っ手の `+` / `-`)。計算は `lighting.ts` の `stepMarker`。
- **画面の外 (T0714):** 大きく拡大してキャンバスの中央が画面の外に出たときは、平行光源の円を、円ごと画面の内側 (縁から 14 CSS ピクセル)
  に寄せる (`lightDial` の `bounds`)。円はキャンバスの中央を示さなくなるが、向きの意味 (取っ手のある向きから光が来る) は変わらない。
  点光源が画面の外にあるときは、取っ手を画面の縁に寄せ、破線の縁で描く (`pointHandleOnScreen`)。寄せた取っ手を掴んで動かすと、
  光源は掴んだ位置へ移る。範囲の円は光源の本当の位置に描く。ビューポートの大きさは `Player.viewportSize`。
- **寄せた平行光源の円 (T0717):** 寄せたとき (`LightDial.shifted`) は、円を点線にし、中心の点を白抜きにして、中心から本当の
  キャンバスの中央 (`LightDial.canvasCenter`) の向きに破線の矢印 (`[data-light=to-center]`) を出す。中心がキャンバスの中央を
  示していないことが見て分かるように。
- **範囲の取っ手 (T0717):** 範囲の円の縁が画面を通らない (円が画面をすっぽり覆う。範囲が大きい既定の 1.4〜1.5 ではよくある。
  または円がまるごと画面の外) ときは、画面の端に小さな範囲の取っ手 (両向きの矢印、`[data-light=range-grip]`) を出す
  (`lighting.ts` の `rangeGrip`)。位置は、範囲を変えたときに縁が最初に画面に入ってくるところ (T0718): 円が画面を覆うときは
  光源からいちばん遠い画面の角、円が画面の外にあるときは光源にいちばん近い画面の点 (どちらも画面の縁から 14 CSS ピクセルの内側)。
  取っ手は光源から伸ばした線の上にあり、その向き (光源から離れる向き) へ動かした分だけ円の半径を増やし、逆向きで減らす
  (`rangeRadiusFromGrip`。向きに直交する動きは効かない)。T0717 は光源から画面の中央へ向かう向きで、光源が中央に近いと
  右向きに固定され、取っ手の位置が光源から伸ばした線に乗らなかった。ドラッグの間は縁が画面に入っても取っ手を消さない。画面の縁に寄せた点光源の取っ手と重なる
  (24 CSS ピクセル以内) ときは出さない。キーボードは今までどおり点光源の取っ手の `+` / `-`。
- **取っ手にフォーカスがあるときの `F` (T0717):** アプリのショートカット (全体を表示) のままにした。取っ手は文字のキーを使わず、
  フィットしても取っ手のフォーカスは残り、印は新しい位置に描き直されるので、横取りする理由が無い (拡大して端に寄せた印を
  もとに戻すのにも使える)。
- **パネルとの同期:** 取っ手のドラッグはパネルと同じ `updateLighting` で値を変える (途中は `preview`、離したときに `commitLighting`)
  ので、パネルの面・スライダーもすぐ動き、モデルごとの記憶 (T0709) にも同じように入る。
- **位置の合わせ方:** キャンバス座標から画面座標への変換は `WebPuppet.view_transform()` (`[scale, offset_x, offset_y]`、描画先の画素) を
  `devicePixelRatio` で割ったもの (`Player.view`)。ズーム・パン・フィット・大きさの変更・モデルを開いたときに読み直す。
  幾何の計算は `lighting.ts` の `lightDial`・`directionalHandle`・`anglesFromScreen`・`pointHandle`・`pointFromScreen`・`pointRangeOnCanvas`。
- ビューポートの描画に鏡像の表示は無い (フェイストラッキングの「鏡像で表示」とランドマークは、右のパネルのカメラの映像にだけ掛かる)
  ので、印は反転を考えなくてよい。ビューポートに鏡像の表示を足すときは、印の変換にも同じ反転を入れる。
- 光源を切っているとき・ライティングを使わないモデルでは出さない。平行光源・点光源のうち切っているものの印も出さない。

## モーションと表情 (T0720)

開いたバンドル (`.ugoe`) にモーションか表情があると、右のパネルに「モーションと表情」が出る (JSON のモデルにはモーションと表情が無いので出ない)。
規則 (優先度・フェード・表情の set と add・フレーム内の適用順) は [docs/spec/motion.md](../../docs/spec/motion.md)、wasm の口は
[crates/ugoe-wasm/README.md#物理モーション表情エフェクト-t0309](../../crates/ugoe-wasm/README.md#物理モーション表情エフェクト-t0309)。
見本の「アバター」「アバター (ライティング)」「着せ替え」には、モーション `idle` (ループ)・`nod`・`shake`・`tilt` と表情が入っている (T0719)。

| 操作 | 内容 |
| --- | --- |
| ▶ / ■ | モーションを「優先度」で始める (`start_motion`)。再生中のものは ■ になり、押すと止める。名前の下に長さ・ループ・フェード (入り / 抜け) を出す |
| 優先度 | 待機 (`idle`)・通常 (`normal`、既定)・強制 (`force`)。再生中のモーションより低い優先度では始まらず、パネルに理由を出す (そのモーションが終わると消す) |
| 繰り返す | ループしないモーションが終わったら、同じ優先度で初めから始め直す (ビューアだけの機能。モデルのループの設定は変えない) |
| 止める | 再生中のモーションをすべてフェードアウトする (`stop_motions`)。待機モーションの自動再生が入っていれば待機に戻る |
| トラッキング中もモーションを優先 | 下の「フェイストラッキングと同時に使う」(T0721。既定で入) |
| 待機モーションを自動で流す | 選んだ待機モーションを待機の優先度で流す (T0721。既定で入、下の「待機モーション」) |
| これだけ | ほかの表情を止めて、その表情だけにする (`set_expression`) |
| 重ねる / 外す | 使っている表情の上に重ねる (`add_expression`) / その表情をフェードアウトする (`stop_expression`) |
| すべて止める | 使っている表情をすべてフェードアウトする (`stop_expressions`) |

- **止めた後の姿:** モーションを止めても (終わっても)、最後に適用した値はパラメータに残る (仕様の「基準値」。`docs/spec/motion.md#フレーム内の適用順`)。
  もとの姿に戻すには、パラメータの「すべて戻す」を押す。表情は止めるとフェードして抜ける (値は残らない)。
- **スライダー:** 再生中のモーションが動かすパラメータのスライダーは「(自動)」になり、手では動かせない (動かしてもモーションに上書きされるため)。
  表情の対象のパラメータは動かせ、その値の上に表情が重なる。
- 使っている表情の一覧は、操作のたびに WebPuppet の `active_expressions()` から読む (T0721。T0720 では操作から求めていた)。同じ表情を 2 回重ねない。
- 直近に発火したモーションのイベント (見本の `nod` の `nod-end` など) を新しい順に 5 つまで出す。
- モデルを開き直すと (同じモデルでも) 再生の状態は初めからになる。着せ替えの付け外しでは続きから動く。
- ヘッドレスのブラウザからは `window.__UGOE_VIEWER__` の `motions`・`expressions`・`currentMotion`・`activeExpressions`・`motionNotice`・`motionEvents`・
  `playMotion(name, priority?)`・`stopMotions()`・`setMotionPriority`・`setRepeatMotion`・`setExpression`・`addExpression`・`stopExpression`・`stopExpressions()`、
  T0721 の `autoIdle`・`idleMotion`・`motionOverTracking`・`setAutoIdle`・`setIdleMotion`・`setMotionOverTracking`、
  T0722 の `idleRemembered`・`forgetIdleMotion()`・`trackingKept`・`setTrackingKept(ids)` を触れる。

### 待機モーション (T0721)

- 「待機モーションを自動で流す」が入っていると、モデルを開いたときと、ほかのモーションが終わったとき (「止める」・■ で止めたときも) に、
  選んだ待機モーションを **待機の優先度** で始める。待機の優先度なので、通常・強制のモーションを再生するとそちらに置き換わり、
  終わると待機に戻る (「繰り返す」が入っていれば、終わったモーションの繰り返しが先)。待機モーションそのものを止めると、待機は流さない
  (ほかのモーションを再生して終わると、また流れる)。
- 待機モーションは、モデルを開くたびに、そのモデルに覚えた選択 (下) か、無ければ既定 (`animation.ts` の `defaultIdleMotion`:
  名前が `idle` のもの (大文字小文字は区別しない)、無ければ `idle` で始まるもの (ループするものを先に)) を選ぶ。選択欄で替えられる
  (「(なし)」で流さない)。一覧では「待機」の印が付く。
- オン・オフ (`ugoe-viewer.auto-idle`) は localStorage に覚える (モデルによらない)。
- **待機モーションの選択はモデルごとに覚える (T0722)。** 選択欄で選ぶと (「(なし)」も)、開いているモデルの分として
  `ugoe-viewer.idle-motion.models` に覚える (`animation.ts` の `idleMemory`。入れ物は `modelMemory.ts` の `ModelMemory`、値は
  `{ motion: string | null }`)。次にそのモデルを開くと覚えた選択で始める (`initialIdleMotion`)。覚えたモーションがモデルから消えていたら
  既定を使う (覚えたものは消さない。書き出し直して名前が戻れば、また使う)。モデルを開いただけでは覚えない (既定は覚えない)。
  覚えていると選択欄の横に「覚えた選択を忘れる」が出る。「覚えたもの」の一覧にも「待機モーション」の列で出し、そこでも忘れられる。
  T0721 では名前の約束で足りるとして覚えなかったが、`idle` の名前でない待機モーションを持つモデルを開くたびに選び直すのを省くため足した。

### フェイストラッキングと同時に使う

ランタイムの適用順 (モーション → トラッキング → 表情 → 目パチ・呼吸 → 物理) を基本にする。決めた扱い
(「トラッキング中もモーションを優先」を切ったとき。入れたときは下の「モーションを優先する」):

| もの | トラッキング中の見え方 |
| --- | --- |
| モーション | **トラッキングが書き込むパラメータ (「変換後のパラメータ」に出るもの。頭・体の向き・目・眉・口・肩など) は、トラッキングの値で置き換わる。** そのほかのパラメータ (髪・腕など) はモーションで動く。見本のモーションは頭と体の向き・目の玉を動かすので、トラッキング中はほとんど見えない |
| 表情 | トラッキングの値の上に重なる (`add`・`multiply` は重ね、`overwrite` は表情の重みの分だけ置き換える)。笑顔のボタンを押すと、トラッキングした顔が笑顔に寄る、という使い方ができる |
| 目パチ | 今までどおり「目パチを止める」(既定で入) でトラッキング中は止める |

- パネルには、トラッキング中にその旨の説明と、モーションごとの印 (すべての対象がトラッキングと重なれば「トラッキングで隠れる」、一部なら「一部トラッキング」。
  上書きされるパラメータは印の説明に出る) と、表情ごとの「トラッキングに重なる」の印を出す。
- トラッキング中も、モーションの時間は進み、イベントも発火する。トラッキングを止めると (トラッキングのパラメータは既定値に戻る)、再生中のモーションがまた見える。

#### モーションを優先する (T0721)

「トラッキング中もモーションを優先」(既定で入。`ugoe-viewer.motion-over-tracking` に覚える) を入れると、**待機以外の優先度で再生中の
モーション** が動かすパラメータでは、トラッキングの値の上にモーションの値が乗る (`WebPuppet.set_motion_over_tracking(true)`)。
見本の `nod` を再生すると、トラッキング中でも頭がうなずき、終わるとフェードアウトしてトラッキングの頭の向きに戻る。
モーションが動かさないパラメータ (目・口など) はトラッキングのまま。

- しくみ: `Puppet::update` が、トラッキングの値をモーションより先に書き込む (`docs/spec/motion.md#トラッキング中のモーションの優先`)。
  モーションはフェードの重みでその上に乗るので、始まりと終わりがなめらかにつながる。T0721 では wasm がホストの書き込みとして
  入れていたが、パラメータごとに分けるために `Puppet` に移した (T0722)。
- トラッキングの値は今までどおり基準値に含まれる。トラッキングを止めると、最後の値が基準値のまま残る。
- **優先する部分 (T0722):** 優先を入れると、その下に「優先する部分」(頭の向き・体・肩・目・眉・口・その他。`animation.ts` の
  `trackingGroup` が ID の先頭で分ける) のチェックと、「パラメータごとに選ぶ」(トラッキングが書き込むパラメータの一覧) が出る。
  外した部分・パラメータは、モーションが動かしてもトラッキングのまま (例: 目を外すと、`nod` でうなずきながら目はトラッキングどおり
  瞬きする)。部分のチェックは、一部だけ外していると中間の表示になり、押すとまとめて優先に戻す。選択は外したパラメータの ID として
  `ugoe-viewer.motion-over-tracking.kept` に覚え (モデルによらない。標準のパラメータの ID は共通のため)、WebPuppet の
  `set_tracking_kept` に渡す。外したパラメータとモーションが重なると、モーションに「一部トラッキング」の印が出る。
- 表情・目パチ・呼吸は、今までどおりトラッキング (とモーション) の値の上に重なる。
- 待機の優先度のモーション (自動で流す待機モーションなど) には効かない。ループする待機モーションが頭の向きをずっと上書きすると、
  トラッキングが見えなくなるため。待機モーションを優先させたいときは、優先度を「通常」にして ▶ で再生する。
- 印: 優先する間は、待機以外のモーションに「トラッキングで隠れる」「一部トラッキング」を出さない (自動で流す待機モーションには出す)。
- 切った場合は T0720 のまま (上の表)。トラッキングの対象をパラメータごとに絞る設定 (`avatar.tracking` の上書き) とは別のもの。

## フェイストラッキング (T0703)

右のパネルの「フェイストラッキング」で、カメラの映像から表情と頭の向き、体の向きと肩の上下 (T0645、「体 (肩) も」。
[packages/face-tracking](../../packages/face-tracking/README.md#体のトラッキング-t0645)) を取ってモデルを動かす。

- 顔の検出は [MediaPipe Face Landmarker](https://ai.google.dev/edge/mediapipe/solutions/vision/face_landmarker/web_js)
  (`@mediapipe/tasks-vision`) で行い、ARKit 互換のブレンドシェイプ (MediaPipe は `_neutral` と 51 個。`tongueOut` は無い) と
  顔の変換行列を得る。
- パラメータへの変換は Rust の対応付け (`ugoe_motion::tracking`、[アバタープロファイル](../../docs/spec/avatar-profile.md#フェイストラッキングとの対応))
  を `WebPuppet.set_tracking` で呼ぶ。TS では式を持たない。モデルの `avatar.tracking` の上書きも効く。
- アプリ側で行うこと (`@ugoe/face-tracking`。仕様の「実装の範囲」でホストの責務とされたもの):
  - 頭の向き: 変換行列 (列優先の 4x4) の回転を Y・X・Z の順に分解し、ヨー・ピッチ・ロール (度) にする (`frame.ts`)。
    反転していない映像では、写った人が自分の左を向くと正のヨー、上を向くと正のピッチ、画面で時計回りに傾くと正のロール。
  - キャリブレーション: 「キャリブレーション」で直近 15 フレームの平均を中立にし、頭の向きは差し引き、
    ブレンドシェイプは中立の値を 0、1 を 1 にする (`calibration.ts`)。
  - 平滑化: 値ごとに One Euro フィルタ (`filter.ts`)。「平滑化」の強さ (0〜1) で止まっているときの遮断周波数を 30 Hz〜0.3 Hz にする。0 なら素通し。
  - 顔を見失ったときは、最後の値を保つ。止めると、トラッキングが書き込んでいたパラメータを既定値に戻す。
  - 左右: 既定では、写った人自身の左右がキャラクター自身の左右になる (仕様の約束。カメラの映像は反転していないので、
    MediaPipe の値をそのまま使う)。「左右を入れ替える」を選ぶと、左右の名前とヨー・ロールの符号を入れ替え、モデルが鏡に映った
    自分のように動く。入力の映像がすでに反転しているとき (反転済みの仮想カメラなど) も、これで直す。
    「鏡像で表示」は映像の表示だけを反転し、値は変えない。
- **目パチ:** トラッキング中は、既定でモデルの自動の目パチを止める (「目パチを止める」、`WebPuppet.set_enabled("eye_blink", false)`)。
  目パチは基準値に重なるので、止めないとトラッキングの目の開閉と目パチが混ざる。止めると元に戻す。
- カメラの無い環境 (CI・ヘッドレス) では「ファイル…」で動画・画像、または記録した値の JSON を入れる。
  - 動画は繰り返し再生し、画像は同じ画像を検出し続ける。
  - 「記録」で検出した値 (キャリブレーション・平滑化の前) を記録し、「保存」で JSON にする。形式は `packages/face-tracking/src/recording.ts`
    (`{"format": "ugoe-face-tracking", "version": 2, "frames": [{"t", "head": {"yaw", "pitch", "roll"}, "blendShapes": {...}, "body": {...}}]}`。
    `body` は体のトラッキングの値で、版 2 で足した)。
    JSON を開くと MediaPipe を使わずに繰り返し再生する (キャリブレーションと平滑化は通す)。
  - ヘッドレスのブラウザからは `window.__UGOE_TRACKER__` (`FaceTracker`) を触れる。`output` (渡した値)・`landmarks`・`status` など。
  - Chrome の偽のカメラ (`--use-fake-device-for-media-stream --use-file-for-fake-video-capture=<file.y4m>`) でも、カメラの経路を確かめられる。

### MediaPipe の同梱

実行時にネットワークに頼らないよう、wasm とモデルをアプリに同梱する (`public/mediapipe/`)。

- wasm は npm パッケージの `wasm/` から写す (SIMD 版と SIMD の無い版。`FilesetResolver` が選ぶ)。
- モデル `face_landmarker.task`・`pose_landmarker_lite.task` は **リポジトリにコミットしない**。`ugoe-mediapipe-assets` (`packages/face-tracking/scripts/mediapipe-assets.mjs`) が版を固定した URL
  (`.../face_landmarker/float16/1/face_landmarker.task`・`.../pose_landmarker_lite/float16/1/pose_landmarker_lite.task`) から取得し、SHA-256 を検証する (違えば失敗する)。
  検証済みのファイルがあれば取得しない。モデルを更新するときは、スクリプトの URL とハッシュを一緒に書き換える。
- `pnpm build` (= `tauri build` の前) は取得できなければ失敗する。`pnpm dev` は取得できなくても警告だけで起動する
  (トラッキングを始めると「モデルが同梱されていません」と表示する)。手で用意するときは `pnpm mediapipe`。
- ライセンスは両方とも Apache-2.0。表記は `public/third-party-notices.txt` にあり、アプリのステータスバーの「ライセンス」で表示する
  (MediaPipe の節は `packages/face-tracking/third-party-notices.txt` から生成する。[docs/release.md](../../docs/release.md#同梱物のライセンスの表記))。

### カメラの許可 (OS ごと)

| OS | webview | 設定 | 確かめたか |
| --- | --- | --- | --- |
| Linux | WebKitGTK | バックエンドの `on_permission_request` でカメラの要求だけを許す (`src-tauri/src/lib.rs`)。ハンドラが無いと WebKitGTK は要求を拒む | WSLg (WebKitGTK 2.52、カメラ無し): 画像での検出とモデルへの反映、`getUserMedia` が呼べること (カメラが無いので `Invalid constraint` で失敗) まで。実際のカメラでは未確認 |
| Windows | WebView2 | 同じハンドラで許す (WebView2 は既定では利用者に尋ねる) | 未確認 |
| macOS | WKWebView | `src-tauri/Info.plist` の `NSCameraUsageDescription` (OS が初回に尋ねる) と、Hardened Runtime 向けの `src-tauri/Entitlements.plist` (`com.apple.security.device.camera`) | 未確認 |

MediaPipe には WebGL の canvas を明示して渡す (`tracker.svelte.ts`)。渡さないと OffscreenCanvas を使い、
WebKitGTK では検出のときに `GLctx.activeTexture` のエラーで失敗した。

確かめていない OS では、利用者が次を確認する: 「開始」でカメラの許可を求められる (または自動で許される) こと、カメラの一覧に名前が出ること、
映像とランドマークが表示され、目の開閉・口の開き・頭の向きがモデルに反映されること。

## 外からパラメータを動かす (ドライバ)

フレームごとにパラメータを設定するものは `ParameterDriver` (`src/lib/drivers.ts`) として登録する。
フェイストラッキング (T0703) や、自動の目パチ・呼吸の確認はこの形で足す。

```ts
const remove = player.drivers.add({
  name: "sway",
  frame(dt) {
    t += dt;
    return [["AngleX", 30 * Math.sin(t)]];
  },
});
```

- 毎フレーム、登録順にすべてのドライバの `frame(dt)` を呼び、返った値を設定してから `update(dt)`・描画する。
  同じパラメータを複数のドライバが返したら、後に登録したものが勝つ。
- フェイストラッキングはドライバではなく `Player.setTracking` で渡す (Rust の `Puppet::set_tracking`)。
  `update(dt)` の中で書き込むので、トラッキングが書き込むパラメータではドライバの値より優先される。
- ドライバが値を返しているパラメータのスライダーは「(自動)」と表示され、手では動かせない。
- 開発中やヘッドレスのブラウザでは、表示中の `Player` を `window.__UGOE_VIEWER__` から触れる。

右のパネル (`App.svelte` の `.sidebar`) は縦にパネルを積む作りで、上からトラッキング → モデルの機能ごとのパネル (着せ替え・ライティング・モーションと表情。
そのモデルが使うときだけ出す、畳める `@ugoe/ui` の `Section`) → パラメータ (残りの高さ) の順に並ぶ。縦が足りないときは、モデルの機能ごとのパネルが
見出しの高さまで縮んで中を送る。幅が 720px より狭いときは、パネルをビューポートの下に置く。

## ヘッドレスのブラウザで確かめる

Tauri の外 (ふつうのブラウザ) で開くと、ファイルの選択は `<input type="file">` になる。さらに:

- `?model=<URL>` を付けると、起動時にその URL のモデルを開く (例: `http://localhost:1430/?model=/c.ugoe`)。
- `window.__UGOE_VIEWER_HOST__` に `ViewerHost` (`src/lib/host.ts`) を置くと、「開く」などでそれを使う。
- ドロップは `DragEvent` と `DataTransfer` に `File` を入れて `window` に送れば確かめられる。差し替えた `ViewerHost` に
  `listenFileDrop` (と `readPath`) を置くと、Tauri のようにパスで受け取る経路を確かめられる (HTML のドロップは扱わなくなる)。
- WebGPU の無いヘッドレスの Chrome でも、SwiftShader の WebGL2 で描ける
  (`--use-gl=angle --use-angle=swiftshader --enable-unsafe-swiftshader`)。

## コマンド

```sh
just viewer                      # 開発モードで起動 (vite の 1430 番ポート + tauri dev)
just viewer-build                # リリースビルド (`just app::build TARGET=viewer` の別名)。例: just viewer-build --bundles deb
pnpm --filter @ugoe/viewer wasm  # ugoe-wasm をビルドして src/lib/wasm/generated/ に JS バインディングを出す
pnpm --filter @ugoe/viewer mediapipe  # MediaPipe の wasm とモデルを public/mediapipe/ に用意する (dev・build でも実行される)
just web-check                   # Biome + svelte-check + Vitest (エディタとビューアの両方)
```

`just viewer` と `just viewer-build` は、先に `pnpm wasm` を実行する (`tauri.conf.json` の `beforeDevCommand` /
`beforeBuildCommand`)。必要な道具 (Rust・Node.js・pnpm・just・wasm-bindgen-cli・各 OS の前提パッケージ) はエディタと同じなので、
[apps/editor/README.md#各-os-でのビルド](../editor/README.md#各-os-でのビルド) を参照。
Linux (WSL2) から Windows 向けのクロスビルドも同じ (`just app::build TARGET=viewer PLATFORM=windows`)。
GitHub Actions の Editor build のワークフローは、ビューアのインストーラ (`ugoe-viewer-<OS>`) も作る。

アイコンは `src-tauri/icons/icon.png` から `pnpm tauri icon src-tauri/icons/icon.png -o src-tauri/icons` で作り直せる
(Android・iOS 用に出力されるディレクトリは使わないので消す)。
