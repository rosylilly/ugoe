---
id: T0654
title: エディタ: 今の編集と書き出したモジュールの違いを文字で出す・本体だけのバンドルをセッション側で持つ
status: done
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0653]
assignee: claude-worker-T0654
---

# T0654: エディタ: モジュールの比較の仕上げ

## 目的

T0653 の後続の候補をまとめる。

## 受け入れ条件

- [x] プレビューの着せ替えの一覧で、今の編集 (`<名前> (editing)`) と、同じ名前の書き出したファイルの違い (要素の数・テクスチャ・条件など) を文字でも出す
- [x] 本体だけのバンドルを、フロントエンドではなくセッション (Tauri の側) で持ち回る (今は PreviewMode がバイト列を覚えている)
- [x] エディタのモジュールの書き出し (Export Module…) でも、アトラス化 (`build_module_atlas`、T0132) を選べるようにする
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0653-editor-module-project-polish.md` のメモ

## メモ

- **違いを文字で出す:** editor-core に `module_compare.rs` (`compare_with_module_file(project, bytes)`・`module_differences`) を足した。
  今の編集を書き出しの検査 (`project::checked_module`。`export_module` から切り出した) に通したモジュールと、書き出した `.ugoemod` を比べ、
  「ファイル → 今の編集」の英語の行にする: 名前・スロット、パーツ・デフォーマ・メッシュ・グルーの数と足した・除いた・変えた ID (メッシュの
  テクスチャは番号でなく名前で比べる)、テクスチャ (足した・除いた・画像か参照が違う)、条件 (レベル・スロット・パラメータ)、使うパラメータ、
  物理のリグと衝突体、ポーズ。ファイルだけがアトラス化してあれば注記を 1 行足す。IPC `compare_preview_module` は `.ugoemod` を本文の
  バイト列で受ける (`tauri::ipc::Request` の `InvokeBody::Raw`。パスの無いドロップしたファイルでも比べられるように)。フロントエンドは
  `DressUpEntry.differences` (今の編集と同じ名前のファイルの行だけ。`null` は途中) を、`setOwn` のたびとそのファイルを読み込むたびに
  取り直し (古い結果は捨てる)、行の下に「N differences from your current edits」の `<details>` か「Same as your current edits.」を出す。
- **本体だけのバンドルをセッションで持つ:** `AppState.preview_base` (`module::PreviewBaseCache`) が最後の 1 つを覚え、本体の版が同じか、
  本体のファイルの指紋 (`ProjectSession.base_disk` の `digest`) とパスが同じなら作り直さない。フロントエンド (`PreviewMode`) はバイト列を
  覚えず、描いている本体の版 (`#baseRevision`) だけを持ち、違うときだけ `get_preview_base_bundle` を呼ぶ。
- **モジュールの書き出しのアトラス化:** `ugoe_editor_core::export_module_with_settings(project, settings)` (`build_module_atlas`)。
  `ModuleExport`・`ModuleExportReport` に `atlas` を足した。設定は `.ugoe` と同じ `editor.export`。画面は「Export Module…」で同じ
  `ExportPanel` を開き (ボタンの文言を `label`、結果を `summary` の行で渡すようにした)、結果は `moduleExportSummary` (大きさとアトラス化の詳細)。
- **IPC:** `compare_preview_module` を足し、`ModuleExportReport.atlas` を足した (ipc.ts・`just gen` の types.ts)。Command の enum は足していない
  (`OP_COUNT` は 101 のまま)。
- **テスト:** editor-core `module_compare_tests.rs` (同じなら空・メッシュを除く/条件/画像の違い・壊れたファイル・アトラス化した書き出しと注記・
  既定の設定では `export_module` と同じバイト列)、Tauri `module.rs` (`preview_base_bundle_is_kept_in_the_session`: 同じ版・開き直し
  (版は変わる) で使い回し、本体を変えて読み直すと作り直す / `preview_module_is_compared_with_the_exported_file`)、Vitest
  (`shows how the exported file differs from the current edits (T0654)`・`moduleExportSummary`)。
- **確かめたこと:** main に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci`・`cargo check -p ugoe-editor`・`just test-gpu`。
  ヘッドレスの Chrome (SwiftShader) と本物の `Document`/`Project` への使い捨ての HTTP ブリッジ (scratchpad/T0654/bridge。T0653 のものに
  `compare_preview_module` (本文のバイト列)・`PreviewBaseCache` と同じ規則の `get_preview_base_bundle`・設定を使う `export_module`・
  `set_export_settings`・本体を変える `bridge_mutate_base` を足した) で `ugoe sample dress-up --project` の見本の cat-ears.ugoeproj:
  - Export Module… → そのまま書き出すと `Size: 9,691 bytes`、「Pack textures into atlases」を入れると `Size: 9,644 bytes`・
    `Packed 2 textures into 1 atlas (228×184)` (T0132 の CLI と同じ大きさ)。設定を変えると未保存の印。
  - プレビュー: 見本の cat-ears.ugoemod (同じ名前) を読み込むと「Same as your current edits.」→ エージェントの編集で `Ears` を消すと
    「1 difference … Meshes: 2 → 1 (removed: Ears)」(本体は作らず・取らず) → アトラス化したファイルを読み込むと同じ行が置き換わり
    「Meshes: 2 → 1 (removed: Ears; changed: Band)」「Textures: 1 → 2 (added: ears, band; removed: atlas-0)」と注記。そのファイルを付けてもエラーなし。
  - 本体の作り直し: 入る 1 回 → 出て入り直す 0 回 (取るのは 1 回) → 開き直す (本体の版 1 → 2) 0 回 → 本体のファイルを変えて確認の画面で
    「Reload the base」→ 1 回。横のはみ出し 0。
  スクリーンショット: scratchpad/T0654/shots/t0654-*.png (マニュアルに module-export-atlas.png・module-compare-differences.png を足した)。
  使い終えた vite preview とブリッジは止めた。
- **確かめていないこと:** 実際の Tauri のウィンドウ (`Request` の本文で送るバイト列・`State` 越しのキャッシュ)。本体のファイルを読み込んだ後、
  指紋を取るまでの間に書き換えられたとき (指紋と読んだ中身が食い違い、古い本体のバイト列を使い回し得る。`ProjectSession.base_disk` と同じ隙間)。
  大きなモジュールで、編集のたびに比べ直す重さ (同じ名前のファイルを読み込んでいるときだけ。今の編集の取り直しと同じ程度の処理)。
  法線マップを持つモジュールのアトラス化 (まとめずに残る。見本がない)。
- **範囲外の変更:** なし (editor-core・apps/editor・docs だけ)。
- **後続の候補:**
  - 違いの行を、どのメッシュの何 (頂点・キーフォーム・描画順) が変わったかまで細かくする。アトラス化したファイルとは UV を戻して比べる。
  - 違いの行からその要素を選ぶ (ツリーで選ぶ) 。
  - 見本の着せ替えのモジュールをアトラス化して配るか (T0132 の候補の残り)。
