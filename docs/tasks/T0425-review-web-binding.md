---
id: T0425
title: コードレビュー: bindings/web (@ugoe/runtime と <ugoe-avatar>)
status: done
milestone: M9
crates: [bindings/web, docs]
depends_on: []
assignee: claude-worker-T0425
---

# T0425: コードレビュー: bindings/web (@ugoe/runtime と <ugoe-avatar>)

## 目的

bindings/web (@ugoe/runtime と <ugoe-avatar>) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `bindings/web` のすべて (src・stub・scripts・examples)

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - `ugoe-wasm` の API との対応、TypeScript の型・エラーの形 (T0124・T0125) のずれ
  - `<ugoe-avatar>` の属性とイベント・資源の解放 (切り離し・再接続)・WebGL のコンテキストの喪失
  - MIT のパッケージに GPL のコード (packages/・apps/) が入っていないか、npm に出すファイルの範囲とサイズ (T0413)
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0438〜T0442 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)。
  使ったのは T0438〜T0440。T0441・T0442 は使っていない。

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 高 | `src/avatar.ts` (`#newCanvas`・`#schedule`) | WebGL2 のコンテキストを失っても気づかず、真っ白のまま戻らない (wgpu-hal の GL は喪失を `is_lost` で知らせない。エディタの T0672 と同じ) | 直した: canvas の `webglcontextlost` で `preventDefault` して描くのを止め、`webglcontextrestored` で新しい canvas に作り直す (モデルとパラメータの値は戻す)。テストを追加。ugoe-wasm 側で知らせる口は T0697 |
| 中 | `src/avatar.ts` (`#stop`) | 切断で取得中の `src` を捨てていた。新しい `src` の取得中に要素を動かす (外して戻す)・`backend` を変える・GPU を失うと、前のモデルを読み直したまま新しい `src` を読まない | 直した (取得は切断しても続け、繋ぎ直したら取得の結果を読む)。テストを追加 |
| 中 | `src/avatar.ts` (`load`・`#fetchSrc`) | 読み込めなかったモデルも「最後のモデル」として覚え、作り直したとき (`backend` の変更・GPU の喪失) に読み直して `error` になり、前に読めたモデルを失う | 直した (読み込めたものだけを覚える)。テストを追加 |
| 中 | `src/avatar.ts` (`load`) | `WebPuppet` の作成前に呼んだ `load()` が、作成に失敗すると永遠に待ち、モデルの読み込みに失敗しても前の警告で resolve していた (失敗が分からない) | 直した (その読み込みの結果で resolve / reject。作成前に `src` を変えたら `AbortError`)。テストを追加 |
| 中 | `src/avatar.ts` (`#restart`) | 作り直しの回数が戻らず、長く開いておくページ (配信のオーバーレイ・スリープからの復帰でコンテキストを失う) では 3 回失うと動かなくなる | 直した (前の作り直しから 1 分を超えて失ったら数え直す)。テストを追加 |
| 中 | `stub/ugoe_wasm.d.ts` | `avatar_slots_json` (T0715) が無い。スタブと ugoe-wasm のずれを見つける検査が無い | 直した (`stub/ugoe_wasm.test.mjs` でメソッドと関数の名前をクレートのソースと比べる) |
| 中 | `src/avatar.ts` (型) | `load` / `error` の `CustomEvent` がイベントの型に無く、TypeScript では `e.detail` が型の誤りになる。読み込みの警告の構造 (T0125) と `DressUpError` (T0124) の型が無い | T0438 を起票 |
| 中 | `src/avatar.ts` (`ready`・諦めた後) | `ready` は作成の失敗で解決も reject もしない。作成の失敗や GPU を続けて失って諦めた後に、やり直す口が無い (諦めた後は `backend` の変更も効かない) | T0439 を起票 (公開の挙動を変えるため) |
| 中 | テスト | `<ugoe-avatar>` のテストが無かった (属性の解釈とトラッキングの変換だけ)。適合テストも `WebPuppet` だけを使う | happy-dom と `WebPuppet` の偽物で作成・切り離し・作り直し・読み込みの順序のテストを足した (`src/avatar.test.ts`)。本物のブラウザと wasm は T0440 を起票 |
| 低 | `src/avatar.ts` (`defineUgoeAvatar`) | `customElements` の無い環境 (SSR) で `@ugoe/runtime/element` を import すると `ReferenceError` で落ちる (「SSR で落ちない」というコメントと食い違う) | 直した (何もしない)。テストを追加 (`src/element.test.ts`) |
| 低 | `src/tracking-filter.ts` | 設定に `{ beta: undefined }` を渡すと、`{ ...defaults, ...options }` で既定値が消えて NaN になる | 直した。テストを追加 |
| 低 | `examples/basic/serve.mjs` | 壊れた `%` の並びの URL (`/%E0%A4%A`) で `decodeURIComponent` が投げ、見本のサーバーが落ちる | 直した (404 を返す)。見本のスクリプトにはテストの仕組みが無いので、`curl` で手で確かめた |
| 低 | `src/avatar.ts` (`#fetchSrc`) | 置き換わった取得を中断しない (結果は捨てるのでダウンロードだけが続く)。資格情報 (`crossorigin`) を選べない | T0439 に含めた |
| 低 | `examples/basic/index.html` (`startPlain`) | `WebPuppet` を直接使う例が `is_lost` を見ない | 見送り (見本の簡潔さを優先。作り直しは `<ugoe-avatar>` と README の「WebGPU と WebGL2」に書いてある) |
| — | ugoe-wasm の API との対応 | 型はスタブ以外は生成物そのもの。`trackingArguments` の並びは `set_tracking` と一致。`UgoeParameterDefinition` は `parameters_json` の要素と一致 (`kind`・`mirror`・`group` は索引の型で受ける) | 問題なし (スタブのずれだけ上で直した) |
| — | ライセンス・npm に出す範囲 | GPL (`packages/`・`apps/`) の import や写しは無い (`tracking-filter.ts` は論文から書いたと明記)。`npm pack --dry-run` (ビューアの release の wasm を借りて `--no-wasm` でビルド) で 21 ファイル: `dist/` の JS と型・`dist/wasm/`・README・LICENSE・package.json。テスト・スタブ・見本は入らない。`web-dist` の wasm の大きさは README の表 (約 4.5MB、gzip 約 1.3MB) のとおりで、今回は測り直していない (重いビルドが並行のレビューと重なるため) | 問題なし |
| — | 切り離し・再接続の競合 (T0672 と同じ形) | 作成中の切断・再接続・`backend` の変更は `#session` で古い `WebPuppet` を捨てており、今のものを上書きしない。取得は `#fetch` の番号で古い結果を捨てる | 問題なし (上の取得の扱いだけ直した) |

