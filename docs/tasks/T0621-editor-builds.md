---
id: T0621
title: エディタの Windows・macOS・Linux 向けビルド
status: in-progress
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

- [ ] 各 OS でエディタをビルドする手順 (前提パッケージ、wasm-bindgen-cli の導入、`just editor-build`) を `apps/editor/README.md` に書く。Windows では WSL ではなく Windows 上で直接ビルドする手順とする
- [ ] `just` のレシピが Windows (PowerShell) でも動く、または Windows 用の代替手順がある (今の justfile は bash 前提)
- [ ] GitHub Actions のワークフローで、Windows・macOS (Apple Silicon と Intel、または universal)・Linux 向けのエディタのインストーラをビルドし、成果物としてダウンロードできる。手動実行 (`workflow_dispatch`) とタグで動く。コード署名は後続とし、署名していないことを README に書く
- [ ] Linux 向けのビルドを手元で実行して確かめる。ワークフローは actionlint で検査する
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0619-editor-manual-check.md` のメモ
- `docs/adr/0017-single-runtime-distribution.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
