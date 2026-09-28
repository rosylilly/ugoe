# apps/viewer

ugoe のビューア (Tauri v2 + Svelte 5)。`.ugoe` バンドルか JSON のモデル (`model.json`) を開いて表示し、
パラメータを動かして確かめるだけの軽いアプリ。ランタイムをアプリに組み込むときの見本と、
フェイストラッキング (T0703) の確認の場を兼ねる。

**ライセンスは GPL-3.0-only** (エディタと同じ。[ADR 0019](../../docs/adr/0019-viewer-license.md))。
エディタのコードは写してよいが、アプリ同士で import はしない。

```
src/                  フロントエンド (Svelte 5 + TS。npm パッケージ @ugoe/viewer)
  App.svelte            ツールバー・ビューポート・右のパネル・ステータスバー、ドロップとショートカット
  lib/player.svelte.ts  WebPuppet (ugoe-wasm) を持ち、フレームのループを回す。UI の状態もここ
  lib/drivers.ts        パラメータを毎フレーム外から動かす仕組み (ParameterDriver)
  lib/host.ts           ファイルの選択と読み込み (Tauri / ブラウザ / 差し替え)
  lib/Viewport.svelte   描画領域 (ズーム・パン・フィット)
  lib/ParameterPanel.svelte  パラメータのスライダー (グループごと)
  lib/wasm/             ugoe-wasm の読み込み。generated/ は `pnpm wasm` の出力 (gitignore 済み)
src-tauri/            バックエンド (Rust クレート `ugoe-viewer`)
```

## 仕組み

- 描画もパラメータの操作も、webview の中の `ugoe-wasm` (`WebPuppet`) で行う (エディタのビューポートと同じ、
  [ADR 0003](../../docs/adr/0003-editor-viewport-in-webview.md))。WebGPU が無ければ WebGL2 で描く。
- バックエンドは webview だけではできないことに限る: モデルのファイルをバイト列で読む (`read_model_file`) ことと、
  起動時の引数のパスを返す (`initial_file`) こと。依存してよいのはランタイム層のクレートだけ (`scripts/check-layers.mjs`)。
  いまは内部のクレートに依存していない。
- 形式は中身で判別する (先頭が `UGOE` ならバンドル、`{` なら JSON)。
- パラメータの値域・既定値・グループは `WebPuppet.parameters_json()` から得る。

## 使い方

| 操作 | 内容 |
| --- | --- |
| 開く (Ctrl+O / ⌘O) | OS のダイアログで `.ugoe` か `.json` を選ぶ |
| ファイルをウィンドウにドロップ | そのファイルを開く |
| `ugoe-viewer path/to/model.ugoe` | 起動時に開く |
| ホイール | カーソルの位置を中心にズーム |
| ドラッグ (左・中ボタン) | パン |
| ダブルクリック・F・Fit | 全体を表示 |
| 背景・透明 | 背景色。透明にすると市松模様の上に描く |
| スライダー | パラメータ。ダブルクリックか ↺ で既定値、「すべて戻す」で全部を既定値に戻す |

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
- ドライバが値を返しているパラメータのスライダーは「(自動)」と表示され、手では動かせない。
- 開発中やヘッドレスのブラウザでは、表示中の `Player` を `window.__UGOE_VIEWER__` から触れる。

右のパネル (`App.svelte` の `.sidebar`) は縦にパネルを積む作りで、トラッキングの映像と値のパネルはパラメータの上に足す。

## ヘッドレスのブラウザで確かめる

Tauri の外 (ふつうのブラウザ) で開くと、ファイルの選択は `<input type="file">` になる。さらに:

- `?model=<URL>` を付けると、起動時にその URL のモデルを開く (例: `http://localhost:1430/?model=/c.ugoe`)。
- `window.__UGOE_VIEWER_HOST__` に `ViewerHost` (`src/lib/host.ts`) を置くと、「開く」などでそれを使う。
- ドロップは `DragEvent` と `DataTransfer` に `File` を入れて `window` に送れば確かめられる。
- WebGPU の無いヘッドレスの Chrome でも、SwiftShader の WebGL2 で描ける
  (`--use-gl=angle --use-angle=swiftshader --enable-unsafe-swiftshader`)。

## コマンド

```sh
just viewer                      # 開発モードで起動 (vite の 1430 番ポート + tauri dev)
just viewer-build                # リリースビルド (インストーラを作る)。例: just viewer-build --bundles deb
pnpm --filter @ugoe/viewer wasm  # ugoe-wasm をビルドして src/lib/wasm/generated/ に JS バインディングを出す
just web-check                   # Biome + svelte-check + Vitest (エディタとビューアの両方)
```

`just viewer` と `just viewer-build` は、先に `pnpm wasm` を実行する (`tauri.conf.json` の `beforeDevCommand` /
`beforeBuildCommand`)。必要な道具 (Rust・Node.js・pnpm・just・wasm-bindgen-cli・各 OS の前提パッケージ) はエディタと同じなので、
[apps/editor/README.md#各-os-でのビルド](../editor/README.md#各-os-でのビルド) を参照。
GitHub Actions の Editor build のワークフローは、ビューアのインストーラ (`ugoe-viewer-<OS>`) も作る。

アイコンは `src-tauri/icons/icon.png` から `pnpm tauri icon src-tauri/icons/icon.png -o src-tauri/icons` で作り直せる
(Android・iOS 用に出力されるディレクトリは使わないので消す)。
