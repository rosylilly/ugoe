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
src/follower.ts          受け手のモード (スタジオの出力ウィンドウ): フレームの読み取り・並べ替え・適用 (T0801)
src/drivers.ts           パラメータを毎フレーム外から動かす仕組み (ParameterDriver)
src/parameters.ts        パラメータの一覧 (parameters_json) の読み取り・グループ分け・スライダーの刻み
src/animation.ts         モーションと表情の一覧の読み取り、トラッキングとの重なり、待機モーション (T0720〜T0722)
src/lighting.ts          光源の設定・既定の組・set_lighting_json の JSON・localStorage への保存・光源の印の幾何 (T0708・T0709)
src/color.ts             色の文字列 (#rrggbb) の読み取り
src/dressUp.ts           着せ替えの一覧の読み取り・公開するスロットの絞り込み・エラーと警告の文 (T0706・T0715)
src/dressUpMemory.ts     付けていたモジュールの組をモデルごとに覚える形と、戻すときにファイルをどこから得るか (T0711)
src/loadWarnings.ts      読み込みの警告 (load_warnings_json) の読み取りと文 (T0125)
src/modelFile.ts         開くファイル (ModelFile) と場所 (FileLocation)、形式の判別
src/modelMemory.ts       モデルを見分ける鍵と、モデルごとの値を上限付きで localStorage に覚える入れ物 (T0711)
src/rememberedModels.ts  モデルごとに覚えたものを 1 モデル 1 行にまとめる (T0714)
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
  できた WebPuppet は捨てる (アンマウントと再マウントが重なっても資源が残らない)。GPU を失ったら (WebGPU は `is_lost`、
  WebGL2 は canvas の `webglcontextlost`) canvas ごと作り直してモデルを読み直す。devicePixelRatio だけが変わったときも
  描画バッファの大きさを合わせ直す (アプリの ResizeObserver は知らせないため)。
- `Player` は `@ugoe/face-tracking` の `TrackingTarget` を満たす (`new FaceTracker(player, …)`)。
- 画像の保存は `capture()` で RGBA を受け取り、PNG にして書くのはアプリ。
- 覚えるもの (光源・着せ替えの組・待機モーションなど) の `localStorage` のキーは、いまはビューアの名前 (`ugoe-viewer.*`) のまま。
  スタジオで使うときに、キーの接頭辞をアプリが渡せるようにする。
- 文言は `@ugoe/i18n` の名前空間 `player` (`locales/`。T0723)。言語はアプリが `setLocale()` で決める。
  `describe*`・`lightingLabel` などの関数と、`MOTION_PRIORITIES`・`TRACKING_GROUPS`・`PRESETS` の `label`・`title`・`name` (読むたびに引く) は、
  呼んだときの言語の文を返すので、画面のテンプレートや `$derived` の中で使えば言語の切り替えに追従する。
  `error`・`warnings`・`dressUpError`・`motionNotice` のように状態に入れた文は、作ったときの言語のまま (次に作り直すまで)。
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
- 受け手のモードでは、光源の設定と着せ替えの組を覚えず、覚えたものも使わない (どちらもシーンが決める)。
- `WebPuppet.follow` を呼ぶのは `src/follower.ts` の `applyFrame` だけ (`follow` の引数が変わったら、こことフレームの形を合わせる)。
- `src/wasm.ts` の `follow`・`reset_physics` は省ける (受け手のモードを使わないビューアの型の宣言に無くてよいように)。
  受け手のモードで無ければ `error` に書いて進めない。
- フレームのバイト列の形はスタジオのバックエンド (`apps/studio/src-tauri/src/stream.rs`) と同じで、両方のテストが同じバイト列を読む。
