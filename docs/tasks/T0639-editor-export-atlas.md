---
id: T0639
title: エディタ: 書き出しでテクスチャのアトラス化を選ぶ
status: done
milestone: M4
crates: [apps/editor, ugoe-editor-core, docs]
depends_on: [T0114]
assignee: claude-worker-T0639
---

# T0639: エディタ: 書き出しでテクスチャのアトラス化を選ぶ

## 目的

T0114 のアトラス化 (`Project::with_atlas`) を、エディタの `.ugoe` の書き出しから選べるようにする。

## 受け入れ条件

- [x] 書き出しの画面でアトラス化の有無・最大の大きさ・余白を選べ、プロジェクトに設定が残る
- [x] 書き出しの後に、テクスチャの枚数と描画の状態の切り替え回数の変化を示す
- [x] 同じ画像の重複を除くか、着せ替えのモジュールのテクスチャをどう扱うかを決めて書く (T0114 のメモの後続の候補)
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0114-texture-atlas.md` のメモ
- `docs/spec/file-formats.md#テクスチャのアトラス化`

## メモ

- **設定の置き場所:** `project.json` の `editor.export` (`ugoe_editor_core::ExportSettings`、`export.rs`)。`editor` は元から
  自由な JSON なので形式のバージョンは上げない。すべて既定値なら書かず (既存のプロジェクトの `project.json` は変わらない)、
  足りないフィールドは `serde(default)`、読めない値は既定値で読む。範囲は最大の大きさ 1〜16384・余白 0〜64。
  設定はモデルを変えないので `Command` にせず、Undo の対象にもしない。代わりに `ProjectSession.meta_modified` で未保存の印を付ける。
- **IPC:** `get_export_settings`・`set_export_settings(settings)` を足し、`export_bundle(path)` はプロジェクトの設定で書き出して
  `ExportReport` (前後の `DrawEstimate` とアトラス化の `AtlasReport`) を返すようにした。画面は「Export…」で開く欄
  (`ExportPanel.svelte`) で、チェック・最大の大きさ・余白と「Export .ugoe…」、書き出しの後に前 → 後の要約 (`exportSummary.ts`)。
- **切り替え回数:** エディタのバックエンドは `ugoe-render` に依存していない (GPU を持たない) ので、`estimate_draws` で見積もる。
  既定のパラメータの描画順で、(テクスチャ・合成モード・カリング) が変わる回数。マスクアトラスの違いは数えない。
  `character.psd` では前後とも `ugoe render --stats` と一致した (テクスチャ 12 → 1、`state_changes` 10 → 2、`draw_calls` 11 → 3。
  残る 2 回は合成モードの違い)。アトラス化の前後の `ugoe render` の画像はバイト単位で一致した。
- **重複の除去 (T0114 の後続の候補):** 行う。デコードした画素が完全に同じ画像は、描画順で先に使われるものだけを詰め、ほかも同じ
  場所を指す (`AtlasReport::deduplicated`)。`character.psd` では `Layer13` (右目) が `Layer12` (左目) と同じ画像だった。
  アトラス化しないときは重複を除かない (テクスチャの名前が変わらないようにするため)。
- **着せ替えのモジュール:** アトラスに含めない (モジュールは実行時に本体へ足して差し替えるので、本体のアトラスに入れると差し替えの
  たびに作り直しになる)。`docs/spec/file-formats.md#テクスチャのアトラス化` に書いた。モジュールの単位のアトラス化は T0113 の後の候補。
- **T0112 との関係:** `atlas.rs` は「UV が 0〜1 の外なら残す」判定の付近を動かしていない (重複の除去は並べる順を作る所だけ)。
  rebase で T0112 の `KeptReason::HasNormalMap` と合わせ、`just gen` をやり直し、画面に「has a normal map」を足した。
  画面の理由の表示は知らない理由をそのまま出す。
- **確かめたこと:** ビルドした画面 (`vite build`) をヘッドレス Chrome (Playwright・SwiftShader) で開き、本物の `ugoe_editor_core`
  (`Document`・`ProjectMeta::set_export_settings`・`export_bundle_with_settings`) に中継する使い捨ての HTTP ブリッジ (スクラッチ領域、
  リポジトリ外) につないで、`character.psd` を取り込んだ状態から「Export…」→ チェック → 1024px・余白 1 → 「Export .ugoe…」で
  要約が出ること、未保存の印が付くこと、余白 100 は拒否されること、設定が残ることを確かめた。書き出したファイルは `ugoe render --stats` で確認。
- **確かめていないこと:** 実際の Tauri のウィンドウでの操作。
- **後続の候補:** 画像の中の使われない領域の切り取り、ミップマップ向けの揃え、モジュールのテクスチャのアトラス化 (T0113 の後)、
  マスクアトラスまで含めた切り替え回数 (エディタで `ugoe-render` をヘッドレスに使う口ができたら)。
