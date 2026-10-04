---
id: T0672
title: コードレビュー: エディタのフロントエンド (ビューポートとプレビュー)
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T0672
---

# T0672: コードレビュー: エディタのフロントエンド (ビューポートとプレビュー)

## 目的

エディタのフロントエンド (ビューポートとプレビュー) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/editor/src/lib/viewport/` と `apps/editor/src/lib/preview/` のすべて

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - WebGL2 の資源の解放・コンテキストの喪失、ポインタのキャプチャとイベントの解除
  - ギズモの座標変換 (高 DPI・拡大縮小)、ドラッグ中のコマンドの積み方
  - プレビューの wasm の資源の解放、毎フレームの確保
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0697〜T0699 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)
- `apps/editor/src/lib/viewport/` と `apps/editor/src/lib/preview/` をテストを含めてすべて読んだ (生成物の `generated/` は除く)。
- `shortcuts.ts` の `isInDialog` は T0670 のブランチと同じ内容をそのまま足した (同じ変更なので、どちらを先にマージしても衝突しない)。

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `viewport/controller.svelte.ts` | WebGL2 のコンテキストを失っても気づかない (wgpu-hal 30 の GL は `is_lost` で知らせず、描画も失敗しない)。ビューポートが真っ白のまま戻らない | 直した (canvas の `webglcontextlost` で作り直す。controller.test.ts)。wasm の側に寄せるのは T0697 |
| 中 | `viewport/controller.svelte.ts` `attach` | 作っている間に detach・次の attach があると、遅れて終わった WebPuppet が今のものを上書きし、今のものを解放しない (`generation` しか見ていない) | 直した (attach ごとの印。テストを追加) |
| 中 | `viewport/ToolOverlay.svelte`・`tools/form/FormTool.svelte`・`tools/deformer/DeformerGizmo.svelte`・`tools/glue/GlueOverlay.svelte` | window で受ける Esc・Delete・Ctrl (Cmd) + A が、ダイアログの中でも効く (後ろのメッシュの頂点を消す・選択を変える) | 直した (`tools/keys.ts` の `acceptsToolKey`。入力欄とダイアログの中では受けない。keys.test.ts) |
| 中 | `tools/physics/PhysicsOverlay.svelte`・`tools/lighting/LightingOverlay.svelte` | `pointercancel` を離したのと同じに扱い、衝突体の形・光源をプロジェクトに残す | 直した (取り消しは送らずに戻す) |
| 中 | `tools/deformer/DeformerGizmo.svelte` | 回転の取っ手を始点からの角度だけで測るので、1 回のドラッグで 180 度を超えると反対向きに折り返す | 直した (動くたびに足す `addTurn`。gizmo.test.ts) |
| 中 | `preview/tracking.test.ts` | `beforeAll` の import が @ugoe/face-tracking 経由で @ugoe/ui の部品をすべて変換し 7〜9 秒かかる。負荷が高いと 10 秒を超えて `just ci` が落ちる | 直した (`vi.mock("@ugoe/ui")`。単独で 8 秒 → 3 秒) |
| 中 | `tools/MeshEditTool.svelte` | 前の編集を送っている間にドラッグすると、離したときの `run` が捨てられ、ずらした表示が次に押すまで残る | 直した (送っている間は押下を受けない) |
| 中 | `tools/scene.svelte.ts` | `canvasPositions`・`hitAll` が呼ぶたびに `drawable_ids()` (全要素の文字列の配列) を作る。選択枠・モジュールの輪郭はメッシュごとに呼ぶので、物理のプレビュー中は毎フレーム O(メッシュ数²) の文字列を作る | 直した (印 (`#tick`) と WebPuppet ごとに使い回す。scene.test.ts) |
| 中 | ビューポートとプレビューの 10 か所ほど (`FormTool`・`DeformerGizmo`・`DepthOverlay`・`OverflowOverlay`・`GlueOverlay`・`LightingOverlay`・`PreviewPanel` など) | 編集のたびにモデル全体の JSON をそれぞれ `JSON.parse` する。`PreviewPanel` は try/catch も無い | T0698 を起票 |
| 中 | ツールのコンポーネント全般 | ポインタの操作から送るコマンドまでの流れにテストが無い (上の取り消し・回転・押下の誤りはどれもここ) | T0699 を起票 |
| 低 | `preview/differenceHighlight.ts:93`・`preview/DifferenceOverlay.svelte` | `Math.min(...xs)`・`push(...p)` が引数の数の上限 (JavaScriptCore は 65536) を超える頂点数で例外になる | 直した (1 つずつ見る。differenceHighlight.test.ts に 30 万点) |
| 低 | `viewport/Viewport.svelte` | devicePixelRatio だけが変わったとき (倍率の違う画面へ移した) は ResizeObserver が知らせず、描画バッファが古い倍率のままぼやける (ギズモの座標は同じ倍率で換算するので合っている) | 直した (`matchMedia` の `resolution` で聞く) |
| 低 | `tools/form/FormTool.svelte` | メッシュの辺を三角形ごとに 3 本描く (共有する辺が 2 重)。ワープの格子の辺は `gizmo.ts` の `gridEdges` と同じものを書き直している | 直した (`meshEdges`・`gridEdges` を使う) |
| 低 | `tools/physics/physicsGeometry.ts` | `toPoints` が `deformer/gizmo.ts` と重複。`segmentDistance` はテストからしか使われていない | 直した (gizmo のものを使い、使われていない関数を除く) |
| 低 | `preview/LicenseDialog.svelte` | 本文を読めなかったときの文言を覚えてしまい、開き直しても読み直さない | 直した |
| 低 | `preview/dressUp.ts` `MODULE_FILTER`・`viewport/screenshot.ts` `PNG_FILTER`・`preview/trackingHost.ts` | ファイルのダイアログのフィルタの名前が英語の直書き。T0670 が `filePaths.ts` のフィルタを翻訳した (`fileMenu.filters.module` を含む) | 見送り (T0670 と同じ箇所・カタログを触るので衝突する。T0670 のマージの後に `filePaths.ts` の `MODULE_FILTER` に寄せ、PNG・JSON を足す。下の「起票の候補」) |
| 低 | `viewport/controller.svelte.ts` `attach` | `just wasm` を流していないときの案内が英語の直書き | 見送り (開発中にしか出ない) |
| 低 | `preview/dressUp.svelte.ts` `load` | プロジェクトを開き直しても比べている途中の `#compareFiles` を捨てない (古い結果が `highlight` を書き換えうる) | 見送り (プレビューの外で開き直すので実害は見つからない。起票の候補) |
| 低 | `tools/overflow/OverflowOverlay.svelte` | 表示を切った後も、待っていた問い合わせを 1 回送る | 見送り (結果は捨てる) |
| 低 | `preview/dressUp.ts` `sameNames` | 片方に重複があると誤って同じとみなす (`["a","b"]` と `["a","a"]`) | 見送り (WebPuppet のモジュールの名前は重ならない) |
| 低 | `viewport/controller.svelte.ts` `capture` | 大きさを読んでから `read_pixels` を待つので、その間に大きさが変わると画素の数が合わずに失敗する | 見送り (失敗は知らせに出る。押し直せばよい) |

