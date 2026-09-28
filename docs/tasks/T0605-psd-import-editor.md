---
id: T0605
title: エディタ: PSD の取り込みと自動メッシュ
status: done
milestone: M2
crates: [ugoe-editor-core, apps/editor, ugoe-cli]
depends_on: [T0501, T0502, T0601, T0604]
assignee: claude-worker-T0605
---

# T0605: エディタ: PSD の取り込みと自動メッシュ

## 目的

PSD を読み込んで、レイヤーの構造からパーツ・テクスチャ・メッシュ (自動メッシュ) を作る。M2 の完了条件の入口になる。

## 受け入れ条件

- [x] `Command::ImportPsd` (または取り込み結果を表す `Batch`) が、グループをパーツに、ラスタレイヤーをテクスチャとメッシュにする。レイヤーの位置と描画順 (上のレイヤーほど手前) を保つ
- [x] メッシュは `ugoe-mesher` で自動生成する。メッシュの再生成 (`RegenerateMesh { mesh, options }`) もコマンドとしてある
- [x] テクスチャの画像はプロジェクト (`textures/`) に保存され、保存・読み込みで失われない
- [ ] エディタのメニューから PSD を選んで取り込め、ビューポートに表示される
- [x] テスト用の小さな PSD (`assets/fixtures/`) を取り込むテストがある
- [x] `just ci` が通る

## 参考

- `docs/ai-integration.md#psd-取り込み`、`docs/ai-integration.md#自動メッシュ`
- `docs/spec/file-formats.md#プロジェクト-ugoeproj`

## メモ

- 取り込みは `ugoe_editor_core::import::import_psd` (純粋な関数。`Command::Batch` と警告を返す)。
  `Command::ImportPsd` は作らなかった (エディタ・CLI・AI が同じ関数でコマンドを作り、`Document::apply` する)。
  対応付けの規則は `docs/editor.md#psd-の取り込み` に書いた。要点:
  - 取り込んだもの全体を最上位のパーツ 1 つ (名前はファイル名) の下に入れる。空のモデルなら
    `set_canvas` (新コマンド) でキャンバスを PSD の大きさにする。既存のモデルではキャンバスを変えず警告する。
  - 非表示のレイヤー・グループは取り込んで不透明度 0 にする (差分の絵を残すため)。
  - テクスチャはレイヤー画像の周りに `ceil(margin) + 1` px の透明な余白を足す (膨張した輪郭の UV が
    0〜1 に収まり、縁の画素が引き伸ばされないように)。
  - 描画順は全体で下から上に `1000 × (i + 1) / (n + 1)`。既存のモデルに取り込むと既存のメッシュと混ざる。
  - クリッピングは土台 (直下のクリッピングでない要素。グループならその中の全メッシュ) への通常マスク。
    土台の不透明度 (非表示なら 0) を掛ける。グループのクリッピングは警告して無視する。
  - ID は日本語だけの名前なら PSD のレイヤー ID から `Layer12` / `Group8`。
- `RegenerateMesh { id, options }` (他のメッシュのコマンドに合わせて `mesh` ではなく `id`)。すべての
  キーフォームの頂点位置が同じで、頂点位置が UV のアフィン写像 (最小二乗で求め、ずれを検査) の
  メッシュだけを作り直す。グルーが参照するメッシュ・`depth` を持つメッシュも拒否する。
  キーフォームの頂点位置以外の値は保つ。
- `ugoe-mesher::MeshOptions` は serde を持たないので、editor-core に `AutoMeshOptions` (serde 付き、同じフィールド) を置いた。
- Tauri: `import_psd(path, mesh_options?)` (`src-tauri/src/psd.rs`)。フロントエンドはファイルメニューに
  「Import PSD…」(パス入力欄) と警告の一覧を足した。取り込み後は `onchange` → `EditorDocument.refresh()`
  でビューポート (T0402) がバンドルを読み直す。
- **確かめていないこと:** GUI は起動していないので、メニューからの取り込みとビューポートへの表示は
  実際には見ていない (受け入れ条件 4 は未チェック)。`svelte-check` とビルドは通る。
- CLI に `ugoe import-psd <psd> -o <dir.ugoeproj> [--bundle x.ugoe]` を足した (タスクの範囲に `ugoe-cli` を追加)。
  `layers.psd` を取り込んで `ugoe render x.ugoe -o x.png --scale 8 --background '#808080'` で描き、目で確かめた:
  背景・胴・肌が PSD の位置に出て、影 (乗算) は肌の内側で暗い四角になり、非表示の目は見えず、
  キャンバスの左上にはみ出したメモが半透明で出る。体グループの不透明度 (90%) で胴と肌がわずかに透ける。
  テスト画像の模様にある縞 (色の値の桁あふれ) はフィクスチャの模様そのもの。
- 後続の候補: 既存のモデルに取り込むときの配置と描画順の指定、レイヤーの差し替え (再取り込み)、
  ファイルダイアログ、取り込み時のアトラス化の設定。
