---
id: T0405
title: Unity パッケージ
status: todo
milestone: M7
crates: [bindings/unity]
depends_on: [T0401]
assignee:
---

# T0405: Unity パッケージ

## 目的

`ugoe-ffi` を使う UPM パッケージを作り、Unity のメッシュとマテリアルで描画する (段階 B)。

## 受け入れ条件

- [ ] サンプルシーンでモデルが動く
- [ ] 変更フラグを使って転送を省く
- [ ] Windows / macOS / Linux / Android / iOS 向けのネイティブライブラリをビルドする CI がある
- [ ] `just ci` が通る

## 参考

- `docs/portability.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
