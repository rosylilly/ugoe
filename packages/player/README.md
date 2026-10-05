# packages/player

モデルの再生の層 (npm パッケージ `@ugoe/player`、pnpm のワークスペース)。ビューア (`apps/viewer`) とスタジオ (`apps/studio`、
[ADR 0028](../../docs/adr/0028-studio.md)) で共有する。T0904 でビューアから切り出した ([ADR 0029](../../docs/adr/0029-shared-ui-and-i18n.md))。

**ライセンスは GPL-3.0-only** (使うアプリと同じ)。依存は `@ugoe/face-tracking` と `@ugoe/i18n`。`@ugoe/ui` には、パネルをここへ移すときに依存する。

```
locales/                 利用者に見せる文 (エラー・警告・既定の光源の組の名前など) のカタログ (名前空間 `player`。en.json が原文)
src/index.ts             公開するもの (アプリはここからだけ import する)
src/i18n.ts              文言の `t()` (@ugoe/i18n の名前空間 `player`)
src/player.svelte.ts     Player: WebPuppet (ugoe-wasm) を持ち、フレームのループを回す。UI に出す状態 ($state) もここ。
                         PlayerHost (アプリが渡すもの)・Screenshot (capture の結果)
src/wasm.ts              Player が使う WebPuppet の口 (構造だけ。生成物はアプリが持つ)
src/surface.ts           PuppetSurface: WebPuppet と canvas の寿命 (作成・WebGL2 への切り替え・GPU を失ったときの作り直し・破棄・
                         devicePixelRatio の追従)。エディタのビューポートも使う (T0932。docs/ui.md#webpuppet-の寿命)
src/follower.ts          受け手のモード (スタジオの出力ウィンドウ): フレームの読み取り・並べ替え・適用 (T0801)
src/drivers.ts           パラメータを毎フレーム外から動かす仕組み (ParameterDriver)
src/parameters.ts        パラメータの一覧 (parameters_json) の読み取り・グループ分け・スライダーの刻み
src/animation.ts         モーションと表情の一覧の読み取り、トラッキングとの重なり、待機モーション (T0720〜T0722)
src/lighting.ts          光源の設定・既定の組・set_lighting_json の JSON・localStorage への保存・光源の印の幾何 (T0708・T0709)
src/color.ts             色の文字列 (#rrggbb) の読み取り
src/dressUp.ts           着せ替えの一覧の読み取り・公開するスロットの絞り込み・エラーと警告の文 (T0706・T0715)
src/dressUpMemory.ts     付けていたモジュールの組をモデルごとに覚える形と、戻すときにファイルをどこから得るか (T0711)
src/loadWarnings.ts      読み込みの警告 (load_warnings_json) の読み取りと文 (T0125)
src/loadError.ts         読み込みの失敗 (wasm の LoadError) の文 (T0434)
src/modelFile.ts         開くファイル (ModelFile) と場所 (FileLocation)、形式の判別
src/modelMemory.ts       モデルを見分ける鍵と、モデルごとの値を上限付きで localStorage に覚える入れ物 (T0711)
src/rememberedModels.ts  モデルごとに覚えたものを 1 モデル 1 行にまとめる (T0714)
src/screenshot.ts        スクリーンショットの画素の検査・RGBA から PNG への変換・既定の名前の日時・パスのヘッダー (エディタとも共有。T0730)
src/storage.ts           覚えるものの localStorage の読み書き (PlayerStorage: 接頭辞と Storage の組。T0930)
browser-test/            PuppetSurface を本物の wasm とヘッドレスの Chrome で確かめるテスト (`just player-browser`。下の「ブラウザのテスト」)
src/testing/              テスト用の部品 (偽の WebPuppet・Player を動かす準備。T0933。localStorage の代わりは `@ugoe/i18n/testing` の `MemoryStorage`)。index.ts から出さない
```

画面の部品 (パラメータ・ライティング・着せ替え・モーションのパネル、ビューポート、覚えたものの一覧) は、いまはビューアに
ある (`apps/viewer/src/lib/*.svelte`)。`@ugoe/ui` ができた後で、部品に作り直してここへ移す。

