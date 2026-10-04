---
id: T0463
title: "@ugoe/runtime: MotionSampler を出すかと、本物の wasm で *_json() の出力のキーを型と比べる"
status: done
milestone: M9
crates: [bindings/web, tests/conformance, docs]
depends_on: []
assignee: claude-worker-T0463
---

# T0463: @ugoe/runtime: MotionSampler を出すかと、本物の wasm で *_json() の出力のキーを型と比べる

## 目的

- T0451 で wasm に `MotionSampler` を足したが、`bindings/web/src/index.ts` からは出していない (`sample_motion_json` だけ。T0451 の候補)。
- T0452 で `*_json()` の戻り値の TS の型 (`queries.ts`) を足したが、型と実際のキーは Rust の doc と手で合わせている。本物の wasm
  (適合テストの `wasm`) で出力のキーを型と比べる検査が無い (T0452 の候補)。

## 受け入れ条件

- [x] `MotionSampler` を `@ugoe/runtime` から出すかを決め、出すなら型と README の使い方とテストがある (出さないなら理由を README に)
- [x] 適合テスト (または bindings/web のテスト) が、本物の wasm の `*_json()` の出力のキーを `queries.ts` の型のキーと比べ、食い違いで落ちる
- [x] `just web-check`・`just conformance`・`just ci` が通る

## 参考

- T0451・T0452 のメモ (起票の候補)、T0435 (手書きの型の写しと生成物の比較)

## メモ

- T0026 で起票した (元の候補: T0451 の `MotionSampler` の公開、T0452 の `*_json()` のキー)。

### やったこと

- **`MotionSampler` を出した (判断と理由):** 案は (A) wasm のクラスをそのまま出し `sampleMotion` で読む、(B) TS で包んだクラス
  (`UgoeMotionSampler`) を出す、(C) 出さない (`@ugoe/runtime/wasm` から使う)。**(A) にした。** `WebPuppet` と同じく wasm のクラスは
  そのまま出し、JSON を読む補助は関数で足すというこのパッケージの形に合い、増える公開の名前が 1 つで、`free()` の扱いも `WebPuppet` と同じに
  なるため。(B) は包みの分だけ `free` や `set_motion_json` を写す必要があり、(C) はタイムラインのような主な使い道で `sampleMotion` が毎回 JSON を
  読み直すのを避けられない。
  - `index.ts` から `MotionSampler` を出し、`sampleMotion(motion: string | MotionSampler, t)` にした (文字列なら今までどおり
    `sample_motion_json`。互換は保つ)。テストは `queries.test.ts` の `parses MotionSampler.sample`。
  - README: 「モーションを何度も評価する (`MotionSampler`、T0451・T0463)」の節 (初期化・`set_motion_json`・`free`)、問い合わせの表の行を直した。
- **キーの突き合わせ:**
  - `bindings/web/src/query-keys.ts`: `queries.ts` の型ごとの必須・省略できるキーの一覧 `QUERY_KEYS` (16 の形。入れ子の形も) と、
    出力を一覧と比べる `QueryKeyCheck` (無い必須のキー・一覧に無いキーを道筋付きで報告し、形ごとに比べた数を数える)。入口からは出さない
    (`dist/query-keys.js` はできるが `exports` に無い)。
  - `query-keys.test.ts`: 一覧と型 (`[key: string]: unknown` を除いた名前のキーを必須・省略できるものに分けて) が同じことを `expectTypeOf` で
    確かめる (`pnpm check`)。一覧から `kind` を消す・`curve_target` に `x` を足すと `pnpm check` が落ちることを確かめた (戻した)。
    偽の `WebPuppet` で `QueryKeyCheck` が食い違いと比べていない形を報告することも確かめる。
  - 適合テストのバインディング `queries` (`tests/conformance/web/queries.html`・`queries.js`、`run.mjs`): 本物の wasm を
    ヘッドレスの Chrome の WebGL2 で動かし、モデル無し・`sample_motion_json` と `MotionSampler`・フィクスチャ `pose_and_effects`
    (`ugoe fixture` で書き出しグループを足す。見本はポーズとグループを持たないため)・`avatar`・`avatar-lit`・`dress-up` (3 つ付け、1 つ読むだけ)
    の出力を比べ、1 度も比べなかった形があれば `coverage` で落ちる。`dist/query-keys.js` の一覧を壊すと (`kind` を消す・`pose` に必須の
    `speed` を足す) 落ちることを確かめた (戻した)。本物の出力と型の食い違いは見つからなかった。
  - `just conformance` (`--only` なし) に含まれるので、CI の `conformance` ジョブで動く (`ci.yml` は変えていない)。`--only queries` だけなら
    参照は作らず、フィクスチャを書き出す `ugoe-cli` だけをビルドする。
- 文書: bindings/web/README.md (開発の「問い合わせの JSON のキーの突き合わせ」・構成)、tests/conformance/README.md (表・実行・出力・
  「問い合わせの JSON のキー」の節)、docs/testing.md (適合テストの説明)。
- `crates:` の外の小さな変更: `justfile` の `conformance` のコメント (`queries` を足した)。
- 確かめたこと (WSL2、Chrome): `just conformance` (c・wasm・wasm-webgpu・avatar・queries・unity・godot がすべて ok)、`just web-check`、
  `just ci` (main の e84654f6 の上で通った)。

### やらなかったこと

- 読み込みの警告 (`load_warnings_json`・`detached_modules_json`・`warning_details`) のキーは `codes.json` で突き合わせ済みなので比べていない。
  `physics_json` は型を出していないので比べていない。
- 値の型 (数か文字列か) までは比べていない (キーの有無だけ)。
- `crates/ugoe-wasm/README.md` には `MotionSampler` の節が無い (範囲外。下の候補)。

### 起票の候補

- crates/ugoe-wasm/README.md に `MotionSampler` (`new`・`set_motion_json`・`sample`) を書く (T0451 で足したが README に無い)。
- 見本のモデル (`assets/samples`) にパラメータのグループとポーズを持たせるか検討する (今は `queries` がフィクスチャに足して確かめている)。
