# apps/viewer

ugoe のビューア (Tauri v2 + Svelte 5)。`.ugoe` バンドルか JSON のモデル (`model.json`) を開いて表示し、
パラメータを動かして確かめるだけの軽いアプリ。着せ替えのモジュール (`.ugoemod`) を付け外しして試せる (T0706)。ランタイムをアプリに組み込むときの見本と、
フェイストラッキング (T0703) の確認の場を兼ねる。

**ライセンスは GPL-3.0-only** (エディタと同じ。[ADR 0019](../../docs/adr/0019-viewer-license.md))。
エディタのコードは写してよいが、アプリ同士で import はしない。

```
src/                  フロントエンド (Svelte 5 + TS。npm パッケージ @ugoe/viewer)
  App.svelte            ツールバー・ビューポート・右のパネル・ステータスバー、ドロップとショートカット
  lib/player.svelte.ts  WebPuppet (ugoe-wasm) を持ち、フレームのループを回す。UI の状態もここ
  lib/drivers.ts        パラメータを毎フレーム外から動かす仕組み (ParameterDriver)
  lib/host.ts           ファイルの選択と読み込み (Tauri / ブラウザ / 差し替え)
  lib/Viewport.svelte   描画領域 (ズーム・パン・フィット)。LightMarkers.svelte は重ねる光源の印 (ドラッグ・キーボードで光源を動かす、T0708・T0714)
  lib/ParameterPanel.svelte  パラメータのスライダー (グループごと)
  lib/LightingPanel.svelte   光源のパネル (ライティングを使うモデルのときだけ出す)。LightPad.svelte は向き・位置の面
  lib/lighting.ts       光源の設定・既定の光源の組・set_lighting_json の JSON への変換・localStorage への保存 (最後に使った設定と、モデルごとの設定 (T0709))
  lib/DressUpPanel.svelte  着せ替えのパネル (スロットのあるモデルか、モジュールを読み込んだときだけ出す)
  lib/dressUp.ts        着せ替えの一覧 (modules_json・slots_json) の読み取り、スロットごとのまとめ、エラーと警告の文
  lib/loadWarnings.ts   読み込みの警告 (load_warnings_json・モジュールの warning_details) の読み取りと日本語の文 (T0125)
  lib/dressUpMemory.ts  付けていたモジュールの組をモデルごとに覚える形と、戻すときにファイルをどこから得るか (T0711)
  lib/modelMemory.ts    モデルを見分ける鍵と、モデルごとの値を上限付きで localStorage に覚える入れ物 (T0711。光源の T0709 と共有)
  lib/rememberedModels.ts  モデルごとに覚えたもの (光源・着せ替えの組) を 1 モデル 1 行にまとめる (T0714)。MemoryDialog.svelte はその一覧の画面
  lib/modelFile.ts      開いたファイルの形式の判別 (バンドル・JSON・モジュール)
  lib/drop.ts           ドロップしたファイルの開き方 (モデルとモジュールを一緒に落としたとき。T0713)
  lib/samples.ts        同梱の見本の一覧と開き方 (ツールバーの「サンプル」)
  lib/wasm/             ugoe-wasm の読み込み。generated/ は `pnpm wasm` の出力 (gitignore 済み)
public/               そのまま配る静的ファイル。third-party-notices.txt (同梱物のライセンス) だけコミットする
  mediapipe/            MediaPipe の wasm とモデル。`pnpm mediapipe` が用意する (gitignore 済み)
src-tauri/            バックエンド (Rust クレート `ugoe-viewer`)
```

フェイストラッキング (T0703) の部品 (MediaPipe の結果の変換・キャリブレーション・平滑化・記録・`FaceTracker`・
`TrackingPanel`) と、MediaPipe の wasm を写しモデルを取得して SHA-256 を検証するスクリプト (`ugoe-mediapipe-assets`) は、
エディタのプレビューと共有するパッケージ `@ugoe/face-tracking` ([packages/face-tracking/](../../packages/face-tracking/)) にある (T0636)。
ビューアは `Player` を `TrackingTarget` として渡し、文言は日本語 (`ja`)、記録の保存は `host.ts` の `saveJsonFile`。

