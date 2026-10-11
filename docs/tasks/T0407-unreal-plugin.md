---
id: T0407
title: Unreal プラグイン
status: todo
milestone: M7
crates: [bindings/unreal]
depends_on: [T0401]
assignee:
---

# T0407: Unreal プラグイン

## 目的

`ugoe-ffi` を使う Unreal プラグインを作る。

## 受け入れ条件

- [ ] サンプルでモデルが動く
- [ ] ProceduralMesh または独自の SceneProxy で描画する
- [ ] `just ci` が通る

## 参考

- `docs/portability.md`

## メモ

- T0085 (利用者の決定 2026-10-11): M7 の完了の条件から外し、1.0 の後の候補にした (docs/roadmap.md の「1.0 の範囲」、ADR 0035)。
  要望が出るまで着手しない。
