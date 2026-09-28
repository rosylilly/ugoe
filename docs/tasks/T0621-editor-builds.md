---
id: T0621
title: エディタの Windows・macOS・Linux 向けビルド
status: done
milestone: M2
crates: [.github/workflows, justfile, apps/editor (設定のみ), docs]
depends_on: []
assignee: claude-worker-T0621
---

# T0621: エディタの Windows・macOS・Linux 向けビルド

## 目的

T0619 の実機確認で、Windows・macOS 向けのエディタのビルドのコマンドがなく、確認できなかった。
Tauri は他の OS 向けのクロスビルドが難しいので、各 OS 上でビルドする手順と、CI で各 OS 向けのインストーラを作る仕組みを用意する。

## 受け入れ条件

- [x] 各 OS でエディタをビルドする手順 (前提パッケージ、wasm-bindgen-cli の導入、`just editor-build`) を `apps/editor/README.md` に書く。Windows では WSL ではなく Windows 上で直接ビルドする手順とする
- [x] `just` のレシピが Windows (PowerShell) でも動く、または Windows 用の代替手順がある (今の justfile は bash 前提)
- [ ] GitHub Actions のワークフローで、Windows・macOS (Apple Silicon と Intel、または universal)・Linux 向けのエディタのインストーラをビルドし、成果物としてダウンロードできる。手動実行 (`workflow_dispatch`) とタグで動く。コード署名は後続とし、署名していないことを README に書く
- [x] Linux 向けのビルドを手元で実行して確かめる。ワークフローは actionlint で検査する
- [x] `just ci` が通る

## 参考

- `docs/tasks/T0619-editor-manual-check.md` のメモ
- `docs/adr/0017-single-runtime-distribution.md`

## メモ

- **Windows 対応の方法:** justfile に `set windows-shell := ["powershell.exe", ...]` を足し、bash の構文を使っていた
  `wasm` と `wasm-bindgen-version` の中身を `scripts/wasm.mjs` (Node) に移した (`crates:` の範囲外だが justfile の実体として必要)。
  PowerShell 7 ではなく、Windows に必ずある Windows PowerShell 5.1 (`powershell.exe`) を使う。
  `setup`・`wasm`・`editor`・`editor-build`・`ci` は 1 行 1 コマンドなので PowerShell でも動く。
  `test-gpu` と `commitlint` は bash の構文のままで、Windows では Git Bash か WSL が要る。
  Windows の実機では未確認 (WSL から `powershell.exe` で引用符・終了コードの扱いだけ確かめた)。
  CI の Windows ジョブが `just editor-build` を PowerShell 経由で通すので、push 後に確かめられる。
- **ワークフロー:** `.github/workflows/editor-build.yml`。macOS は `universal-apple-darwin` (1 つの .dmg で
  Apple Silicon と Intel の両方)。成果物は `ugoe-editor-{linux-x86_64,windows-x86_64,macos-universal}`。
  **GitHub 上で実行して全 OS のビルドが通ることは、push するまで確かめられない** (3 つ目の受け入れ条件は未チェック)。
  タグで動いても GitHub Release は作らない (成果物だけ)。
- **手元の Linux ビルド (WSL2, 2026-09-28):** `.deb` 5.3 MiB、`.rpm` 5.3 MiB、`.AppImage` 82 MiB、実行ファイル 16 MiB。
  WSL では AppImage を作る linuxdeploy が `PATH` 中の `/mnt/c/...` を読めずに落ちる。`PATH` から外せば通る (README に記載)。
- **後続の候補:** コード署名と macOS の公証 (Windows の証明書、Apple Developer ID)。タグでの GitHub Release の作成と
  自動更新 (tauri-plugin-updater)。AppImage の glibc 互換のため、古い Ubuntu のランナーでビルドするか。
