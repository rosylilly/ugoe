---
id: T0623
title: エディタを空のプロジェクトで起動し、PSD から新規作成できるようにする
status: done
milestone: M2
crates: [ugoe-editor-core, apps/editor]
depends_on: [T0619]
assignee: claude-worker-T0623
---

# T0623: エディタを空のプロジェクトで起動し、PSD から新規作成できるようにする

## 目的

T0619 の 2 回目の確認で、Import PSD のたびに「PSD のキャンバス (320x400) がモデルのキャンバス (256x256) と違う」という警告が出た。
エディタが起動時にサンプルのモデル (`single_quad`、256x256) を開いているため、PSD がそこに追加されるのが原因。
初めて使う人が迷わないように、空のプロジェクトで起動し、PSD から新しいプロジェクトを作れるようにする。

## 受け入れ条件

- [x] エディタは空のプロジェクト (メッシュ・パーツなし) で起動する。空のモデルへの Import PSD は、PSD の大きさをキャンバスに使い、警告を出さない (T0605 の既存の規則)
- [x] File に「New from PSD…」がある: 未保存の変更の確認のあと、新しいプロジェクトを作って PSD を取り込む (1 回の操作)
- [x] 空のプロジェクトでは、ビューポートとツリーに次の操作の案内 (例: 「File > New from PSD… で始める」) を出す
- [x] 既存のモデルに取り込む場合の警告の文言を、何が起きるか分かるものにする (キャンバスを変えない・位置は PSD のまま)
- [x] テストと `docs/editor.md` を更新する
- [x] `just ci`・`just gen` (差分なし)・エディタのビルドが通る

## メモ

- **起動時の状態**: `Project::empty("Untitled")` (1024×1024、保存先なし) にした (`new_project` と同じ)。
  起動時のサンプル (`single_quad` と白いテクスチャ) はなくなった。
- **New from PSD…**: Tauri コマンド `new_project_from_psd(path, mesh_options?)` (`src-tauri/src/project.rs`)。
  PSD のファイル名の空のプロジェクトを作って `import::import_psd` を `Document::apply` し、成功したら今の
  プロジェクトと置き換える (失敗したら今のまま)。取り込みは Undo の 1 単位で、未保存の状態から始まる
  (Undo すると空のプロジェクトに戻る)。PSD の読み込みは `psd::read_psd` に分けて `import_psd` と共有した。
  ファイルメニューの確認は New / Open と同じ (未保存の変更があれば 2 回押す)。
- **空のモデルの扱い**: 保存・書き出しはそのまま許す (メッシュのない `.ugoe` になり、読み直せる。Rust のテストで確認)。
  ビューポート (`load_bundle`) とパネルも空のモデルで問題なく動いた (下の検証)。空の判定は取り込みの規則と同じ
  (パーツ・メッシュ・デフォーマ・テクスチャがない。`src/lib/emptyState.ts`)。
- **案内**: 空のプロジェクトでは、ビューポートの中央に「This project is empty. Use File > New from PSD… to start
  from a PSD file.」、ツリーに「No parts yet. Start with File > New from PSD…」を出す。
- **警告の文言**: `the PSD canvas (WxH) differs from the model canvas (WxH); the model canvas is kept and the layers
  are placed at their PSD pixel positions, so they may not line up with the canvas (import into an empty project
  to use the PSD canvas)`。CLI も同じ関数を使うので、エディタのメニュー名は入れなかった。
- 新規・開く・取り込みの後にビューポートを Fit に戻すようにした (キャンバスの大きさが変わるため)。
- **テスト**: Rust 4 件 (Tauri: 起動時の状態が空で書き出せること、起動時の状態への `character.psd` の取り込みで
  キャンバスが 320x400・警告なし、`from_psd` の名前・未保存・書き出し・Undo、壊れたファイルの拒否。
  editor-core: 警告の文言の検査を更新)。Vitest 1 件 (`isEmptyModel`)。
- **検証したこと**: ヘッドレス Chrome (WebGL2、SwiftShader) で `vite build` した画面に、Tauri のプロジェクトと
  PSD のコマンドを本物の `Document` と `import_psd` で真似る使い捨ての HTTP ブリッジ (スクラッチ領域) をつないだ。
  起動時に両方の案内が出ること、+ Part で未保存にした後の「New from PSD…」が確認を求め、2 回目で入力欄が出ること、
  `assets/fixtures/character.psd` で名前が `character`・警告なし・キャンバス 320x400 (原点 160,200)・メッシュ 12・
  案内が消えること、そこへ `layers.psd` を Import PSD すると新しい文言の警告が出ること、New で空に戻り案内が出ることを
  確かめた。
- **検証できなかったこと**: 実際の Tauri の WebView での操作。
