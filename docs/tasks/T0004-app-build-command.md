---
id: T0004
title: デスクトップアプリのビルドを 1 つのコマンドにまとめる (app:build TARGET=... PLATFORM=...)
status: done
milestone: M8
crates: [justfile, scripts, apps/editor, apps/viewer, apps/studio, .github/workflows, docs]
depends_on: [T0807, T0809]
assignee: claude-worker-T0004
---

# T0004: アプリのビルドの入口をまとめる

## 目的

利用者の要望。いまはアプリごと・OS ごとにビルドの入口が割れている (`just editor-build`・`just editor-build-windows`・`just viewer-build`・`just studio-build`。
Windows 向けのクロスビルドはエディタにしか無い)。`just app:build TARGET=editor,viewer,studio PLATFORM=windows,macos,linux` のような 1 つの入口にまとめる。

## 受け入れ条件

- [x] `just app:build` (または just の構文で同等のもの) で、アプリ (editor・viewer・studio、複数可・既定はすべて) と OS (windows・macos・linux、複数可・既定は今の OS) を選んでビルドできる。
      just のレシピの名前と引数の書き方は just の制約に合わせて決め、justfile と README に書く
- [x] 今の OS 向けは tauri build、Linux (WSL2 を含む) からの Windows 向けは scripts/editor-windows.mjs の仕組み (cargo-xwin・NSIS) を 3 つのアプリに広げる。
      スタジオの仮想カメラの DLL (T0807)・macOS の Camera Extension の束 (T0809。`pnpm vcam:macos` と tauri.macos-vcam.conf.json) と、エディタのサイドカー (ugoe-mcp) のような アプリごとの違いは 1 か所の設定で持つ
- [x] 作れない組み合わせ (Linux から macOS など) は、理由と代わりの方法 (macOS の上で実行する・CI の行列) を出して止まる。足りない道具は入れ方を示す
- [x] 既存の `just editor-build` などは、新しい入口を呼ぶ薄い別名として残す (CI の editor-build.yml などを壊さない)。CI の行列を新しい入口に寄せる
- [x] 成果物の置き場所を 1 か所にまとめて一覧を出す
- [x] `just ci` が通る。Windows 向けのクロスビルドを 3 つのアプリで実際に通す (作業の環境で)

## 参考

- justfile のビルドのレシピ、scripts/editor-windows.mjs、apps/*/README.md、.github/workflows/editor-build.yml

## メモ

- **入口:** `just app::build TARGET=editor,viewer,studio PLATFORM=windows,macos,linux` (`just app build ...` でも同じ)。
  just のレシピの名前に `:` は使えないので、モジュール (`justfile` の `mod app`、`scripts/app.just`) の `app::build` にした。
  `TARGET=` は just の変数の上書きではなくレシピの引数としてスクリプトが読む (just の変数の上書きはモジュールに届かないため、
  レシピ名の後ろに書く)。ほかに `VCAM=1` (macOS のスタジオに Camera Extension)・`DRY_RUN=1`。`KEY=値` でない引数は tauri build へ。
- **実体:** `scripts/app-build.mjs` (テストは `scripts/app-build.test.mjs`、`just app-build-check` を `just ci` と ci.yml の
  rust-checks に追加)。アプリごとの違いは `APPS` だけに持つ。作れない組み合わせは 1 つも作らずに理由と代わり (その OS で実行・
  CI の Editor build) を出して止まる。足りない道具は入れ方を出す。WSL では Linux 向けのビルドで PATH から `/mnt/` を外す
  (AppImage の linuxdeploy のため。前は README で手で行う手順だった)。PowerShell が `a,b` を 2 つの引数に分ける件は、続きとして読む。
- **成果物:** `target/dist/<アプリ>/<OS>/` に、製品名と版で始まりそのビルドでできたインストーラだけを写して一覧を出す。
- **別名:** `editor-build`・`editor-build-windows`・`viewer-build`・`studio-build` は `node scripts/app-build.mjs TARGET=...` の別名。
  `scripts/editor-windows.mjs` は app-build.mjs に吸収して消した。
- **CI:** editor-build.yml の行列を `platform: [linux, windows, macos]` + `label` (成果物名とキャッシュのキーは前と同じ
  `ugoe-<アプリ>-<linux-x86_64 など>`) にし、`just app::build TARGET=... PLATFORM=...` と `target/dist/<アプリ>/<OS>/` の upload にした。
- **確かめたこと (WSL2):** `just app::build PLATFORM=windows` で 3 アプリの NSIS を作れた (editor 28.5 MiB・viewer 21.0 MiB・
  studio 19.8 MiB。スタジオは Tauri が tauri.windows.conf.json をクロスビルドでも自動で重ね、`pnpm vcam` で DLL を作った)。
  `just app::build TARGET=viewer --bundles deb` で deb を作れた。Linux から macOS を指定すると何も作らずに止まる。`just ci` が通る。
- **確かめていないこと:** Windows と macOS の実機での実行 (ネイティブビルド・PowerShell の引数の分割・VCAM=1)、CI の Editor build の
  実行 (push 後の workflow_dispatch で確かめる)、できたインストーラを Windows に入れて起動すること、AppImage と rpm。
- **後続の候補:** AGENTS.md のコマンドの表に `just app::build` を足す (範囲外なので触っていない)。macOS から Windows 向けの
  クロスビルド (Tauri は対応しているが未検証なので止める扱いにした)。

