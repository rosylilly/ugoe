# packages/player

モデルの再生の層 (npm パッケージ `@ugoe/player`、pnpm のワークスペース)。ビューア (`apps/viewer`) とスタジオ (`apps/studio`、
[ADR 0028](../../docs/adr/0028-studio.md)) で共有する。T0904 でビューアから切り出した ([ADR 0029](../../docs/adr/0029-shared-ui-and-i18n.md))。

**ライセンスは GPL-3.0-only** (使うアプリと同じ)。依存は `@ugoe/face-tracking` だけ。`@ugoe/ui` には、パネルをここへ移すときに依存する。

```
src/index.ts             公開するもの (アプリはここからだけ import する)
src/player.svelte.ts     Player: WebPuppet (ugoe-wasm) を持ち、フレームのループを回す。UI に出す状態 ($state) もここ。
                         PlayerHost (アプリが渡すもの)・Screenshot (capture の結果)
src/wasm.ts              Player が使う WebPuppet の口 (構造だけ。生成物はアプリが持つ)
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
- `Player` は `@ugoe/face-tracking` の `TrackingTarget` を満たす (`new FaceTracker(player, …)`)。
- 画像の保存は `capture()` で RGBA を受け取り、PNG にして書くのはアプリ。
- 覚えるもの (光源・着せ替えの組・待機モーションなど) の `localStorage` のキーは、いまはビューアの名前 (`ugoe-viewer.*`) のまま。
  スタジオで使うときに、キーの接頭辞をアプリが渡せるようにする。
- 文言は日本語の直書き。`@ugoe/i18n` に移すのは後の作業 (ADR 0029)。

## 受け手のモード (T0801 で足す)

スタジオの出力ウィンドウは、指揮役が配るフレーム `{seq, dt, values}` を順に適用して物理を回す受け手になる
([docs/studio.md](../../docs/studio.md))。`Player` のフレームのループは、時計と描画を受け持つ `#tick` と、1 フレーム分を
進める `#advance(puppet, dt)` (ドライバ → トラッキング → モーションの設定 → `update(dt)` → 値の同期) に分けてある。
受け手のモードは、`#tick` で時計の `dt` の代わりに届いたフレームを順に取り出し、`#advance` の代わりに「値を書いて
`update(dt)`」する分岐として足す想定。
