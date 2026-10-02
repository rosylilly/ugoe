---
id: T0801
title: スタジオ: アプリの骨組み (操作ウィンドウと出力ウィンドウ)
status: todo
milestone: M8
crates: [apps/studio, scripts, justfile, commitlint.config.mjs, .github/workflows, docs]
depends_on: [T0800, T0901, T0902, T0904]
assignee:
---

# T0801: スタジオ: アプリの骨組み (操作ウィンドウと出力ウィンドウ)

## 目的

apps/studio を作り、モデルを出力ウィンドウに出して操作ウィンドウから動かせるところまで作る。

## 受け入れ条件

- [ ] Tauri v2 + Svelte 5 のアプリ (GPL-3.0-only)。@ugoe/player・@ugoe/ui・@ugoe/i18n を使う
- [ ] 操作ウィンドウと出力ウィンドウ。出力ウィンドウは枠なし・背景の透過・最前面 (切り替え)・大きさの指定 (1920x1080 などのプリセット)
- [ ] 2 つのウィンドウの状態の受け渡し (操作 → 出力)。出力ウィンドウだけが描画する
- [ ] `just studio`、CI のフロントエンドの検査、ライセンスの検査、commitlint の scope `studio`、AGENTS.md の表
- [ ] `just ci` とスタジオのビルドが通る

## 参考

- ADR 0028、docs/studio.md

## メモ
