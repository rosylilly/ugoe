---
id: T0801
title: スタジオ: アプリの骨組み (操作ウィンドウと出力ウィンドウ)
status: todo
milestone: M8
crates: [apps/studio, packages, AGENTS.md, scripts, justfile, commitlint.config.mjs, .github/workflows, docs]
depends_on: [T0800, T0901, T0902, T0904, T0133]
assignee:
---

# T0801: スタジオ: アプリの骨組み (操作ウィンドウと出力ウィンドウ)

## 目的

apps/studio を作り、モデルを出力ウィンドウに出して操作ウィンドウから動かせるところまで作る。

## 受け入れ条件

- [ ] Tauri v2 + Svelte 5 のアプリ (GPL-3.0-only)。@ugoe/player・@ugoe/ui・@ugoe/i18n を使う
- [ ] 操作ウィンドウと出力ウィンドウ。出力ウィンドウは枠なし・背景の透過・最前面 (切り替え)・大きさの指定 (1920x1080 などのプリセット)
- [ ] ADR 0028 の指揮役: バックエンドに物理を切った Puppet を 1 つ置き、刻みごとに {seq, dt, values} を配る。出力ウィンドウは受け手として全フレームを順に適用し、自分で物理を回して描く
- [ ] ADR 0028 の 10. の一致の検査 (記録したフレームの列を受け手に適用し、物理を有効にした 1 つの Puppet と頂点・不透明度を比べる)
- [ ] @ugoe/player に受け手のモード (値と dt を受けて進める) を足す
- [ ] `just studio`、CI のフロントエンドの検査、ライセンスの検査、commitlint の scope `studio`、AGENTS.md の表とライセンスの行に apps/studio
- [ ] `just ci` とスタジオのビルドが通る

## 参考

- ADR 0028、docs/studio.md

## メモ
