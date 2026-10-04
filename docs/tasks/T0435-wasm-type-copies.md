---
id: T0435
title: ugoe-wasm の手書きの型の写しを生成物と比べる
status: done
milestone: M9
crates: [bindings/web, packages, apps/editor, apps/viewer, apps/studio, scripts, docs]
depends_on: [T0424]
assignee: claude-worker-T0435
---

# T0435: ugoe-wasm の手書きの型の写しを生成物と比べる

## 目的

wasm-bindgen の生成物が無い環境で型検査を通すため、`WebPuppet` の型の手書きの写しが 5 か所にある:
`bindings/web/stub/ugoe_wasm.d.ts`・`apps/viewer/src/lib/wasm/stub.d.ts`・`apps/editor/src/lib/viewport/ugoe-wasm-stub.d.ts`・
`apps/studio/src/lib/wasm/stub.d.ts`・`packages/player/src/wasm.ts`。すでにずれている (T0424 の時点で `bindings/web` の写しに
`avatar_slots_json` が無い)。公開 API を変えるたびに手で直すのは漏れやすいので、生成物と写しを比べる検査を用意するか、写しを 1 つにまとめる。
T0424 (ugoe-wasm のレビュー) で見つけた。

## 受け入れ条件

- [x] 写しを減らす (共有できるものは 1 つの写しから使う) か、残す理由を書く
- [x] `just wasm` で作った `ugoe_wasm.d.ts` と写しを比べ、メソッドの抜け・型の違いを報告するスクリプト (CI の wasm を作るジョブで走らせる)
- [x] `bindings/web/stub/ugoe_wasm.d.ts` に `avatar_slots_json(): string` を足す
- [x] `just ci` が通る

## 参考

- `bindings/web/scripts/check.mjs` (T0003 は写しを写し直す仕組みを直した。写しと生成物のずれは扱っていない)
- `crates/ugoe-wasm/README.md#ビルド`

## メモ

- **写しを 5 つから 2 種類に減らした。** アプリ (エディタ・ビューア・スタジオ) の手書きの型 (`ugoe-wasm-stub.d.ts`・`wasm/stub.d.ts`) を消し、
  `tsconfig.json` の `paths` の 2 番目 (生成物が無いとき) を `bindings/web/stub/ugoe_wasm.d.ts` に向けた。GPL のアプリが MIT のスタブの
  型の宣言を読むだけで、`bindings/web` から GPL のコードは読まない。生成物が無い手元で 3 つのアプリの `check` が通ることと、スタブのメソッドの
  名前を 1 つ変えるとビューアとエディタの `check` が落ちること (スタブが本当に使われていること) を確かめた。
- **残した写しと理由:**
  - `bindings/web/stub/ugoe_wasm.d.ts` (全体の写し): `@ugoe/runtime` は MIT で生成物が無い環境でも `check` を通す必要があり、アプリの
    生成物の無いときの型もここから取る。
  - `packages/player/src/wasm.ts` の `WebPuppet` と `packages/face-tracking/src/frame.ts` の `TrackingPuppet` (一部の写し): パッケージは
    wasm の生成物を持たず (読み込みはアプリが決める)、使うメソッドだけを構造で書く口なので、スタブを import する形にはしなかった。
- **検査:** `scripts/check-wasm-types.mjs` (`just wasm-types [<生成物>]`、テスト `scripts/check-wasm-types.test.mjs`)。TypeScript の
  構文木 (bindings/web の依存の typescript) で読み、書き方・union の並び・引数の名前によらずに比べる。
  - 全体の写し: クラス・メンバー (static を含む)・関数・型の別名の過不足と型の違い。`InitOutput` は wasm の内部の口なので写しにあるものだけを見る。
  - 一部の写し: メンバーが生成物にあり型が同じか。末尾の省略できる引数は省いてよい。`?` のメンバー (`follow?` など) は生成物に無くてよい。
  - CI の web ジョブで `just wasm` の直後に生成物と比べる。`just ci` の `web-check` は引数なしで走らせ、生成物があれば生成物と、無ければ一部の写しを
    スタブと比べる。手元で `just wasm` の生成物と比べ、違いが無いことを確かめた (wasm-bindgen 0.2.129)。
- `avatar_slots_json(): string` は T0425 がすでにスタブに足していた (このタスクでは変えていない)。
- 範囲外の小さな変更: `justfile` (`wasm-types` と `web-check` の依存)・`.github/workflows/ci.yml` (web ジョブの 1 段)。
- やらなかったこと: player の `UgoeWasmModule.WebPuppet.create` (プロパティの中の関数) は比べていない (アプリの型検査で確かめられる)。
- 起票の候補: `just script-check` (scripts/ の単体テスト) が CI のどのジョブにも入っていない (`.github/workflows/ci.yml` に無い)。

