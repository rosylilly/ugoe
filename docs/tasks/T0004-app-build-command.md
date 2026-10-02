---
id: T0004
title: デスクトップアプリのビルドを 1 つのコマンドにまとめる (app:build TARGET=... PLATFORM=...)
status: todo
milestone: M8
crates: [justfile, scripts, apps/editor, apps/viewer, apps/studio, .github/workflows, docs]
depends_on: [T0807]
assignee:
---

# T0004: アプリのビルドの入口をまとめる

## 目的

利用者の要望。いまはアプリごと・OS ごとにビルドの入口が割れている (`just editor-build`・`just editor-build-windows`・`just viewer-build`・`just studio-build`。
Windows 向けのクロスビルドはエディタにしか無い)。`just app:build TARGET=editor,viewer,studio PLATFORM=windows,macos,linux` のような 1 つの入口にまとめる。

## 受け入れ条件

- [ ] `just app:build` (または just の構文で同等のもの) で、アプリ (editor・viewer・studio、複数可・既定はすべて) と OS (windows・macos・linux、複数可・既定は今の OS) を選んでビルドできる。
      just のレシピの名前と引数の書き方は just の制約に合わせて決め、justfile と README に書く
- [ ] 今の OS 向けは tauri build、Linux (WSL2 を含む) からの Windows 向けは scripts/editor-windows.mjs の仕組み (cargo-xwin・NSIS) を 3 つのアプリに広げる。
      スタジオの仮想カメラの DLL (T0807) と、エディタのサイドカー (ugoe-mcp) のような アプリごとの違いは 1 か所の設定で持つ
- [ ] 作れない組み合わせ (Linux から macOS など) は、理由と代わりの方法 (macOS の上で実行する・CI の行列) を出して止まる。足りない道具は入れ方を示す
- [ ] 既存の `just editor-build` などは、新しい入口を呼ぶ薄い別名として残す (CI の editor-build.yml などを壊さない)。CI の行列を新しい入口に寄せる
- [ ] 成果物の置き場所を 1 か所にまとめて一覧を出す
- [ ] `just ci` が通る。Windows 向けのクロスビルドを 3 つのアプリで実際に通す (作業の環境で)

## 参考

- justfile のビルドのレシピ、scripts/editor-windows.mjs、apps/*/README.md、.github/workflows/editor-build.yml

## メモ
