---
id: T0626
title: エディタのファイル選択ダイアログ
status: done
milestone: M2
crates: [apps/editor]
depends_on: [T0623]
assignee: claude-worker-T0626
---

# T0626: エディタのファイル選択ダイアログ

## 目的

今のエディタは、開く・保存・書き出し・PSD の取り込みで、パスを入力欄に打ち込む。
T0619 の確認では、Save As に相対パス (`test.ugoeproj`) を入れると、開発サーバーの作業ディレクトリ (`apps/editor/src-tauri`) を基準に解釈され、リポジトリの中の思わぬ場所に保存された。
OS のファイル選択ダイアログを使い、パスの手入力をなくす。

## 受け入れ条件

- [x] Tauri のダイアログプラグイン (`tauri-plugin-dialog` / `@tauri-apps/plugin-dialog`、ライセンスを確認する) を入れ、権限 (capabilities) を最小限に設定する
- [x] New from PSD… / Import PSD… / Open / Save As / Export が OS のダイアログを使う (PSD・`.ugoeproj`・`.ugoe` のフィルタ、既定のファイル名、`.ugoeproj` はディレクトリの選択または作成)
- [x] パスの入力欄は、ダイアログが使えない環境の予備としてだけ残すか、取り除く (決めて書く)。残す場合、相対パスはホームディレクトリ基準で解釈し、どこに保存したかを表示する
- [x] ヘッドレスのブラウザでの確認のため、ダイアログの呼び出しを差し替えられるようにする (テストではダイアログの代わりに固定のパスを返す)
- [ ] `docs/editor.md` と操作マニュアル (`docs/manual/`、T0624) の該当箇所を更新する (`docs/editor.md` は更新した。マニュアルは main にまだないので、メモに申し送りを書いた)
- [x] `just ci`・`just licenses`・エディタのビルドが通る

## メモ

- **依存:** `tauri-plugin-dialog` 2.8 (Rust、`[workspace.dependencies]`) と `@tauri-apps/plugin-dialog` 2.8 (JS)。どちらも
  MIT OR Apache-2.0 で GPL-3.0 と両立する。Rust 側は `tauri-plugin-fs`・`rfd` などを引き込む (`just licenses` は通る)。
  権限は `dialog:allow-open` と `dialog:allow-save` だけ (`dialog:default` は message・ask・confirm も含むので使わない)。
- **入力欄は取り除いた。** ダイアログが使えない環境の予備も置かない (ヘッドレスの確認は差し替えで行える)。加えて、
  Tauri コマンド (`open_project`・`save_project_as`・`export_bundle`・`import_psd`・`new_project_from_psd`) が
  相対パスを `path must be absolute: …` で拒否するようにした (`project::absolute_path`。T0619 の事故の再発防止)。
- **ダイアログの使い分け:** PSD はファイルを開く (`.psd` のフィルタ)、Open は `.ugoeproj` がディレクトリなのでフォルダーの選択、
  Save As / Export は保存のダイアログ (`.ugoeproj` / `.ugoe` のフィルタ、既定は `<プロジェクト名>.ugoeproj` / `.ugoe`)。
  選んだ名前に拡張子がなければフロントエンドで付け足す。最初の場所は、最後に成功した場所 → プロジェクトの隣 → ドキュメント
  (なければホーム)。保存・書き出しの後は書いた場所の絶対パスをメニューの横に出す。ダイアログの間はボタンを押せない。
- **差し替え:** `window.__UGOE_FILE_DIALOGS__` に `FileDialogs` を置くとダイアログの代わりに呼ぶ (`src/lib/fileDialogs.ts`)。
  純粋な関数 (フィルタ・拡張子の付け足し・パスの結合・親のディレクトリ・ファイル名の整形) は `src/lib/filePaths.ts` で Vitest 7 件。
  Rust は相対パスの拒否のテストを足し、`project_dir` のテストを絶対パスに直した。
- **検証したこと:** ヘッドレス Chrome (WebGL2、SwiftShader) で `vite build` した画面に、Tauri のプロジェクトと PSD のコマンドを
  本物の `Document`・`Project::save/load`・`import_psd`・`export_bundle` で真似る使い捨ての HTTP ブリッジ (スクラッチ領域、
  絶対パスの規則も同じ) と、固定のパスを返すダイアログの差し替えをつないだ。New from PSD (`character.psd`、フィルタ `.psd`、
  既定の場所がドキュメント) → Save As (拡張子なしの名前で `mychar.ugoeproj` ができ、名前が `mychar`、既定は PSD の隣の
  `character.ugoeproj`) → Export (`mychar.ugoe` ができる、既定は `…/mychar.ugoe`) → + Part で未保存にして Open (1 回目は確認で
  ダイアログを出さず、2 回目にフォルダーの選択、既定はプロジェクトの隣) → 取り消した Import PSD は何もしない → 相対パスの
  Save As はエラーで、何も書かず、最初の場所も変えない → Import PSD (`layers.psd`) で警告が出る、を確かめた。入力欄はない。
- **実際の Tauri のウィンドウで確かめること (未確認):**
  - 各ダイアログが開き、フィルタ (`.psd` / `.ugoeproj` / `.ugoe`) と既定の名前・場所 (ドキュメント) が効くこと。
  - Open のフォルダーの選択で `.ugoeproj` を選べること (macOS で `.ugoeproj` がパッケージ扱いされないか、Windows・GTK で
    フォルダーの中に入らずに選べるか)。
  - Save As で既存の `.ugoeproj` を選んだとき、OS の上書きの確認 (ディレクトリに対して) がどう出るか、上書き保存できるか。
    保存のダイアログがフォルダーを「作る」ことはないので、選んだ名前のディレクトリはバックエンドが作る。
  - Windows (T0625 のクロスビルド) で `documentDir()` と `\` のパスの既定の名前が正しいこと。
- **マニュアル (T0624) への申し送り:** `docs/manual/` は rebase の時点で main になかった。ファイルの選び方は「ボタンを押すと
  OS のダイアログが開く。Open は `.ugoeproj` フォルダーを選ぶ。Save As / Export は名前を入れれば拡張子が付く。保存・書き出しの
  後にメニューの横に保存先が出る」と書く。パスの入力欄の説明があれば消す。
