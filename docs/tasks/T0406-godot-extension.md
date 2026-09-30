---
id: T0406
title: Godot GDExtension
status: todo
milestone: M7
crates: [bindings/godot, docs]
depends_on: []
assignee:
---

# T0406: Godot GDExtension

## 目的

godot-rust で `ugoe` を使う GDExtension を作る。

## 受け入れ条件

- [ ] サンプルシーンでモデルが動く
- [ ] Godot のノードとしてパラメータを設定できる
- [ ] ライセンスの扱い (利用者の決定、2026-10-01): `bindings/godot` は crates/ の外の独立したクレートにし、バインディング自体は MIT のまま、godot-rust (gdext、MPL-2.0) に依存してよい。ワークスペースの外に置くか、ライセンスの検査の対象から外す方法を決め、ADR 0026 に書く (MPL はファイル単位なので、ugoe 側のコードは MIT のまま)
- [ ] `just ci` が通る

## 参考

- `docs/portability.md`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
