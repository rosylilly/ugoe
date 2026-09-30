---
id: T0655
title: 見本のプロジェクトの変換を editor-core にまとめる・プレビューで公開していないスロットに印・物理の無いときのスロットの衝突体
status: todo
milestone: M4
crates: [ugoe-editor-core, ugoe-cli, apps/editor, ugoe, ugoe-runtime, docs]
depends_on: [T0715]
assignee:
---

# T0655: 見本のプロジェクトとスロットの公開の残り

## 目的

T0715 の後続の候補をまとめる。

## 受け入れ条件

- [ ] CLI の `save_projects` と、エディタの `sample.rs` にある見本からプロジェクトへの変換 (約 40 行の写し) を editor-core にまとめる
- [ ] エディタのプレビューの着せ替えで、アバタープロファイルが公開していないスロットに印を付ける (ビューアは T0715 で隠している)
- [ ] スロットが `colliders` で本体の衝突体を公開しているとき、物理の設定を渡さずにモデルだけで作った Puppet では、そのスロットにモジュールを付けられなくなる。扱いを見直す (物理が無ければ衝突体の参照を無視するなど)。仕様 (spec/model.md の Slots と Composition) と合わせ、変えるなら spec/ と docs/spec/ を同じ変更で直す
- [ ] 見本をアバタープロファイルのレベル 3 にするかを決める
- [ ] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0715-sample-and-viewer-slots.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
