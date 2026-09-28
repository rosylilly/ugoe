# ugoe-wasm

**層:** L5 バインディング (依存してよいクレートは `scripts/check-layers.mjs` を参照)

WebAssembly バインディング。Web ビューアとエディタのビューポートで使う。

## 守ること

- WebGPU がなくても WebGL2 で動くこと
- ブラウザに依存するコードは `src/web.rs` (wasm32 のときだけビルド) に閉じ込め、
  それ以外 (`load`・`view`・`hit`) はネイティブでテストできるように保つ

## 構成

| モジュール | 内容 |
| --- | --- |
| `web` | JS に公開する `WebPuppet`。canvas に wgpu のサーフェスを作り、クリアと描画を行う |
| `load` | JSON / `.ugoe` バンドルの読み込み、PNG のデコード (乗算済みアルファへ)、パラメータ値の引き継ぎ |
| `view` | フィット・ズーム・パンと、キャンバス座標 ↔ 画面座標の変換 |
| `hit` | 評価済みの要素の外接矩形とヒットテスト (エディタの選択用) |

## `WebPuppet` の使い方

```js
import init, { WebPuppet } from "./generated/ugoe_wasm.js";
await init();
const puppet = await WebPuppet.create(canvas, "auto"); // "auto" | "webgpu" | "webgl2"
puppet.load_json(json);           // または load_bundle(bytes)。戻り値は警告の配列
puppet.resize(canvas.clientWidth * devicePixelRatio, canvas.clientHeight * devicePixelRatio);
puppet.set_parameter("AngleX", 15);
puppet.render();                  // 呼ばれたときだけ描く。ループはホストが回す
```

- 画面座標は canvas の描画バッファの画素 (CSS ピクセル × `devicePixelRatio`)。
- `set_parameter` は評価を遅らせ、次の `render` や問い合わせでまとめて評価する。物理やモーションを動かすときは `update(dt)` を呼ぶ。
- モデルを読み直すと、同じ ID のパラメータの値は引き継がれる (エディタで編集・Undo のたびに読み直すため)。
- `"auto"` は WebGPU のアダプタを実際に要求して判定し、無ければ WebGL2 を使う。1 つの canvas は WebGPU と WebGL2 を併用できないので、作成に失敗したら新しい canvas で作り直す。
- `is_lost()` が `true` になったら (GPU プロセスのクラッシュなど)、新しい canvas で作り直す。

## ビルド

```sh
cargo install wasm-bindgen-cli --version "$(just wasm-bindgen-version)" --locked  # 初回だけ
just wasm        # apps/editor/src/lib/viewport/generated/ に出力 (gitignore 済み)
just wasm-check  # wasm32 向けの clippy
```

wasm-bindgen-cli の版は `Cargo.lock` の `wasm-bindgen` と一致させる必要がある (`just wasm` が検査する)。

## 仕様

- [docs/editor.md#ビューポート](../../docs/editor.md#ビューポート)
- [docs/adr/0003-editor-viewport-in-webview.md](../../docs/adr/0003-editor-viewport-in-webview.md)
- [docs/spec/rendering.md](../../docs/spec/rendering.md) (乗算済みアルファ、クリアはホストの責務)

## 関連タスク

- T0402 (`docs/tasks/T0402-*.md`)