## 使い方

```ts
import { Player } from "@ugoe/player";

const player = new Player({
  loadWasm, // ugoe-wasm を初期化して返す (生成物が無ければ null)
  readFile, // 覚えた場所 ("path:" / "sample:" / "url:") からファイルを読み直す
  wasmMissingMessage: "…", // 省くと「ugoe-wasm がビルドされていません」
});
await player.attach(container); // container の中に canvas を作り、描画を始める
await player.load(file, modules); // モデル (と一緒に開くモジュール)
```

- **wasm はアプリが渡す。** 生成物の置き場所と読み込み方 (ビューアは `apps/viewer/src/lib/wasm/load.ts` の `import.meta.glob`) は
  アプリが決める。`src/wasm.ts` の `WebPuppet` は Player が使うメソッドだけを書いた構造の型で、アプリの生成物
  (`ugoe_wasm.d.ts`) がそれを満たすかはアプリの型検査で確かめる。Player が新しいメソッドを使うときは `src/wasm.ts` に足す。
- `attach` は前の `attach` を先に `detach` する。WebPuppet を作っている間に `detach` か次の `attach` があれば、遅れて
  できた WebPuppet は捨てる (アンマウントと再マウントが重なっても資源が残らない)。GPU を失ったら (WebGPU も WebGL2 も `is_lost`)
  canvas ごと作り直してモデルを読み直す。WebGL2 はコンテキストの復元 (`webglcontextrestored`) を待ってから作り直す。前の作り直しから
  1 分以内に失うことが続き 4 回目になったら、作り直さずに `status` を `unavailable`、`error` を `errors.gpuLostRepeatedly` にする
  (`<ugoe-avatar>` と同じ数と時間。T0935)。devicePixelRatio だけが変わったときも
  描画バッファの大きさを合わせ直す (アプリの ResizeObserver は知らせないため)。この寿命は `src/surface.ts` の `PuppetSurface` が持ち、
  エディタのビューポートも同じものを使う ([docs/ui.md](../../docs/ui.md#webpuppet-の寿命))。
- `Player` は `@ugoe/face-tracking` の `TrackingTarget` を満たす (`new FaceTracker(player, …)`)。
- 画像の保存は `capture()` で RGBA を受け取り、`encodePng` (`src/screenshot.ts`) で PNG にする。保存のダイアログと書き込みはアプリ。
  `src/screenshot.ts` はエディタのビューポート (`ViewportController.capture`) も使う (T0730。エディタが `@ugoe/player` から import するのは
  これと `PuppetSurface`)。例外の文はカタログの `screenshot.*`。
- 覚えるもの (光源・着せ替えの組・待機モーションなど) の `localStorage` のキーは「接頭辞 + `.` + 名前」(`src/storage.ts`。T0930)。
  接頭辞は `PlayerHost.storagePrefix` で渡し、省くとビューアの `ugoe-viewer` (前から覚えたものを失わない)。スタジオの出力ウィンドウは
  `ugoe-studio.player`。名前の一覧は `STORAGE_NAMES` (下の表)。`PlayerHost.storage` で覚える先 (`Storage`) も差し替えられる (テスト)。
  - 読み書きはすべて `PlayerStorage` (`player.storage`) を通す (中は `@ugoe/i18n` の `readStoredItem` など。T0937)。localStorage を引くだけで例外になる環境・壊れた JSON・容量の超過では、
    覚えていないものとして動き、例外を出さない。`load*`・`save*` の関数と `ModelMemory` はこれを受け取る。
  - Player の外で覚えるもの (ビューアの「覚えたもの」の一覧の並び `loadRememberedOrder`・`saveRememberedOrder`) も
    `player.storage` を渡して同じ接頭辞で覚える。
  - `PlayerStorage.clear()` はその接頭辞で覚えたものをすべて消す (アプリが古い値を捨てるとき)。
  - **保存の形の版 (T0937):** `ModelMemory` の値 (`{version, entries}`) が今より新しい版 (整数で今の版より大きい。新しい版のアプリが
    同じ localStorage に覚えた) なら、読まずに、書き換えも消しもしない。その間の覚える・忘れる・すべて忘れるは入れ物の中だけで効き、
    起動し直すと残らない (1 つのキーに多くのモデルの値が入るので、1 つ覚えるだけで新しい版の値をすべて失わないように)。
    壊れた値・版の無い値は、前と同じく次に覚えるときに書き直す。版を上げるときは、前の版の値を今の形に直して読む。
    1 つの値だけを持つもの (`lighting` の `{version, settings}`) は、エディタ・スタジオの設定と同じく、起動では書かず、
    利用者が変えたときに今の版で書く (T0011)。

  | 名前 (`STORAGE_NAMES`) | キーの名前の部分 | 値 |
  | --- | --- | --- |
  | `lighting` | `lighting` | 最後に使った光源の設定 (`{version, settings}`) |
  | `modelLighting` | `lighting.models` | モデルごとの光源の設定 (`ModelMemory`) |
  | `lightMarkers` | `lighting.markers` | 光源の印を出すか (`on`・`off`) |
  | `dressUp` | `dress-up` | モデルごとの着せ替えの組 (`ModelMemory`) |
  | `autoRestore` | `dress-up.auto-restore` | 組を尋ねずに戻すか (`true`・`false`) |
  | `idleMotion` | `idle-motion.models` | モデルごとの待機モーションの選択 (`ModelMemory`) |
  | `autoIdle` | `auto-idle` | 待機モーションを自動で流すか (`on`・`off`) |
  | `motionOverTracking` | `motion-over-tracking` | トラッキング中もモーションを優先するか (`on`・`off`) |
  | `trackingKept` | `motion-over-tracking.kept` | トラッキングのままにするパラメータ (JSON の文字列の配列) |
  | `rememberedOrder` | `remembered-order` | 覚えたものの一覧の並び (`recent`・`name`) |
- 文言は `@ugoe/i18n` の名前空間 `player` (`locales/`。T0723)。言語はアプリが `setLocale()` で決める。
  `describe*`・`lightingLabel` などの関数と、`MOTION_PRIORITIES`・`TRACKING_GROUPS`・`PRESETS` の `label`・`title`・`name` (読むたびに引く) は、
  呼んだときの言語の文を返すので、画面のテンプレートや `$derived` の中で使えば言語の切り替えに追従する。
  `error`・`warnings`・`dressUpError`・`motionNotice` のように状態に入れた文は、作ったときの言語のまま (次に作り直すまで)。
- モデルの読み込みの失敗は、wasm の `LoadError` の `code` と補足の値から `loadError.*` の文にして `error` に入れる
  (`describeLoadError`。T0434。コードの一覧は `crates/ugoe-wasm/README.md#読み込みの失敗-t0434`)。原因が細かいもの (壊れたファイル・
  検査のエラーなど) は wasm の英語の文を括弧で添え、知らないコードは英語の文のまま出す。`load_module` が上限を超えた `DressUpError`
  (`read_limit_exceeded`・`image_limit_exceeded`) も同じ文にする。アプリ (ビューア・スタジオ) は `error` を出すだけで、訳を持たない。
- テストは Svelte のプラグインを通す (`vitest.config.ts`。@ugoe/i18n が runes を使うため)。文を確かめるテストは `setLocale("ja")` で言語を決めてから比べる。

## 受け手のモード (T0801)

スタジオの出力ウィンドウは、指揮役が配るフレーム `{seq, revision, dt, values, lipSync}` を順に適用して物理を回す受け手になる
([docs/studio.md](../../docs/studio.md#フレームの配信-内部の形)、ADR 0028 の 3.)。

```ts
const player = new Player({ loadWasm, readFile });
player.startFollowing();             // 受け手のモードにする (時計の dt では進めない)
await player.attach(container);
// シーンの通知を受けたら、そのモデルを読んでから、そのシーンのフレームを受け付ける
await player.load({ name: scene.modelName, bytes });
player.followScene(scene.revision);
// フレーム (バイト列) が届くたびに
player.pushFrame(decodeFrame(buffer));
```

- 受け手のモードでは時計の `dt` を使わず、届いたフレームを `FrameQueue` (`src/follower.ts`) から順にすべて取り出して `follow` で適用する
  (物理 → リップシンク → ポーズ。T0134)。適用は `pushFrame`・`followScene` の **その場で** 行い、`#tick` (`requestAnimationFrame`) は描くだけ
  (残っていれば適用してから描く)。ページが隠れて `requestAnimationFrame` が止まっても物理は止まらない (T0804、docs/studio.md#隠れても止まらない)。
  ドライバ・トラッキング・モーション・待機モーションは使わない (値は指揮役が決める)。`values` (スライダーの表示) も同期しない。
- `startFollowing(onRender)` の `onRender` は、描くたびに `{at, renderMs, dropped}` (前に描いてからの適用と描画の CPU の時間・溢れて捨てたフレームの合計) を受け取る
  (スタジオの負荷の表示)。
- 前のシーンのフレームは捨て、まだ読み終えていないシーンのフレームは待たせる。同じシーンの中で `seq` が飛んだら (待たせすぎて
  溢れたときも) 物理をリセットしてから続ける。
- モデルを読み直さずに次のシーンへ移るとき (スタジオで見せ方や着せ替えだけが変わった。T0802) は `continueScene(revision)`。
  前のシーンのフレームも捨てずに順に適用し、`seq` の続きも保つ (受け手ごとに物理の刻みが抜けないように)。
- 受け手のモードでは、光源の設定と着せ替えの組を覚えず、覚えたものも使わない (どちらもシーンが決める)。待機モーションの選択も
  読まず書き直さない (受け手は待機モーションを流さない。T0930)。
- `WebPuppet.follow` を呼ぶのは `src/follower.ts` の `applyFrame` だけ (`follow` の引数が変わったら、こことフレームの形を合わせる)。
- `src/wasm.ts` の `follow`・`reset_physics` は省ける (受け手のモードを使わないビューアの型の宣言に無くてよいように)。
  受け手のモードで無ければ `error` に書いて進めない。
- フレームのバイト列の形はスタジオのバックエンド (`apps/studio/src-tauri/src/stream.rs`) と同じで、両方のテストが同じバイト列を読む。

## ブラウザのテスト

`browser-test/` は `PuppetSurface` を本物の wasm (`just wasm` の生成物) とヘッドレスの Chrome の WebGL2 で動かし、作成・描画・
WebGL2 の喪失からの作り直し (復元を待つ。T0935)・続けて失ったときの諦め・作っている間の破棄を確かめる (T0936)。
`just player-browser` で流す (Chrome と wasm-bindgen-cli が要る。`just ci` には入れず、CI の `conformance` ジョブで流す)。
回し方とシナリオは [docs/testing.md](../../docs/testing.md#puppetsurface-のブラウザのテスト)。

- `run.mjs` (Node): ugoe-wasm をビルドし (`--no-build` で省く)、`src/surface.ts` を TypeScript の `transpileModule` で JS にし、
  小さなサーバで配って playwright-core で Chrome を開く。結果の置き場は `target/player-browser/` (ページのコンソールは `page.log`)。
- `scenarios.js` (ページ): シナリオ。足すときは `scenarios` に関数を足す (失敗は例外で知らせる)。
- **置き場所をここにした理由 (ライセンス):** テストは GPL-3.0-only の `surface.ts` をそのまま読むので、GPL のパッケージの中に置く。
  MIT の `tests/conformance` に置くと、MIT と書いた場所が GPL のコードを読み込むことになる (AGENTS.md の「外部のコードとライセンス」)。
  `tests/ui-audit` もライセンスを書いておらず、リポジトリの既定 (ルートの LICENSE。MIT) になるので使わない。
