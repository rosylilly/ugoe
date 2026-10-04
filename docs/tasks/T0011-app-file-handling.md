---
id: T0011
title: アプリのファイルの扱いの残り (通常のファイルだけを読む・保存の拡張子・読めなかった設定を上書きしない)
status: done
milestone: M9
crates: [apps/editor, apps/viewer, apps/studio, ugoe-cli, docs]
depends_on: []
assignee: claude-worker-T0011
---

# T0011: アプリのファイルの扱いの残り (通常のファイルだけを読む・保存の拡張子・読めなかった設定を上書きしない)

## 目的

M9 のコードレビューの横断の所見のうち、3 つのアプリにまたがってまだ確かめていないもの (まとめは docs/workflow/code-review.md#m9-の結果)。
アプリごとの同期のコマンド・CSP・一時ファイル経由の保存は、それぞれのタスク (エディタ T0685〜T0687、ビューア T0726〜T0728、スタジオ T0819・T0820) で扱う。

- **外から渡されたパスを `fs::read` する (T0212・T0529・T0530):** 装置や FIFO を読むと終わらない。`ugoe-mcp` と `ugoe-headless` は通常のファイルかと
  大きさを確かめるようにしたが、エディタ・スタジオの IPC で受けるパス (PSD・モーション・物理の JSON・背景の画像など) は確かめていない。
  線引きは T0530 の案に揃える: サーバやアプリの IPC で受けるパスは通常のファイルだけ、利用者が自分で渡す CLI の引数は FIFO を読めるまま。
- **保存のダイアログの拡張子 (T0724):** Linux のダイアログはフィルタの拡張子を付けずにパスを返すことがある。スタジオの `pickSavePath` (プロファイル) は付けていない。
- **読めなかった設定のファイルを既定で上書きする (T0814):** スタジオで直した形が、エディタ・ビューアの覚える設定・最近の一覧にもないか。

## 受け入れ条件

- [x] 3 つのアプリの IPC で受けたパスを読む箇所が、通常のファイルだけを読み、大きさに上限がある (共通の小さな関数を使う。テストを足す)
- [x] 保存のダイアログを使うところすべてで、返ったパスに拡張子が無ければ付ける (エディタの `withExtension` と同じ形)
- [x] エディタとビューアの設定のファイル (覚える設定・最近の一覧) が、読めなかったときに既定で上書きされないことを確かめ、要れば直した
- [x] docs/editor.md・docs/studio.md・apps/viewer/README.md の該当の節を直した
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0212・T0529・T0530・T0724・T0814 のメモの「横断」

## メモ

### やったこと

- **通常のファイルだけを読む:** 3 つのアプリに同じ形の小さな関数 `read_regular_file(path, max)` (`src-tauri/src/read_file.rs`) を置いた。
  開く前に `fs::metadata` で通常のファイル (リンクは辿る) かと大きさを見て、開いた後にも見直し、上限 + 1 バイトまでに限って読む。
  アプリどうしで共有するクレートが無いので、`atomic_write.rs` と同じく写しを持つ (直すときは 3 つとも直す、とモジュールの説明に書いた)。
  - エディタ: PSD (`import_psd`・`new_project_from_psd`・AI の提案の `read_tree_with`。2 GiB)・`read_module_file` (512 MiB)・
    `assign_texture_normal_map` (64 MiB)。エラーは今の `editor.io` (文に `not a regular file` / `file is too large`)。新しいコードは足していない。
  - ビューア: `read_model_file` (前から上限はあったが、FIFO は `File::open` で止まり、ディレクトリも開いていた)。`file.read_failed`。
  - スタジオ: 舞台の `read_asset` (モデル・モジュール・背景の画像。外部 API の `dressup.list` もこれを通る) と `Profile::read`。
  - テスト: 共通の関数 (通常のファイル・上限・無いファイル・ディレクトリ・`/dev/zero`・FIFO・リンク) と、各アプリの呼び出し側。
- **保存のダイアログの拡張子:** スタジオの `pickSavePath` (プロファイル) に `.json` を、エディタの Claude Desktop の拡張の保存
  (`ConnectAgents.svelte`。バックエンドは `.mcpb` でない名前を `mcp.not_mcpb_file` で断るので、Linux では保存できなかった) に `.mcpb` を付けた。
  ほかの保存のダイアログ (エディタのファイルメニュー・スクリーンショット・トラッキングの記録、ビューアの `save_file_dialog`) は付けていた。
- **読めなかった設定:** エディタの `editor-settings.json`・`ai-provider.json`・`mcp-bridge.json` は、読めなければ既定のまま起動し、
  起動では書かない (利用者が変えたときに書く。スタジオの T0814 の直し方と同じ) ことを確かめ、テストを足した。
  ビューアの `readable-files.json` は、読めない (権限など) ときに空から始め、次にファイルを選んだときに 1 件だけの一覧で上書きしていたので、
  そのセッションでは書かないようにした (テストを足した。壊れた JSON はスタジオと同じく次の操作で書き直す)。
  ビューアのそのほかの設定 (言語・`@ugoe/player` の覚えるもの) は localStorage で、起動では書かない (読めなければ覚えていない扱い)。
- **CLI は変えていない:** T0530 の線引きのとおり、利用者が自分で渡す CLI の引数は FIFO・`/dev/stdin` を読めるままにする。
- docs/editor.md (ファイルダイアログ・書き出しの節)・docs/studio.md (シーンとプロファイルの操作)・apps/viewer/README.md を直した。

### やらなかったこと・確かめていないこと

- `ugoe-editor-core` の `Project::load` (`.ugoeproj` の中の `model.json`・PNG など) と本体の `.ugoe` の読み込みは `crates:` の外なので
  触っていない (`open_project`・`read_module_base`・`open_module_with_base` が画面から受けたパスで読む)。プロジェクトの中のファイルが FIFO なら止まる。
- 見てから開くまでの間に FIFO へ差し替えられると、開くところで止まりうる (`O_NONBLOCK` で開くには libc か unsafe が要る。窓は小さいので受け入れた)。
- Windows・macOS では動かしていない (装置・FIFO の検査は `cfg(unix)` のテスト。ディレクトリの検査はどの OS でも通るはず)。

### 起票の候補

- editor-core の `Project::load` / `load_base` / モジュールの読み込みと `ProjectFingerprint` でも、通常のファイルだけを読み大きさに上限を設ける。
- `@ugoe/player` の `ModelMemory` は、版 (`version`) が今と違う保存値を空として扱い、次に覚えるときに上書きする (新しい版のビューアで覚えたものを
  古い版で開くと失う)。スタジオの設定と同じ方針 (操作したら書く) なので今は受け入れているが、版が上がるときに方針を決める。
- アプリの Rust で共有する小さなクレート (一時ファイル経由の保存・通常のファイルの読み込み・パスの許可) を作るか。今は 3 つのアプリに写しがある。

- `just ci` は通った (main に追従済み、2026-10-04)。