## 仕組み

- 描画もパラメータの操作も、webview の中の `ugoe-wasm` (`WebPuppet`) で行う (エディタのビューポートと同じ、
  [ADR 0003](../../docs/adr/0003-editor-viewport-in-webview.md))。WebGPU が無ければ WebGL2 で描く。
- バックエンドは webview だけではできないことに限る: モデルとモジュールのファイルをバイト列で読む (`read_model_file`。
  拡張子が `.ugoe`・`.json`・`.ugoemod` のものだけ) こと、
  トラッキングの記録を JSON で書く (`write_json_file`。保存のダイアログで選んだ `.json` だけ) こと、
  起動時の引数のパスを返す (`initial_file`) こと、webview からのカメラの許可の要求に応じること。依存してよいのはランタイム層のクレートだけ (`scripts/check-layers.mjs`)。
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
| サンプル | 同梱の見本 (`assets/samples/`、MIT) をメニューから選んで開く。「アバター」はアバタープロファイルのレベル 3 の見本で、フェイストラッキングをすぐ試せる。「アバター (ライティング)」は同じ見本に法線マップを付けた版で、ライティングのパネルが出る。「着せ替え」はスロットを持つ本体と衣装のモジュール 4 つを読み込み、`LongHair`・`CatEars`・`Sailor` を付ける (`TwinTails` はパネルで差し替えて試す) |
| ファイルをウィンドウにドロップ | そのファイルを開く。`.ugoemod` は読み込んで付ける。モデルとモジュールを一緒に落とすと、モデルを先に開く。フォルダや開けないファイルが混ざっていたら、開けるものだけを開き、飛ばしたものをステータスバーに出す。落とす前 (ドラッグ中) に、開けるかを枠の色と文で示す (アプリはパスから。T0717。ブラウザは名前が分からないので種類と数から。T0718) |
| `ugoe-viewer path/to/model.ugoe` | 起動時に開く |
| ホイール | カーソルの位置を中心にズーム |
| ドラッグ (左・中ボタン) | パン。光源の印 (ライティングを使うモデル) を掴んだときは光源を動かす |
| ダブルクリック・F・Fit | 全体を表示 (光源の取っ手にフォーカスがあるときも F はフィット。T0717) |
| 覚えたもの | モデルごとに覚えた光源と着せ替えの組の一覧。モデルごとに・すべて忘れる (下の「[覚えたものの一覧](#覚えたものの一覧-t0714)」) |
| 背景・透明 | 背景色。透明にすると市松模様の上に描く |
| 着せ替え | スロットのあるモデルか、モジュールを読み込んだときに右のパネルに出る。スロットごとにモジュールを付ける・外す・差し替える (下の「着せ替え」) |
| ライティング | ライティングを使うモデルのときだけ右のパネルに出る。光源の向き・位置・色・強さと既定の光源の組 (下の「ライティング」) |
| スライダー | パラメータ。ダブルクリックか ↺ で既定値、「すべて戻す」で全部を既定値に戻す。中継用のパラメータ (`internal`、物理のリグの間の受け渡し用) は既定で隠し、「中継用のパラメータも表示」で出す |

## 着せ替え (T0706)

スロットを持つモデル (本体) に、着せ替えのモジュール (`.ugoemod`) を付け外しして試す。仕組みは
[docs/spec/modules.md](../../docs/spec/modules.md) と [ADR 0024](../../docs/adr/0024-module-authoring.md)、wasm の口は
[crates/ugoe-wasm/README.md#着せ替え-t0123](../../crates/ugoe-wasm/README.md#着せ替え-t0123)。

- 「着せ替え」のパネルの「モジュールを開く…」(複数選べる)、ツールバーの「開く」、ウィンドウへのドロップで `.ugoemod` を読み込む。
  モデルを開いていれば、読み込んだモジュールをそのまま付ける (同じスロットに付いているモジュールとは差し替わる)。
  同じ名前のモジュールを読み込み直すと新しい版に置き換える (付けていれば付け替える)。
- パネルはモデルのスロットを定義の順に並べ、スロットごとに付けているモジュール (無ければ「なし」) と、読み込んだそのスロットの
  モジュールを出す。モデルに無いスロットのモジュールは「モデルに無い」の印を付けて後ろに並べ、付けられない。

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
- 読み込みのほかの警告 (T0125) も `load_warnings_json()` の `code` から日本語にする: テクスチャを白で代用した (`texture_ktx2`・
  `texture_missing`)、法線マップを使わない (`normal_map_ktx2`)、物理の設定が合わないので外した (`physics_removed`。理由は英語のまま括弧で添える)。
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
  日本語でステータスバーに出す (`skippedMessage`。次に開くまで出す)。ブラウザの File API のドロップも同じ経路。
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

モデルごとの設定 (着せ替えの組と、光源の設定 (T0709)) は、`modelMemory.ts` の部品で覚える。

| 覚えるもの | localStorage のキー | 上限 | 忘れる操作 |
| --- | --- | --- | --- |
| 着せ替えの組 (`dressUpMemory.ts`) | `ugoe-viewer.dress-up` | 20 体 | 着せ替えのパネルの「覚えた組を忘れる」、「覚えたもの」の一覧 |
| 光源の設定 (`lighting.ts` の `lightingMemory`) | `ugoe-viewer.lighting.models` | 50 体 | 「覚えたもの」の一覧 (T0714。T0709 ではパネルの「既定に戻す」だった) |


- **モデルの鍵 (`ModelIdentity`):** `{ name, fingerprint, location? }`。
  - `location` (`FileLocation`) は後から読み直せる場所: `path:<絶対パス>` (Tauri のダイアログ・ドロップ・起動時の引数)、
    `sample:<samples/ からのパス>` (同梱の見本)、`url:<絶対 URL>` (ブラウザの `?model=`)。ブラウザでドロップした・選んだファイルには無い。
    `host.ts` がファイルを読むときに `ModelFile.location` に入れ、`readFileAt(location)` で読み直す。
  - `fingerprint` はバイト列の指紋 (長さと 2 つの 32 ビットのハッシュ。暗号のハッシュではない。`fingerprint()`)。
  - `name` は表示だけに使い、比べない (`model.ugoe` のような同じ名前の別のモデルがよくあるため)。
- **同じモデル (`sameModel`):** 場所が同じか、指紋が同じ。場所が同じなら中身が変わっても (書き出し直し) 同じモデル。ブラウザでの
  ドロップで場所が分からなくても、中身が同じなら同じモデル。`ModelMemory.get` は場所が同じ項目を優先し、無ければ指紋が同じ項目を返す。
  `set` は同じモデルの古い項目を消して新しい鍵で覚える (ブラウザでドロップして覚えたモデルを、場所の分かる開き方で開くと、場所が付く)。
- **入れ物 (`ModelMemory<T>`):** `new ModelMemory(key, parse, { limit })`。localStorage の 1 つのキーに
  `{"version": 1, "entries": [{"model": ModelIdentity, "value": T}]}` を最近使った順に置き、`limit` を超えたら古いものから消す。
  `get`・`set`・`delete`・`models`・`entries`・`clear` (後の 2 つは覚えたものの一覧、T0714)。値は `parse` (unknown → `T | null`) が確かめ、読めない項目・例外を出した項目・
  壊れた JSON・違う版・使えない localStorage では、何も覚えていないものとして動き、例外を出さない。
- 光源の設定 (T0709) も `Player.load` で作る鍵 (`modelIdentity(file)`) で `ModelMemory<LightingSettings>` に覚える
  (`parse` は `parseModelLighting`。オブジェクトでない値は読み飛ばし、中の壊れた値は `sanitizeLighting` で直す)。
- 確かめるための本体とモジュールはリポジトリに無い。`ugoe_model::fixtures` の `dress_up_base`・`long_hair_module`・`dress_module` を
  `ugoe_format` の `write_bundle` / `module_file::write_module` で書き出して作る (T0706 のメモ)。

### 覚えたものの一覧 (T0714)

ツールバーの「覚えたもの」(とライティングのパネルの「覚えたもの…」) で、モデルごとに覚えたもの (光源の設定と着せ替えの組) を
一覧にする (`MemoryDialog.svelte`)。2 つの入れ物の項目を同じモデル (`sameModel`) ごとに 1 行にまとめ (`rememberedModels.ts` の
`mergeRemembered`)、開いているモデルを先頭に、あとは「並び」で選んだ順に並べる。既定は覚えた日時の新しい順で、名前の順も選べる (T0717)。選んだ並びは localStorage の
`ugoe-viewer.remembered-order` に覚え (`loadRememberedOrder`・`saveRememberedOrder`)、次に一覧を開いたときも使う
(T0718。知らない値・読めないときは新しい順)。

- **覚えた日時 (T0717):** `ModelMemory` の項目に `at` (最後に覚えた日時、ミリ秒) を持たせた。`set` のたびに今の時刻にする
  (`ModelMemoryOptions.now` で差し替えられる)。保存の形の版 (1) は変えない: `at` の無い前の保存値もそのまま読め、その項目は
  「日時なし」として新しい順の最後 (その中は名前の順) に並ぶ。覚え直すと日時が付く。行の日時は光源と組の新しい方。

| 列 | 内容 |
| --- | --- |
| モデル | 名前と、覚えた場所 (見本・パス・URL。場所の無いものは「場所なし (中身で見分ける)」)。開いているモデルには「開いている」 |
| 光源 | 既定の組の名前か「カスタム」「切」と「忘れる」。覚えていなければ「—」 |
| 着せ替えの組 | 付けていたモジュールの名前と「忘れる」。覚えていなければ「—」 |
| 覚えた日時 | 光源と組のどちらかを最後に覚えた日時 (`2026/10/01 09:05`)。前の保存値だけなら「日時なし」 |
| (操作) | 光源と組の両方を覚えていれば「両方を忘れる」 |

- 「すべて忘れる…」は一覧の中で確かめてから (「すべて忘れる」「やめる」)、2 つの入れ物のキーを消す (`ModelMemory.clear`)。
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
- ライセンスは両方とも Apache-2.0。表記は `public/third-party-notices.txt` にあり、アプリのステータスバーの「ライセンス」で表示する。

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

右のパネル (`App.svelte` の `.sidebar`) は縦にパネルを積む作りで、上からトラッキング → モデルの機能ごとのパネル (着せ替え・ライティング。
そのモデルが使うときだけ出す、畳める `<details>`) → パラメータ (残りの高さ) の順に並ぶ。

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
just viewer-build                # リリースビルド (インストーラを作る)。例: just viewer-build --bundles deb
pnpm --filter @ugoe/viewer wasm  # ugoe-wasm をビルドして src/lib/wasm/generated/ に JS バインディングを出す
pnpm --filter @ugoe/viewer mediapipe  # MediaPipe の wasm とモデルを public/mediapipe/ に用意する (dev・build でも実行される)
just web-check                   # Biome + svelte-check + Vitest (エディタとビューアの両方)
```

`just viewer` と `just viewer-build` は、先に `pnpm wasm` を実行する (`tauri.conf.json` の `beforeDevCommand` /
`beforeBuildCommand`)。必要な道具 (Rust・Node.js・pnpm・just・wasm-bindgen-cli・各 OS の前提パッケージ) はエディタと同じなので、
[apps/editor/README.md#各-os-でのビルド](../editor/README.md#各-os-でのビルド) を参照。
GitHub Actions の Editor build のワークフローは、ビューアのインストーラ (`ugoe-viewer-<OS>`) も作る。

アイコンは `src-tauri/icons/icon.png` から `pnpm tauri icon src-tauri/icons/icon.png -o src-tauri/icons` で作り直せる
(Android・iOS 用に出力されるディレクトリは使わないので消す)。
