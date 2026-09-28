---
id: T0625
title: WSL (Linux) から Windows 向けのエディタをビルドする
status: in-progress
milestone: M2
crates: [justfile, scripts, apps/editor (設定・README), .github/workflows]
depends_on: [T0621]
assignee: claude-worker-T0625
---

# T0625: WSL (Linux) から Windows 向けのエディタをビルドする

## 目的

T0621 では「Windows 向けは Windows 上でビルドする」としたが、Tauri は Linux から Windows 向けの NSIS インストーラをクロスビルドできる (署名なし)。
WSL2 で開発している人が、Windows に切り替えずに Windows 向けのエディタを作って試せるようにする。

## 受け入れ条件

- [ ] `just editor-build-windows` (仮) で、Linux (WSL2) から Windows x86_64 向けのエディタ (NSIS のインストーラと実行ファイル) を作れる。必要な道具 (例: `cargo-xwin`、LLVM の clang / lld、NSIS の `makensis`、`x86_64-pc-windows-msvc` ターゲット) の導入手順を README に書く
- [ ] 手元 (WSL2) で実際にビルドし、できたファイルと大きさを報告する。可能なら WSL から Windows 側で起動して確かめる (できなければ、起動の確認は利用者にお願いする)
- [ ] MSI はクロスビルドできないこと、署名していないことを README に書く
- [ ] 必要なら CI (Linux のランナー) でもクロスビルドを試す
- [ ] `just ci` が通る

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
