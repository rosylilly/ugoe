---
id: T0625
title: WSL (Linux) から Windows 向けのエディタをビルドする
status: done
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

- [x] `just editor-build-windows` (仮) で、Linux (WSL2) から Windows x86_64 向けのエディタ (NSIS のインストーラと実行ファイル) を作れる。必要な道具 (例: `cargo-xwin`、LLVM の clang / lld、NSIS の `makensis`、`x86_64-pc-windows-msvc` ターゲット) の導入手順を README に書く
- [x] 手元 (WSL2) で実際にビルドし、できたファイルと大きさを報告する。可能なら WSL から Windows 側で起動して確かめる (できなければ、起動の確認は利用者にお願いする)
- [x] MSI はクロスビルドできないこと、署名していないことを README に書く
- [ ] 必要なら CI (Linux のランナー) でもクロスビルドを試す
- [x] `just ci` が通る

## メモ

- **仕組み:** `just editor-build-windows` → `scripts/editor-windows.mjs`。rustup の `x86_64-pc-windows-msvc`
  (rust-toolchain.toml の版)・cargo-xwin・`clang`・`lld-link`・`llvm-rc`・`llvm-lib`・`makensis` を調べ、足りなければ
  入れ方 (apt のパッケージ名など) を示して止まる。そろっていれば
  `pnpm --filter @ugoe/editor tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis` を実行する
  (`just wasm` は beforeBuildCommand で走る)。Windows 上で実行したら `just editor-build` を使うよう促す。
- **手元のビルド (WSL2, Ubuntu, 2026-09-28):** clang 18・lld 18・NSIS 3.09・cargo-xwin 0.21.4。初回 (MSVC CRT/SDK は
  キャッシュ済み) 約 1 分半。`ugoe_0.0.0_x64-setup.exe` 4.05 MiB (NSIS)、`ugoe-editor.exe` 13 MiB (PE32+ GUI)。
  リンクで LNK4099 (CRT の PDB がない) の警告が大量に出るが無害。`CARGO_TARGET_..._RUSTFLAGS` で `/ignore:4099` を
  渡しても消えなかった (cargo-xwin が RUSTFLAGS を上書きするらしい) ので、README に無害と書くに留めた。
- **Windows 側での起動 (WSL から):** 実行ファイルを `%TEMP%` にコピーして起動し、`tasklist /V` でウィンドウ
  タイトル `ugoe` のプロセスが動いていることを確かめた (その後 taskkill で終了)。画面の中身 (WebView2 での描画) と
  インストーラでのインストールは目視していない。 → 利用者の確認が要る (T0619 のチェックリスト)。
- **CI:** クロスビルドのジョブは足さなかった。CI の Windows ランナーのネイティブビルドが MSI と NSIS の両方を作るので、
  NSIS だけのクロスビルドを重ねる意味が薄い。また CI で MSVC CRT/SDK のライセンスに自動で同意することになる。
- **後続の候補:** Windows 向けのコード署名 (Linux からは `bundle.windows.signCommand` で osslsigncode などを使える)。
