---
id: T0733
title: "ビューア: Windows・Linux で起動中に開いたファイルを今のウィンドウで開く (single-instance)"
status: done
milestone: M9
crates: [apps/viewer, docs]
depends_on: []
assignee: claude-worker-T0733
---

# T0733: ビューア: Windows・Linux で起動中に開いたファイルを今のウィンドウで開く (single-instance)

## 目的

T0732 でビューアを OS の「このアプリで開く」に対応させた。macOS は起動中に開くと同じウィンドウで開く (`RunEvent::Opened`) が、
Windows・Linux は新しいプロセスが起動して新しいウィンドウになる (T0732 の候補)。`tauri-plugin-single-instance` で 2 つ目の起動の引数を 1 つ目へ渡し、
`open_from_os` を通して今のウィンドウで開く。新しい外部の依存なので、ライセンス (`just licenses`) と同梱の表記 (`third-party-notices.txt`) を確かめる。

## 受け入れ条件

- [x] Windows・Linux で 2 つ目の起動が 1 つ目にファイルを渡して終わり、1 つ目のウィンドウで開く (前に出す)。引数の解釈は今の起動時の経路と同じ
  (Linux (WSLg) で確かめた。Windows は型検査 (`just cross-check apps`) まで。下のメモ)
- [x] 引数から開くファイルを選ぶ処理のテストがある
- [x] docs/manual-check.md の V08 を、起動中に開いたときの期待 (すべての OS で同じウィンドウ) に直した
- [x] 実機で確かめていないことをメモに書いた (T0731 の手動確認に回す)
- [x] `just ci` が通る

## 参考

- T0732 のメモ (起票の候補)
- apps/viewer/README.md、docs/manual-check.md

## メモ

- T0032 で起票した (元の候補: T0732 (single-instance))。

### やったこと (T0733)

- 依存に `tauri-plugin-single-instance` 2 (2.5.2、Apache-2.0 OR MIT。Tauri の公式のプラグイン) を足した (ルートの `[workspace.dependencies]`
  とビューアのクレート)。Linux の実装が使う zbus などは既にロックファイルにあり、増えたクレートはこれ 1 つ。`just licenses` は通る。
  `third-party-notices.txt` は JS のバンドルだけから作るもの (docs/release.md の「同梱物のライセンスの表記」。Rust のクレートは
  まだ扱っていない) で、このプラグインに JS の API は無い (使わない) ので変わらない (`just licenses` の突き合わせも通る)。
- `lib.rs`: `run` の最初のプラグインに `tauri_plugin_single_instance::init(on_second_instance)` を置いた。`on_second_instance` は
  2 つ目の引数から開くファイルを選び (`paths_from_second_instance`: 先頭の実行ファイルのパスを除いて `paths_from_args` に渡す)、
  `open_from_os` を通し、ウィンドウ `main` を `unminimize`・`show`・`set_focus` で前に出す。`paths_from_args` は今のディレクトリを
  引数 `cwd` で受けるようにした (2 つ目の相対パスは 2 つ目のプロセスの今のディレクトリから解くため。起動時は `current_dir()` を渡す)。
  プラグインはどの OS でも登録する (macOS でも、コマンドラインから 2 度起動したときにまとまる。macOS の「このアプリで開く」は
  今までどおり `RunEvent::Opened`)。
- テスト: `opens_files_from_a_second_instance` (先頭は開かない・`-` と読めない拡張子を除く・相対パスは 2 つ目の cwd から・ファイルが
  無ければ空)。`opens_models_and_modules_from_the_arguments` を `cwd` を渡す形にした。
- 文書: README の「OS から開く」の「起動中に別のファイルを開いたとき」、docs/manual-check.md の V08 の期待 (どの OS でも同じウィンドウで、
  前に出る)、docs/manual/dress-up.md の「ビューアで付け外しする」。

### 確かめた範囲と、確かめていないこと (T0731 の手動確認に回す)

- Linux (WSLg・WebKitGTK、D-Bus のセッションバスあり): `tauri/custom-protocol` を付けて組んだバイナリを `w1/` で `ugoe-viewer avatar.ugoe`
  と起動し、D-Bus に `app.ugoe.viewer.SingleInstance` が出ることを確かめた。別のディレクトリ `w2/` から
  `ugoe-viewer dress-up.ugoe dress-up/cat-ears.ugoemod` (相対パス) で 2 つ目を起動すると、0.26 秒で終了コード 0 で終わり、プロセスは 1 つのまま、
  1 つ目の `readable-files.json` に `w2/` からの 2 つの絶対パスが加わった (= `open_from_os` がイベントで送った)。引数なしの 2 つ目も
  終了コード 0 で終わった。ウィンドウを前に出すことのエラーは出なかった。**画面にモデルが入れ替わって描かれたかと、ウィンドウが実際に
  前に出たかの目視はしていない** (WSLg のウィンドウマネージャでは前に出す挙動も実機の GNOME などと違いうる)。
- Windows: `just cross-check apps` (x86_64-pc-windows-gnu の clippy) が通っただけで、実機では動かしていない。エクスプローラのダブルクリックで
  2 つ目が渡して終わること・ウィンドウが前に出ること (Windows はフォアグラウンドの制限で、タスクバーが点滅するだけのことがある) を V08 で見る。
- macOS: 型検査もしていない (CI の `rust (macos)` が受け持つ。T0732 と同じ)。`.app` を 2 度起動するのは LaunchServices が止めるので、
  コマンドラインから 2 度起動したときだけプラグインが効く。
- プラグインの制約 (上流): 2 つ目は引数を `std::env::args` で集めるので、UTF-8 でないファイル名 (Linux) を渡すと 2 つ目が失敗する
  (README に書いた)。D-Bus のセッションバスが無い Linux では、プラグインは何もせず、今までどおり別のウィンドウになる。
  D-Bus の名前は識別子から決まるので、同じ利用者が別の場所に置いた開発版と入れたものも 1 つにまとまる。

### 起票の候補

- なし (Linux の .deb の MIME の定義は T0732 の候補のまま)。