- 依存: テストのために `happy-dom` (MIT。`packages/ui` と同じ版で、ワークスペースには既にある) を `bindings/web` の
  devDependencies に足した。そのため範囲の外の `pnpm-lock.yaml` も変わっている (依存の記録だけ)。
- コンテキストの喪失は `webglcontextrestored` を待ってから作り直す (すぐ作り直すと、GPU が戻る前に作成に失敗したり、
  コンテキストの数の上限で別の canvas のコンテキストを奪い合ったりするため)。ブラウザが復元しなければ、これまでどおり描かない。

### 横断

- **WebGL2 のコンテキストの喪失:** wgpu-hal の GL は喪失を `is_lost` で知らせない。`WebPuppet` を使うところ (エディタは T0672、
  ビューア・スタジオの出力・`packages/player`) はどれも `webglcontextlost` / `webglcontextrestored` を見る必要がある (T0697 で ugoe-wasm 側の口)。
- **作り直しのときに読み直すデータ:** 読み込めなかったデータを「最後のモデル」として覚えていると、作り直しで前のモデルを失う。
  読み直しのために覚えるものは、成功した後に更新する。
- **作成前の呼び出しを待たせる Promise:** 作成の失敗で settle しない Promise (`ready`・`load`) は、呼び出し側を永遠に待たせる。
- **手書きの型のスタブ:** 生成物の代わりのスタブは、名前だけでも元と比べるテストが無いとずれる。
- **`{ ...defaults, ...options }`:** 利用者の設定に `undefined` があると既定値が消える。公開の設定では `??` で埋める。
- **カスタム要素の `CustomEvent`:** イベントの型 (`HTMLElementEventMap` の拡張) が無いと、`error` などの既存の名前は TypeScript で別の型になる。
