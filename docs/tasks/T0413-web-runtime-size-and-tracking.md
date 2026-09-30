---
id: T0413
title: npm: wasm の読み込みコードの縮小・エディタとビューアの wasm のプロファイル・トラッキングの平滑化
status: todo
milestone: M7
crates: [bindings/web, ugoe-wasm, ugoe-format, docs]
depends_on: [T0410]
assignee:
---

# T0413: npm パッケージの後続

## 目的

T0410 の後続の候補をまとめる。

## 受け入れ条件

- [ ] MessagePack (`rmp_serde`) の読み込みコード (約 0.8MB) の大きさを見直す。読み込みの結果と互換性テストは変えない
- [ ] エディタとビューアの wasm にも、パッケージと同じビルドのプロファイル (大きさ優先・毎フレーム動くクレートは速さ優先) を使うかを決める。使うなら数字を書く
- [ ] トラッキングの平滑化とキャリブレーションを、MIT の別の部品として `bindings/web` に置くかを決める (GPL の `packages/face-tracking` は使えない。ADR 0025)
- [ ] `assets/samples/` の見本にモーションを入れるかを決める (いまは `bindings/web` の見本だけが生成している)
- [ ] `just ci`・`just wasm-check` が通る

## 参考

- `docs/tasks/T0410-web-runtime-release.md` のメモ、`docs/adr/0025-web-runtime-package.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
