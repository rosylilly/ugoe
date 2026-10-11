---
id: T0088
title: "editor-build.yml の初回の失敗を直す: エディタの fileMenu の名前の衝突 (Windows・macOS) と、スタジオの Windows のインストーラが仮想カメラの DLL を見つけない"
status: done
milestone: Release
crates: [apps/editor, apps/studio, .github/workflows, scripts, docs]
depends_on: []
assignee: claude-worker-T0088
---

# T0088: editor-build.yml の初回の失敗を直す

## 目的

2026-10-11 に初めて手で起動した editor-build.yml (run 38098550160、main d7b0aaeb) で、9 つのビルドのうち 3 つが落ちた (Linux はすべて通った)。

1. **エディタ (macOS・Windows):** `"FileMenuFlow" is not exported by "src/lib/fileMenu.svelte"`。apps/editor/src/lib に `FileMenu.svelte` (部品) と `fileMenu.svelte.ts` (runes のモジュール) があり、
   `import … from "./fileMenu.svelte"` が大文字と小文字を区別しないファイルシステムで `FileMenu.svelte` に解決される。Linux だけで動く CI と手元では見つからない。
2. **スタジオ (Windows):** NSIS のフック (apps/studio/src-tauri/windows/hooks.nsh) が `target/vcam/x86_64-pc-windows-msvc/release/ugoe_studio_vcam.dll` を見つけず、makensis が止まる
   (`File: ... -> no files found`)。`pnpm vcam` (build-vcam.mjs) が CI では作らないか、違う場所に作っている。

## 受け入れ条件

- [x] 1: 名前の衝突を無くした (モジュールの名前を変えるなど)。同じ種類の衝突 (大文字と小文字だけが違うファイル名) がリポジトリの他の所に無いかを調べ、再び入らないよう検査を足した
      (例: scripts/ の検査で、同じディレクトリに大文字小文字を無視して同じ名前 (拡張子の `.svelte` と `.svelte.ts` の import の解決も含む) が無いこと。`just ci` に入れる)
- [x] 2: 原因を確かめ (CI のログで `pnpm vcam` が動いたか・出力先・T0084 の署名の変更との関係)、Windows のスタジオのインストーラが DLL を入れられるよう直した
- [x] 手元で `DRY_RUN=1 just app::build` と、できる範囲の検査 (`just cross-check`・`just vcam-check`) が通る
- [x] `just ci` が通る
- [ ] (オーケストレーター) push の後に editor-build.yml を workflow_dispatch で流し直し、9 つすべて通ることを確かめる

## 参考

- `gh run view 38098550160 --log-failed`、apps/editor/src/lib (FileMenu.svelte・fileMenu.svelte.ts)、apps/studio/scripts/build-vcam.mjs・src-tauri/tauri.windows.conf.json・windows/hooks.nsh、
  scripts/app-build.mjs、.github/workflows/editor-build.yml、T0084 (署名。build-vcam.mjs を触った)

## メモ

- オーケストレーターが初回の editor-build.yml の失敗から起票した (利用者の方針: CI の失敗は起票してすぐ直す)。
- **1 (エディタ):** `apps/editor/src/lib/fileMenu.svelte.ts` を `fileMenuFlow.svelte.ts` に (テストも `fileMenuFlow.test.ts` に) 変え、import と
  docs/editor.md を直した。検査 `scripts/check-case-collisions.mjs` (`just case-check`。`just ci` と ci.yml の rust-checks に入れた。
  docs/testing.md の表に載せた) は `git ls-files` の名前を、同じディレクトリで小文字にして比べる。スクリプトの拡張子 (`.ts`・`.js` など) を取った
  import の名前も比べるので、`FileMenu.svelte` と `fileMenu.svelte.ts` がぶつかる。ディレクトリの名前も比べる。0.1 秒で終わる。
  今のリポジトリで見つかったのはこの 1 組だけだった。
- **2 (スタジオ):** CI のログでは `pnpm vcam` (build-vcam.mjs) は動き、cargo は成功していた (出力先も正しい。手元の cargo-xwin でも
  `target/vcam/x86_64-pc-windows-msvc/release/ugoe_studio_vcam.dll` にできる)。原因は hooks.nsh のパスの区切り: `${__FILEDIR__}` (Windows では `\`)
  に `/../../../../target/...` をつないでいた。Windows の makensis は File のパスを最後の `\` でディレクトリとファイル名に分ける
  (NSIS の Source/util.cpp の `get_dir_name` は `PLATFORM_PATH_SEPARATOR_STR` だけで分ける。https://github.com/kichik/nsis/blob/master/Source/util.cpp)
  ので、`src-tauri` の中から `windows/../../../../target/.../ugoe_studio_vcam.dll` という名前を探して見つからない。Linux の makensis は `\` を `/` に直すので、
  手元のクロスビルドでは起きなかった。区切りを `\` にし (Linux の makensis 3.09 で、実際の hooks.nsh と cargo-xwin で作った DLL から
  インストーラができることを確かめた)、hooks.nsh の `!define`・`File` のパスに `/` が無いことを scripts/app-build.test.mjs で検査する。
  build-vcam.mjs は cargo の後に DLL が無ければ止まるようにした。T0084 (署名) の変更は関係しない (失敗した run は T0084 の前で、
  T0084 は DLL の場所を変えていない)。
- 範囲外の小さな変更: justfile (`case-check` のレシピと `ci` への追加)。
- 確かめていないこと: Windows と macOS の実際のビルド (オーケストレーターの editor-build.yml の流し直しで確かめる)。
