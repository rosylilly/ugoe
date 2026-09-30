---
id: T0652
title: エディタ: プレビューの着せ替えの一覧のモジュールの警告を英語の知らせにする
status: done
milestone: M4
crates: [apps/editor, docs]
depends_on: [T0125]
assignee: claude-worker-T0652
---

# T0652: エディタ: プレビューの着せ替えの一覧のモジュールの警告

## 目的

T0125 の後続の候補。エディタのプレビューの着せ替えの一覧に出るモジュールの警告 (`entry.warnings`) が、wasm の文のまま出ている。
T0125 で `modules_json()` の要素に `warning_details` が足されたので、そこから英語の知らせを作る。

## 受け入れ条件

- [x] `warning_details` から、読み込みの警告 (`preview/loadWarnings.ts`) と同じ言い回しの英語の知らせを作って一覧に出す
- [x] Vitest。`just ci`・エディタのビルドが通る

## 参考

- `docs/tasks/T0125-wasm-load-warnings-structured.md` のメモ、`crates/ugoe-wasm/README.md` の「読み込みの警告 (T0125)」

## メモ

- **まとめた関数:** `preview/loadWarnings.ts` の警告 1 つの文を `describeLoadWarning(w)` に切り出し、読み込みの警告
  (`describeLoadWarnings`) とモジュールの警告 (`describeModuleWarnings(warnings, details)`) の両方で使う。要素の読み取りは
  `readLoadWarnings(list)` (`parseLoadWarnings` はこれを呼ぶ)。モジュールの警告は、ビューアの `replaceWarnings` と同じく文の警告の順に
  並べ、`warning_details` の `warning` と同じ文を知らせに置き換える (構造の無い文・古い wasm はそのまま)。`dressUp.ts` の
  `parseModules` がこれを呼ぶので、一覧 (`DressUpSection.svelte`) は変えていない。テクスチャの番号はモジュールのテクスチャの番号
  (行の下に出るので、文は読み込みの警告と同じ「Texture 0 …」のまま)。
- **ドキュメント:** `docs/editor.md` の「読み込みの警告 (T0125)」に 1 文足した。
- **テスト:** Vitest `loadWarnings.test.ts` (`describeModuleWarnings`: 構造のある文・無い文・`details` が無い)、`dressUp.test.ts`
  (`parseModules` が wasm の本物の文と `warning_details` から知らせを作り、`warning_details` を落とす)。
- **確かめたこと:** main (T0131) に rebase 後、`just wasm` 2 つ・`just ci`・エディタの `vite build`。ヘッドレスの Chrome (SwiftShader) と
  本物の `Document` への使い捨ての HTTP ブリッジ (scratchpad/T0652/bridge。T0651 のものに、最初のテクスチャを KTX2 にした
  `.ugoemod` を作る `ktx-module` を足した) で、T0648 の本体 (Headwear のスロット) のプレビューに KTX2 の `KittyKtx` と元の `Kitty` を
  読ませ、`KittyKtx` の行の下に「Texture 0 is a KTX2 image, which the preview can't show yet, so it is drawn white.」、`Kitty` には
  何も出ないことを確かめた。横のはみ出し 0。スクリーンショット: scratchpad `T0652/shots/t0652-module-warnings.png`。
  使い終えた vite preview とブリッジは止めた。
- **確かめていないこと:** Tauri のウィンドウ。法線マップが KTX2 のモジュールの画面 (Vitest だけ)。
- **範囲外の変更:** なし (apps/editor・docs だけ)。
- **後続の候補:** なし (ビューアとエディタで言い換えの型が別々にあるが、言語が違い、共有の置き場 (packages/) に移すほどではない)。
