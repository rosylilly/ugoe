# @ugoe/runtime

ugoe のランタイムを Web で使うための npm パッケージです。中身は次の 3 つです。

- ugoe-wasm (`crates/ugoe-wasm`) の wasm と JS バインディング (`WebPuppet`) と、その TypeScript の型
- 初期化・作成・読み込みの薄い補助 (`initRuntime`・`createPuppet`・`loadModel`・`fetchModel`)
- Web Component の `<ugoe-avatar>`

WebGPU が使えれば WebGPU、使えなければ WebGL2 で描きます。ライセンスは **MIT** です
([ADR 0025](../../docs/adr/0025-web-runtime-package.md)。GPL-3.0-only の `packages/` とは別の場所に置いています)。

> まだ npm には公開していません (`private: true`)。下の「ローカルで使う」の手順で tarball を作って入れます。

## `<ugoe-avatar>`

```html
<script type="module">
  import "@ugoe/runtime/element"; // <ugoe-avatar> を登録する
</script>

<ugoe-avatar src="avatar.ugoe" parameters="AngleX: 15; EyeLOpen: 0.5" style="width: 400px; height: 400px"></ugoe-avatar>
```

| 属性 | 内容 |
| --- | --- |
| `src` | モデルの URL。`.ugoe` バンドル (先頭が `UGOE`) か、モデルの JSON |
| `backend` | `auto` (既定) / `webgpu` / `webgl2`。`auto` で WebGPU の作成に失敗したら、新しい canvas の WebGL2 で作り直す。変えると作り直す (モデルは読み直し、パラメータの値は引き継ぐ) |
| `parameters` | `"ID: 値; ID: 値"` (区切りは `;` か改行、`:` の代わりに `=` も可)。読み込みのたびと、属性を変えたときに設定する。属性から消した ID は元に戻さない |
| `expression` | バンドルの表情の名前。消すと表情を止める |
| `motion` | バンドルのモーションの名前。消すとモーションを止める |
| `motion-priority` | `idle` (既定) / `normal` / `force` |
| `paused` | 時間を止める (物理・モーション・目パチ・呼吸を進めない)。パラメータを変えたときだけ描き直す |

パラメータの ID は大文字と小文字を区別します (HTML の属性名は小文字になるので、パラメータごとの属性にはしていません)。

```js
const avatar = document.querySelector("ugoe-avatar");
avatar.addEventListener("load", (e) => console.log(e.detail.backend, e.detail.warnings));
avatar.addEventListener("error", (e) => console.error(e.detail.error));

avatar.setParameter("AngleX", -20);       // 値域にクランプ。ID が無ければ false
avatar.parameters = { AngleY: 10, BodyAngleX: 5 };
avatar.parameters;                         // { AngleX: -20, AngleY: 10, ... } (今の値)
avatar.parameterDefinitions;               // [{ id, name, min, max, default, ... }]
await avatar.load(bytesOrJsonString);      // src の代わりに直接読み込む。戻り値は警告
const puppet = await avatar.ready;         // WebPuppet そのもの (下)
puppet.set_lighting_json(JSON.stringify({ ambient: [0.5, 0.5, 0.5] }));
avatar.requestRender();                    // puppet を直接操作したら描き直しを頼む
```

- 大きさは要素の CSS の大きさ (既定 300x300) × `devicePixelRatio` です。背景は透明なので、要素の `background` がそのまま見えます。
- 画面外にあるとき (IntersectionObserver) と `paused` の間は `requestAnimationFrame` を回しません。
- GPU を失ったら (GPU プロセスのクラッシュなど)、新しい canvas で作り直してモデルを読み直します (パラメータの値は引き継ぐ。3 回まで)。
- ページから外すと GPU の資源を捨て、戻すと作り直します。
- 別の名前で登録するなら、`@ugoe/runtime/element` の代わりに `defineUgoeAvatar("my-avatar")` を呼びます。

## `WebPuppet` を直接使う

```js
import { createPuppet, fetchModel } from "@ugoe/runtime";

const canvas = document.querySelector("canvas");
canvas.width = canvas.clientWidth * devicePixelRatio;
canvas.height = canvas.clientHeight * devicePixelRatio;
const puppet = await createPuppet(canvas, { backend: "auto" }); // wasm の初期化もする
const warnings = await fetchModel(puppet, "avatar.ugoe");        // または loadModel(puppet, bytes)
puppet.set_parameter("AngleX", 15);
let last = performance.now();
requestAnimationFrame(function frame(now) {
  puppet.update(Math.min((now - last) / 1000, 0.1));
  last = now;
  puppet.render();
  requestAnimationFrame(frame);
});
```

`WebPuppet` の API (物理・モーション・表情・トラッキング・ライティング・着せ替えなど) は
[crates/ugoe-wasm/README.md](../../crates/ugoe-wasm/README.md) を見てください。

### wasm の場所

`dist/wasm/ugoe_wasm_bg.wasm` を `import.meta.url` からの相対で読みます。Vite などのバンドラはこれを資産として扱います。
別の場所に置くときは、最初に `initRuntime({ wasm: "/assets/ugoe_wasm_bg.wasm" })` を呼びます。
サーバーは `.wasm` を `application/wasm` で返してください。wasm は約 8MB (gzip で約 2MB) です。

## ローカルで使う (npm i 相当)

リポジトリのルートで:

```sh
cargo install wasm-bindgen-cli --version "$(just wasm-bindgen-version)" --locked  # 初回だけ
just web-runtime                     # wasm と TypeScript をビルドして bindings/web/dist/ に出す
cd bindings/web && pnpm pack --out ugoe-runtime.tgz
```

できた `ugoe-runtime.tgz` を `npm i ./ugoe-runtime.tgz` で入れます。

### 見本 (`examples/basic`)

```sh
cd bindings/web/examples/basic
npm install        # ../../ugoe-runtime.tgz を node_modules/@ugoe/runtime に入れる
npm start          # http://localhost:8080/ (?backend=webgl2 で WebGL2 に固定)
```

パッケージを作り直したら、`rm -rf node_modules package-lock.json && npm install` で入れ直します
(npm は同じ名前の tarball の中身の変化を見ないため)。

`<ugoe-avatar>` (属性・スライダーからの `setParameter`・表情とモーションの切り替え・`backend` の切り替え) と、
`createPuppet` で canvas に直接描く例を並べています。モデルはリポジトリの `assets/samples/` (MIT) を配ります。

## 開発

| コマンド | 内容 |
| --- | --- |
| `pnpm --filter @ugoe/runtime build` | = `just web-runtime`。wasm のビルド (`src/wasm/` に生成、gitignore 済み)・`tsc`・`dist/` への出力 |
| `pnpm --filter @ugoe/runtime check` | 型検査。`src/wasm/` が無ければ手書きの型 `stub/ugoe_wasm.d.ts` で検査する |
| `pnpm --filter @ugoe/runtime test` | 属性の解釈などの単体テスト (Vitest) |

- `crates/ugoe-wasm` の公開 API を変えたら、`stub/ugoe_wasm.d.ts` を生成物 (`src/wasm/ugoe_wasm.d.ts`) から作り直します。
- このパッケージは MIT です。`apps/`・`packages/` (GPL-3.0-only) のコードを import したり写したりしないでください ([ADR 0025](../../docs/adr/0025-web-runtime-package.md))。
- 構成: `src/index.ts` (入口)・`src/runtime.ts` (初期化・作成・読み込み)・`src/avatar.ts` (`<ugoe-avatar>`)・
  `src/element.ts` (登録するだけの入口)・`src/attributes.ts` (属性の解釈。DOM と wasm に触れない)。
