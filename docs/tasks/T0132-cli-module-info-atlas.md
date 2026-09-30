---
id: T0132
title: CLI: `ugoe info` でモジュールの概要を出す・モジュールのアトラス化・見本のまとまりの設定を共有する
status: todo
milestone: M4
crates: [ugoe-cli, ugoe-format, ugoe-ai, docs]
depends_on: [T0131, T0716]
assignee:
---

# T0132: CLI: モジュールの概要とアトラス化

## 目的

T0131・T0716 の後続の候補をまとめる。

## 受け入れ条件

- [ ] `ugoe info <m>.ugoemod` でモジュールの概要 (名前・スロット・条件・要素の数・テクスチャ) を出す
- [ ] モジュールのテクスチャのアトラス化 (`build --atlas` をモジュールでも使えるようにする。モジュールの単位でまとめる。docs/spec/file-formats.md のアトラス化の節に書いた後続の課題)
- [ ] 法線マップ付きの見本 (sample/lit.rs) のまとまりの設定 (GROUPS) と、`ugoe_ai` の既定のまとまりが同じ値を別々に持っている。共有するか、別のままにする理由を書く (共有すると ugoe-ai の既定を変えたときに見本も変わる)
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0131-cli-build-normal-maps.md`・`docs/tasks/T0716-sample-lit-shared-inflate.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
