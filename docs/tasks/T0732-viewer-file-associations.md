---
id: T0732
title: ビューアを OS の「このアプリで開く」に対応させる (fileAssociations と macOS の RunEvent::Opened)
status: done
milestone: M9
crates: [apps/viewer, docs]
depends_on: []
assignee: claude-worker-T0732
---

# T0732: ビューアを OS の「このアプリで開く」に対応させる (fileAssociations と macOS の RunEvent::Opened)

## 目的

`apps/viewer/src-tauri/src/lib.rs` の `initial_file` の doc コメントは「OS の『このアプリで開く』」と書くが、`tauri.conf.json` に
`bundle.fileAssociations` が無い。macOS はファイルを起動の引数ではなく `RunEvent::Opened` で渡すので、対応しても届かない (T0724 の所見と起票の候補)。

## 受け入れ条件

- [x] `tauri.conf.json` の `bundle.fileAssociations` に `.ugoe`・`.ugoemod` を登録した
- [x] macOS の `RunEvent::Opened` を受け、起動時と起動中のどちらでもファイルを開く (起動中に開いたときの扱いを決めて docs に書いた)。開いたパスは `FileAccess` に許可として加える
- [x] 受け取ったパスを開く部分 (引数・`Opened` の両方) のテストがある
- [x] `initial_file` の doc コメントと apps/viewer/README.md・docs/manual を実際に合わせた
- [x] `just ci` が通る (Windows・macOS の実機で確かめていないことはメモに書き、T0731 の手動確認の項目に足した)

## 参考

- T0724 のメモ (所見の表と起票の候補)、T0718 のメモ、T0731

## メモ

- T0026 で起票した (元の候補: T0724 の「このアプリで開く」)。オーケストレーターの集計には無かったが、M9 の done のタスクのメモの起票の候補にあった。

### やったこと (T0732)

- `tauri.conf.json` の `bundle.fileAssociations` に `.ugoe` (`ugoe model`) と `.ugoemod` (`ugoe dress-up module`) を登録した
  (role `Viewer`。macOS の UTI `app.ugoe.model`・`app.ugoe.module` (`public.data`)、MIME `application/x-ugoe`・`application/x-ugoe-module`)。
  JSON のモデルは、ほかのアプリの JSON まで開くので登録しない (テスト `registers_bundles_and_modules_with_the_os`)。
- バックエンド (`lib.rs`): `OpenedFiles` が OS から渡されたパスを、画面が受け取るまで溜め、受け取った後はイベント `open-files` で送る。
  起動時の引数 (`paths_from_args`: `-` で始まるものと読めない拡張子を除き、相対パスは絶対パスに。前はモデルだけだったが `.ugoemod` も受ける)
  と macOS の `RunEvent::Opened` (`paths_from_urls`: `file:` の URL だけ) の両方が `open_from_os` を通る。`OpenedFiles` は `Builder::manage`
  で置くので、`Opened` が `setup` より先に届いても受けられる。読めるパス (`FileAccess`) には、画面へ渡すとき (`initial_files`・送る前) に加える。
  `run` は `build(...).run(on_run_event)` にした。
- コマンド `initial_file` (モデル 1 つ) を `initial_files` (パスの配列) に替えた (acl.rs・capabilities も)。
- 画面: `ViewerHost.listenOpenedFiles` (Tauri) は先に `open-files` を待ち受けてから `initial_files` を呼ぶ。`App.svelte` は届いたパスを
  ドロップと同じに開く (`openPaths`)。`initialModelFile` はブラウザの `?model=` だけになり、省けるようにした。
- **起動中に開いたときの扱い** (README の「OS から開く」・docs/manual/dress-up.md): ドロップと同じ (モデルは入れ替え、`.ugoemod` だけなら今のモデルに
  付ける、開けないものは飛ばして知らせる)。macOS は同じウィンドウ。Windows・Linux は OS が新しいプロセスを起動するので別のウィンドウ。
- テスト: Rust (`holds_files_from_the_os_until_the_page_takes_them`・`opens_models_and_modules_from_the_arguments`・`opens_file_urls_from_macos`
  ・`registers_bundles_and_modules_with_the_os`)、TS (`host.test.ts` の「files opened from the OS」: 待ち受けと受け取りの順・後から届いたもの・失敗で待ち受けをやめる)。
- 文書: README (仕組み・使い方の表・「OS から開く」の節・読み込みの範囲)、docs/manual/dress-up.md、docs/manual-check.md に V08、T0731 の受け入れ条件に 1 項目足した。

### 確かめた範囲と、確かめていないこと

- Linux (WSLg・WebKitGTK) で、`tauri/custom-protocol` を付けて組んだバイナリを `ugoe-viewer avatar.ugoe dress-up/cat-ears.ugoemod` (相対パス) で起動し、
  画面が `initial_files` を呼んで、2 つの絶対パスが `readable-files.json` に入った (= 読めるパスに加わった) ことを確かめた (XDG_DATA_HOME を一時ディレクトリにして)。
  描画結果の目視はしていない。
- **macOS の `RunEvent::Opened` の分岐 (`#[cfg(target_os = "macos")]` の数行) は、この環境では型検査もしていない。** `just cross-check` の `apps` は
  `x86_64-pc-windows-gnu` だけで、macOS は Apple の SDK が要るので CI の `rust (macos)` ジョブ (`just lint-group apps`) が受け持つ
  (手元で `cargo clippy --target aarch64-apple-darwin -p ugoe-viewer` を試したが、objc2-exception-helper の C のコンパイルで止まった)。
  分岐の外 (`paths_from_urls`・`open_from_os`) は全 OS でコンパイルし、Linux でテストした。`just cross-check apps` (Windows) は通った。
- 確かめていない: Windows・macOS の実機でのダブルクリック・「このアプリで開く」・起動中に開いたとき、インストーラが種類を登録すること、
  Linux の .deb が `.desktop` の `MimeType` と `Exec` の引数 (`%F` など) を書いて関連付けが効くか (MIME の定義 (shared-mime-info) は入れていない)。
  docs/manual-check.md の V08 と T0731 に足した。

### 起票の候補

- Windows・Linux で起動中に開いたとき、新しいウィンドウではなく今のウィンドウで開く (tauri-plugin-single-instance で 2 つ目の起動の引数を
  1 つ目へ渡し、`open_from_os` を通す)。
- Linux の .deb に MIME の定義 (`/usr/share/mime/packages/ugoe.xml`) を入れ、ファイルマネージャで `.ugoe` を種類で見分けられるようにする
  (V08 の L で関連付けが効かなければ)。
