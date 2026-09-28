---
id: T0203
title: GPU テストの CI (lavapipe)
status: done
milestone: M1
crates: [.github/workflows, justfile]
depends_on: [T0201]
assignee: claude-worker-T0203
---

# T0203: GPU テストの CI (lavapipe)

## 目的

GitHub Actions の Linux ランナーで Mesa の lavapipe を入れ、`just test-gpu` を実行するジョブを追加する。

## 受け入れ条件

- [ ] CI で GPU テストが実行される
- [x] docs/testing.md を更新する
- [x] `just ci` が通る

## 参考

- `docs/testing.md#描画テスト`

## メモ

- `ci.yml` に `gpu` ジョブを追加した。matrix は `vulkan` (lavapipe) と `gl` (llvmpipe / EGL、WebGL2 相当の経路の確認)。`WGPU_BACKEND` でバックエンドを絞り、Vulkan ローダーは `VK_DRIVER_FILES` / `VK_ICD_FILENAMES` で `lvp_icd*.json` だけに限定し、GL は `LIBGL_ALWAYS_SOFTWARE=1` で llvmpipe に固定する。Tauri のシステム依存は入れない。
- `just test-gpu` は `scripts/gpu-test-args.mjs` (`cargo metadata --no-deps`) で `gpu-tests` feature を持つクレートを集めて 1 回の `cargo test` にまとめる。T0204 が ugoe-cli に feature を足せば自動で対象になる。`just test-gpu -- --nocapture` でアダプタ情報が出る (CI はこれで実行)。
- 手元 (WSL2、Mesa 26.0.4) で、ジョブと同じ環境変数と `just test-gpu -- --nocapture` を両バックエンドで実行し、25 件すべて通ることを確認した。actionlint 1.7.12 も通る。
- 未確認: 「CI で GPU テストが実行される」は push 後に GitHub Actions で確認する必要がある。確認点は、ubuntu-latest (24.04) の Mesa で lavapipe の ICD ファイル名 (`lvp_icd.x86_64.json` 想定) がグロブに一致すること、ログのアダプタ行が `llvmpipe` になっていること、gl 側で EGL の surfaceless / device プラットフォームでアダプタが取れること。gl が不安定なら matrix から外すか `continue-on-error` にする。
