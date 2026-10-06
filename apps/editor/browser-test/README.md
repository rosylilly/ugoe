# エディタのブラウザのテスト (`just editor-browser`)

ビルドしたエディタの画面を、本物の wasm (`just wasm` の生成物) とヘッドレスの Chrome (WebGL2 は SwiftShader) で動かし、
偽の wasm の Vitest では確かめられない GPU の喪失のまわりを確かめる (T1656)。順序の細部は Vitest
(`src/lib/preview/dressUp.test.ts`・`src/lib/viewport/controller.test.ts`) が確かめるので、ここでは本物の `WebPuppet`
(`is_lost`・`load_module` の `device_lost`・作り直し) でも通ることを見る。

```sh
just editor-browser                                              # wasm・エディタ・見本をビルドしてすべてのシナリオ
just editor-browser --no-build --only dress-up-loss-during-load  # 前のビルドで、選んだシナリオだけ
just editor-browser --out DIR                                    # 結果の置き場 (既定 target/editor-browser)
```

要るもの: Google Chrome か Chromium (環境変数 `CHROME` で場所を指定できる)、`just wasm` に要るもの (wasm-bindgen-cli)、cargo
(見本を `ugoe sample dress-up` で作る)。`just ci` には含めない (ビルドが重く Chrome が要るため)。CI では `conformance` ジョブで
`just player-browser` の後に流す。

## 流れ

1. `just wasm` (エディタの `src/lib/viewport/generated/`)・`vite build` (`<out>/dist`)・`ugoe sample dress-up --project` (`<out>/sample`)。
2. dist を `just ui-audit` の静的サーバ (`tests/ui-audit/lib/serve.mjs`) で配り、Tauri の代わりに `just ui-audit` の偽のホスト
   (`tests/ui-audit/lib/editor-host.mjs`) を置く。偽のホストは見本のプロジェクトを開いたことにし、`read_module_file` で
   見本の `.ugoemod` を読む。ファイルのダイアログは `window.__UGOE_FILE_DIALOGS__` の差し替えで、次に選ぶパスを決める。
3. シナリオ (`scenarios.mjs`) を 1 つずつ新しいページで流し、画面を Playwright で操作する (ボタンの名前は `locales/en.json` から引く)。
   結果は `ok` / `FAIL` で出し、ページのコンソールは `<out>/page.log`、失敗した画面は `<out>/<シナリオ>-FAILED.png` に残す。

本物の `WebPuppet` は、シナリオがページで wasm-bindgen のグルー (Vite が別のチャンクにした `assets/ugoe_wasm-*.js`) を
エディタと同じ URL で読み (同じモジュールになる)、`WebPuppet` を包んで見る: 描いている WebPuppet (`render` を最後に呼んだもの。
付いているモジュールと `read_pixels` の読み戻しを確かめる)・生きている WebPuppet (`free` していないもの)・作成の遅れ。
WebGL2 のコンテキストは、ページの初期化で `getContext` を包んで canvas ごとに覚え、`WEBGL_lose_context` で失わせて戻す。

## シナリオ

| 名前 | 確かめること |
| --- | --- |
| `dress-up-loss-during-load` | プレビューの着せ替えで、ビューポートの WebGL2 のコンテキストを失っている間 (ビューポートが「GPU の復元を待っています」) に「Load .ugoemod…」で猫耳 (`cat-ears.ugoemod`) を読み込む (`load_module` が `device_lost`)。パネルに失敗の文が出ず、復元した後に作り直した WebPuppet に猫耳が付き (一覧の行のチェックと `attached_modules`)、描かれる (読み戻しで、描いた画素の一番上の行が猫耳のぶん上がる) |
| `dress-up-change-while-lost` | 猫耳・長い髪・ツインテールを読み込み (猫耳とツインテールが付く) 、失っている間に猫耳のチェックを外し、スロット `Hair` の選択で長い髪に替える。パネルに失敗の文が出ず、作り直した WebPuppet には長い髪だけが付き、一覧の行もそのとおりで、猫耳が描かれない |

どちらも、作り直しの完了 (新しい WebPuppet が描き、ビューポートの「GPU の復元を待っています」が消える) を待ってから確かめる。
作り直しの `WebPuppet.create` を 500 ms 遅らせて、CI の SwiftShader のように作成が遅いときに、作り終える前に確かめないことも兼ねる
(Player の `player-loss-during-load` と同じ。T0960)。3000 ms 遅らせても通ることと、5 回続けて流して揺れないことを確かめた (T1656)。
T1644 の前の `dressUp.svelte.ts` では 2 つとも落ち (失っている間にパネルに「The GPU stopped working.」が出る)、作り直した後に取っておいた
モジュールを読み込まない (`#loadPending` を呼ばない) と `dress-up-loss-during-load` が落ちることも確かめた。

## 選んだ仕組み

実機の e2e (`just e2e`。本物の Tauri と WebKitGTK) ではなく、ここ (ヘッドレスの Chrome と偽のホスト) にした (T1656)。

- 喪失と復元を決まった時機で起こせる: Chrome の `WEBGL_lose_context` は `restoreContext` まで含めて確かに動き、`player-browser`
  (`packages/player/browser-test`) で同じ起こし方を使っている。WebKitGTK の WebView で同じように起こせるかは確かめていない。
- 本物の WebPuppet を直接見られる: ページから wasm のグルーを読んで包めるので、付いているモジュールと読み戻しの画素で確かめられる
  (e2e は WebDriver の外から画面を見るだけ)。
- 速く、何度も流せる: e2e はエディタの筋書き全体 (MediaPipe・AI の提案など) を 1 つの起動で順に流し、1 つの手順だけを選べない。
  CI でも、e2e は Tauri のビルドが要る別のワークフローで、ここは `player-browser` と同じジョブで流せる。

Tauri と WebView の間の不具合 (IPC・ファイルのダイアログ) はここでは見ない (偽のホスト)。それは e2e の着せ替えの手順が見る。

## シナリオを足す

- `scenarios.mjs` の `SCENARIOS` に `async "名前"(page, ctx)` を足す。`ctx` は `url`・`wasm` (グルーの URL)・`sample`・`modules`
  (見本の `.ugoemod` の置き場)。見本のプロジェクトを開くのは `openSample` (wasm を包む `hookWasm` も呼ぶ)。
- 待つときは決まった時間眠らず、`until` (ページの中の条件) か Playwright の `waitFor` で待つ。作り直しを待つときは、canvas が
  替わったことではなく、新しい WebPuppet が描いた (`restoreAndRecreate`) ことを待つ。
- 偽のホストに無いコマンドが要れば `tests/ui-audit/lib/editor-host.mjs` に足す (`just ui-audit` と共有)。