確かめた観点 (受け入れ条件の追加分):

- WebGL2 の資源の解放: WebPuppet は `detach` で `free`。作り直し (デバイス・コンテキストの喪失、WebGL2 へのフォールバック) は `generation` で canvas ごと替える。喪失は上の表のとおり直した。
- ポインタのキャプチャとイベントの解除: ツールは押した要素で `setPointerCapture` し、離す・取り消しで解く。window のリスナーは `svelte:window` なので外れる。DressUpSection の Tauri のドロップのリスナーは `onMount` の後片付けで外す (外す前に解けた場合も扱っている)。
- ギズモの座標変換: 公開する座標はすべて CSS ピクセルで、WebPuppet との間は `ViewportController` が devicePixelRatio で換算する (高 DPI)。拡大縮小は `transform.scale` で割ってキャンバス座標に直し、点は `ipc.canvasToLocal` で親の座標系に直す。
- ドラッグ中のコマンドの積み方: どのツールもドラッグ中はオーバーレイだけを動かし、離したときに 1 つのコマンド (Undo の 1 単位) を送る。
- プレビューの wasm の資源: モジュールのバイト列は `DressUp` が名前で持ち、外す・除くときに `unload_module`。トラッカーはプレビューを出るときに止める。毎フレームの確保は `afterUpdate` の `motion_events()` と `#sync` の `parameter(id)` (パラメータの数だけ) で、大きくはない。

### 横断

- **WebGL2 のコンテキストの喪失:** `is_lost` だけを見るホスト (packages/player・bindings/web の `<ugoe-avatar>`) は WebGL2 で同じく戻らないはず (T0697)。ビューア・スタジオも packages/player を使う。
- **window で受けるキー操作とダイアログ:** `svelte:window onkeydown` で受けるショートカットは、入力欄だけでなくダイアログ (`role="dialog"`・`alertdialog`) の中も除く必要がある (T0670 の Undo / Redo と同じ)。ほかのアプリ・パネルにもありそう。
- **`pointercancel` を離したのと同じに扱う:** ドラッグで確定するツールは、取り消しでは確定しない。パネルのドラッグ (タイムラインなど) にもありそう。
- **`Math.min(...配列)`・`push(...配列)`:** 引数の数の上限 (JavaScriptCore は 65536) を超える配列で例外になる。頂点・キーの配列を広げているところを探す。
- **モデルの JSON の解析の重複:** 部品ごとに `JSON.parse(doc.model.json)` する形がエディタ全体にある (T0698)。
- **重い import のテスト:** 共有の部品 (@ugoe/ui) を間接に読むテストは、変換に数秒かかり時間切れになりうる。描かない部品は `vi.mock` で空にする。

### 起票の候補 (枠が足りないもの)

- ファイルのダイアログのフィルタの翻訳 (T0670 のマージの後): `preview/dressUp.ts` の `MODULE_FILTER` を `filePaths.ts` のものに寄せ、`viewport/screenshot.ts` の `PNG_FILTER` と `preview/trackingHost.ts` の JSON のフィルタの名前をカタログに足す。
- `DressUp.load` で比べている途中の結果を捨てる (`#compareToken` を進める)。

### just ci

- `git rebase main` の後に `just ci` が通った (前面で実行、終了コード 0)。
