---
id: T0125
title: wasm: 読み込みのほかの警告 (物理を外した・テクスチャを代用した) も構造化されたデータで返す
status: done
milestone: M4
crates: [ugoe-wasm, apps/viewer, apps/editor, docs]
depends_on: [T0124]
assignee: claude-worker-T0125
---

# T0125: wasm: 読み込みのほかの警告も構造化されたデータで返す

## 目的

T0124 の後続の候補。付け直しで外れたモジュールは構造化されたデータ (`detached_modules_json`) で返すようになったが、読み込みのほかの警告
(物理の設定を外した・テクスチャを代用したなど、`crates/ugoe-wasm/src/load.rs`・`animation.rs`) は英語の文のままで、ビューアとエディタのプレビューでは英語で出る。

## 受け入れ条件

- [x] 読み込みの警告を、種類 (`code`) と補足の値を持つ構造化されたデータで返す。T0124 の `detached_modules_json` と形を揃えるか、1 つの一覧にまとめるかを決めて README に書く。今の文の警告の扱いは T0124 と同じ方針にする
- [x] ビューアとエディタのプレビューが、構造化されたデータから日本語 (ビューア) / 英語 (エディタ) の知らせを作る
- [x] Rust のテストと Vitest。`just wasm-check`
- [x] `just ci`・ビューアとエディタのビルドが通る

## 参考

- `docs/tasks/T0124-wasm-dress-up-structured-warnings.md` のメモ、`crates/ugoe-wasm/README.md`

## メモ

- **wasm:** `WebPuppet.load_warnings_json()` を足した。直前の `load_json` / `load_bundle` の警告を、戻り値の文の警告と
  **同じ順・同じ数の 1 つの一覧** で返す (`[{code, warning, ...}]`)。`code` は `texture_ktx2`・`texture_missing`・
  `normal_map_ktx2` (補足 `texture`)、`physics_removed` (`reason`: 英語の理由)、`module_detached` (`detached_modules_json()` の
  要素と同じ項目)。Rust では `load::LoadWarning` (enum。`Display` が今までの文、`code`・`to_json`) を `LoadedModel.warnings`・
  `LoadedModule.warnings`・`animation::carry_over` の戻り値に使い、`load::warnings_json` が外したモジュールと合わせて並べる。
- **決めたこと (README の「読み込みの警告 (T0125)」):** 別の口を種類ごとに作らず 1 つの一覧にまとめた (戻り値と 1 対 1 なので、
  言い換える側はこれだけ読めばよい)。T0124 の `detached_modules_json()` は残し、要素に `code: "module_detached"` を足しただけ。
  **文の警告は T0124 と同じく残した** (ログに出すだけの組み込み先のため。文面は安定した値として扱わない)。
  `.ugoemod` の警告 (テクスチャの代用) は `modules_json()` の要素に `warning_details` (同じ順の構造) を足した。
- **ビューア:** `lib/loadWarnings.ts` (読み取り・テクスチャと物理の日本語・`replaceWarnings`)。`dressUp.ts` の
  `describeLoadWarnings(warnings, structured)` が `module_detached` を T0124 と同じ文にし、ほかは `loadWarnings.ts` で日本語にする
  (順は戻り値のまま。構造の無い文と知らない `code` は英語のまま)。`parseModules` もモジュールの `warning_details` を日本語にする
  (パネルの「警告 N」のツールチップ)。`parseDetachedModules` は使わなくなったので消した (`stub.d.ts` も `load_warnings_json` に)。
  `player.svelte.ts` は 1 行と import だけ (T0709 の import と rebase で衝突したので合わせた)。
- **エディタのプレビュー:** `preview/loadWarnings.ts` (英語)。`PreviewMode.loaded` が `warnings` を読み込むたびに置き換え、
  `PreviewPanel.svelte` の上に枠付きで出す。`module_detached` は DressUp の知らせに出るので除き、`physics_removed` は
  ドキュメントの物理を渡し直せたときは除く (編集のたびの読み直しで外れるのは古い設定のため)。DressUp (`dressUp.svelte.ts`) と
  モジュールの警告の文は変えていない (T0651 との衝突を避けた。英語の文のまま)。プレビューの外はコンソールに出すだけのまま。
- **テスト:** Rust は `load` に 2 件 (KTX2 のテクスチャと法線マップ・画像が無い・物理を外した。文面が前と同じことも)、
  `carry_over` と着せ替えの読み直しの件に構造の検査。Vitest はビューアの `loadWarnings.test.ts` (4 件)・`dressUp.test.ts`
  (`describeLoadWarnings` とモジュールの `warning_details`)、エディタの `preview/loadWarnings.test.ts` (3 件) と
  `tracking.test.ts` に `PreviewMode` が警告を出す件。
- **確かめたこと:** `just ci`・`just wasm-check`・ビューアとエディタの `vite build`。ヘッドレスの Chrome (SwiftShader) でビルドした
  ビューアに、テクスチャを KTX2 にしたバンドルを開かせ「テクスチャ 0 は KTX2 の画像で、まだ表示できないため白で代わりに描いています」、
  同じ場所で LongHair と Dress を付けてから、スロット Outfit の無い KTX2 の版に変えて開き直し、その文と
  「このモデルに合わないため、着せ替えの「Dress」を外しました (…)」が並ぶことを確かめた。
  スクリーンショット: scratchpad `T0125/shots/ktx2-texture.png`・`T0125/shots/reopen-detached.png`。
- **確かめていないこと:** エディタのプレビューの画面 (Vitest だけ)。`physics_removed` のビューアの画面 (Vitest だけ)。Tauri のウィンドウ。
- **後続の候補:** エディタのプレビューの着せ替えの一覧の、モジュールの警告 (`entry.warnings`) も `warning_details` から英語にすること
  (今は wasm の英語の文)。
