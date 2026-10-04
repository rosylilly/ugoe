---
id: T0452
title: "@ugoe/runtime: ほかの *_json() の型と、警告・エラーのコードの一覧の突き合わせ"
status: done
milestone: M9
crates: [bindings/web, ugoe-wasm, docs]
depends_on: [T0434]
assignee: claude-worker-T0452
---

# T0452: @ugoe/runtime: ほかの *_json() の型と、警告・エラーのコードの一覧の突き合わせ

## 目的

T0438 で `<ugoe-avatar>` のイベントと警告を型付きにした (`bindings/web/src/warnings.ts` の和型) が、次が残った。

- `modules_json()`・`slots_json()`・`expressions_json()` など、ほかの `*_json()` の戻り値には TS の型と読む補助が無い (利用者が `JSON.parse` して形を推測する)。
- Rust の `LoadWarning::code` / `DressUpError::code` の一覧と `warnings.ts` の和型が食い違っても、機械では気づけない。
  wasm のテストか stub の検査で、知っている `code` の一覧を出して比べる。T0434 で足す読み込みのエラーのコードも同じ検査に入れる。

## 受け入れ条件

- [x] `*_json()` の戻り値ごとに TS の型と読む補助を足し、テストがある (足さないものは理由を README に書いた)
- [x] Rust の `code` の一覧 (`LoadWarning`・`DressUpError`・T0434 の読み込みのエラー) と TS の和型を比べる検査があり、`just ci` で流れる。片方だけに足すと失敗することを確かめた
- [x] `bindings/web/README.md` を更新した
- [x] `just ci` と `just wasm-check` が通る

## 参考

- T0438 のメモ (起票の候補)
- bindings/web/src/warnings.ts、crates/ugoe/src/warning.rs・dress_up.rs、T0435 (手書きの型の写しと生成物の比較)

## メモ

- T0019 で起票した (元の候補: T0438 の 2 つ)。読み込みのエラーのコード (T0434) も同じ検査に入れるので、その後に着手する。

### やったこと

- **`code` の突き合わせ:** 一覧を `crates/ugoe-wasm/codes.json` に置いた (`load_warning`・`dress_up_error`・`load_error` ごとに、`code` → 補足の値のキー)。
  - Rust (`crates/ugoe-wasm/src/codes.rs`、テストだけのモジュール): `LoadWarning`・`WardrobeError` (その中の `DressUpError`・`ComposeError`・
    `CompileError`)・`LoadError` のすべての種類の見本を `warning_json`・`Detached::to_json`・`dress_up::error_info`・`LoadError::info` に通して一覧を作り、
    `codes.json` と比べる。種類を足すと網羅の `match` (`guard`) がコンパイルできなくなる。`UGOE_UPDATE_CODES=1 cargo test -p ugoe-wasm codes` で書き直せる。
    `FormatError`・`RenderError` は網羅しない (ほかは `invalid_file`・`texture` にまとまるため。README に書いた)。
  - TS (`bindings/web/src/codes.test.ts`): `codes.json` を import し (tsconfig に `resolveJsonModule`)、`code` の配列と、補足の値のキーを比べる。
    キーの配列は `expectTypeOf` で `UgoeDressUpErrorFields`・`UgoeLoadErrorFields`・`UgoeKnownLoadWarning` の各型のキーと同じであることを型で確かめる
    (`pnpm check` の tsc が見る)。
  - 片方だけに足すと落ちることを確かめた: TS の `DRESS_UP_ERROR_CODES` に `future_code` を足す → `codes.test.ts` が落ちる。
    Rust の `no_model` を `no_model_v2` に変える → `codes_json_lists_every_code_and_field` が落ち、`codes.json` を書き直すと今度は `codes.test.ts` が落ちる (どちらも戻した)。
- **`code` の配列を公開した:** `LOAD_WARNING_CODES`・`DRESS_UP_ERROR_CODES`・`LOAD_ERROR_CODES` (`as const`)。`UgoeDressUpErrorCode`・`UgoeLoadErrorCode` は
  この配列の要素の型にした (型と配列がずれないように)。比べるには実行時の値が要るので足した (訳の表の網羅のテストにも使える)。
- **T0157 の補足の値:** `UgoeDressUpErrorFields` に `mask`・`rig`・`collider`・`part` (文字列)、`texture`・`required`・`actual`・`errors` (数) を足し、
  `load_module` の上限の値のために `UgoeLimitErrorFields` を継承した。README の例の一覧も直した。
- **`*_json()` の型と補助 (`bindings/web/src/queries.ts`):** `parameterDefinitions`・`motionInfos`・`expressionInfos`・`moduleInfos`・`slotDefinitions`・
  `avatarSlots`・`drawableStates`・`currentPose` と型 (`UgoeParameterDefinitions`・`UgoeMotionInfo`・`UgoeExpressionInfo`・`UgoeModuleInfo`・`UgoeSlot`・
  `UgoeDrawableState`・`UgoePose` など)。`sample_motion_json` は `sampleMotion` (`runtime.ts`。wasm の値を import するので、`<ugoe-avatar>` のテストが
  `runtime.js` を差し替える都合でそちらに置いた) と `UgoeCurveValue`。`UgoeParameterDefinition` は `avatar.ts` から `queries.ts` に移し、
  `kind`・`mirror`・`group` を足した (入口からの名前は変えていない)。テストは `queries.test.ts` (偽の `WebPuppet` と、wasm の関数の差し替え)。
- 足さなかったもの (理由は bindings/web/README.md#問い合わせの-json-の型-t0452 の表): `physics_json`・`base_physics_json` (`*.physics.json` の大きな形で、
  主な使い道は `set_physics_json` に戻すこと。JSON Schema がある)、`MotionSampler.sample` (形は同じ。`MotionSampler` は入口から出していない)。
- 文書: bindings/web/README.md (問い合わせの JSON の型の節・`code` の配列・補足の値・開発の「`code` の突き合わせ」)、crates/ugoe-wasm/README.md (「`code` の一覧」の節)。
- `just ci` と `just wasm-check` は通った (2026-10-04、main の f64571a2 の上)。

### やらなかったこと

- 偽の `WebPuppet` で形を確かめただけで、本物の wasm の `*_json()` の出力と TS の型は比べていない (形の一覧は Rust の doc と README から写した)。
- `packages/player/src/loadError.ts` などの訳の表が全種類を持つかの検査 (範囲外)。

### 起票の候補

- `packages/player` (`loadError.ts`・`loadWarnings.ts`) とエディタのプレビューの訳の表を、`@ugoe/runtime` の `LOAD_ERROR_CODES`・`LOAD_WARNING_CODES`・
  `DRESS_UP_ERROR_CODES` (か `codes.json`) と比べるテスト。
- 本物の wasm (適合テストの `wasm`) で `*_json()` の出力のキーを `queries.ts` の型と比べる検査 (今は Rust の doc と手で合わせている)。
- bindings/unity の `UgoeException.FieldKeys` を `codes.json` のキーと比べる (T0157 の候補と同じ所)。

