---
id: T0646
title: エディタ: ライティングの操作 (有効化・法線マップ・プレビューの光源)
status: done
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs, ugoe-cli]
depends_on: [T0210, T0506, T0640]
assignee: claude-worker-T0646
---

# T0646: エディタ: ライティングの操作 (有効化・法線マップ・プレビューの光源)

## 目的

利用者の指摘。ライティングはレンダラとホストにはあるが、エディタから操作できない。エディタでライティングを設定し、確かめられるようにする。

## 受け入れ条件

- [x] ライティングのパネル: モデルのライティングの有効・無効 (`SetLighting`)。メッシュごとの有効・無効 (`Mesh::lit` を変える Command を足す。Undo のテスト)
- [x] 法線マップ: 選んだメッシュのテクスチャに、形からの生成 (AI なし。`ugoe-rigging` の bulge。強さ・縁の丸みを選べる) か、画像の割り当てで法線マップを付け・差し替え・外す (`SetNormalMap`)。法線マップの画像を確かめられる
- [x] プレビューの光源: 環境光・平行光源・点光源をビューポートの上で置き、向き・色・強さを変えて確かめられる。光源はモデルに入らない (ホストが渡す) ので、プレビューの設定としてプロジェクトに残すかを決めて書く
- [x] ヘッドレスのブラウザで確かめる。操作マニュアルに章 (`docs/manual/lighting.md`)
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/spec/rendering.md#ライティング`、`docs/tasks/T0209`・`T0210`・`T0506`・`T0640` のメモ

## メモ

- **Mesh::lit:** 新しい Command にせず、`SetMeshProperties` の末尾に `lit: Option<bool>` を足した (Command の enum と OP_COUNT は
  変わらないので T0647 と衝突しない)。Undo のテストは `lighting_tests.rs`。
  - **範囲外の変更:** 構造体リテラルを使う `crates/ugoe-cli/src/sample.rs` に `lit: None,` を 1 行足した (コンパイルを通すだけ。
    team-lead に相談済み、返事待ちのまま進めた)。
  - モデルの JSON は `lit: true` と `lighting: false` を省略するので、フロントエンドは `lit` の欠けを true とみなす (`lighting.ts` の `isLit`)。
- **法線マップ:** コマンドの組み立ては `ugoe_editor_core::lighting` (`generate_normal_map`・`assign_normal_map`・`normal_map_name`)。
  editor-core はすでに ugoe-rigging に依存していたので、Tauri 側に依存は足していない。`SetNormalMap` (ライティングが無効なら
  `SetLighting` も) を `Batch` にして 1 回の Undo で戻る。画面の既定の強さは 0.5 (`BulgeOptions` の既定 1 は大きなパーツに強い、T0506)。
  「Round edges only」は `BulgeOptions::radius`。PNG の割り当ては「Flip green (DirectX)」で G を反転できる。名前の規則は `ugoe_ai::normal` と同じ
  (`<texture>_normal`、共有していれば新しい名前) だが、ugoe-ai の private な関数なので editor-core に書き直した。
- **光源の保存:** `project.json` の `editor.lighting` (`PreviewLighting`。既定値なら書かない、読めなければ既定値)。値が丸めの誤差で
  読みにくくならないよう `f64` で持ち、取っ手の向きは小数 3 桁に丸める。変えると未保存の印が付く (Undo の対象ではない)。
  `docs/spec/file-formats.md` の表に行を足した。
- **画面:** 右の列の「Lighting」タブ (`panels/lighting/`)、光源の取っ手 (`viewport/tools/lighting/LightingOverlay.svelte`。タブを開いている間と
  プレビュー中)、プレビューのパネルにも同じ光源の節。`ViewportController.setLighting` と `usesLighting`。App.svelte はタブ 1 つ・
  オーバーレイ 1 行・光源の `$effect` と読み込みだけを足した。
- **確かめたこと:** T0644 と同じ方法 (ビルドした画面 + 本物の `Document` / `ugoe_editor_core::lighting` への HTTP ブリッジ + ヘッドレス Chrome、
  scratchpad の `T0646/`) で、`comfyui-see-through-bustup-parts.psd` から新規 → 「Lighting」タブの案内 (ライティングが無効) → face・topwear・
  front hair・back hair・neck に強さ 1 で生成 (ライティングが自動で有効、画像の表示) → 平行光源の取っ手を右・左へドラッグ (顔の左右の頬の
  明るさが 81 / 136 → 102 / 81 と入れ替わる) → `editor.lighting` に向きが残る → 点光源を足してドラッグ → Undo で最後の生成だけが戻り Redo →
  face の「Lit」を外すと顔だけ陰がなくなる → 「Light the viewport」を切ると平らになる → nose に PNG を Flip green で割り当て → 外す →
  プレビューのパネルに光源の節。画像は scratchpad の `T0646/shots/c-*.png` (マニュアルの画像 2 枚もこれから作った)。
- **確かめていないこと:** 実際の Tauri のウィンドウ (OS のファイルダイアログでの PNG の選択)、WebGPU のバックエンド (ヘッドレスは WebGL2)。
- **後続の候補:**
  - ビューア (T0705) とエディタで光源の UI・JSON の変換を別々に持っている (`apps/viewer/src/lib/lighting.ts` と `panels/lighting/lighting.ts`)。
    `packages/` に共有の部品として切り出す。
  - 選んだ複数のテクスチャ (パーツ) にまとめて法線マップを作る操作、意味ラベルごとの既定の強さ (T0506 の後続と同じ)。深度推定からの生成 (AI) を画面から。
  - パーツの重なりで境目に段差が出る問題 (T0506 のメモ) の軽減。
