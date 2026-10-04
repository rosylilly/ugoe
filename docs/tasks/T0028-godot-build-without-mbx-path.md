---
id: T0028
title: bindings/godot の build.mjs を mbx の PATH なしで流しても target/ に書けるようにする
status: in-progress
milestone: M9
crates: [bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0028
---

# T0028: bindings/godot の build.mjs を mbx の PATH なしで流しても target/ に書けるようにする

## 目的

`bindings/godot/build.mjs` を mbx の PATH を通さずに流すと、mbx の置き場からのハードリンク (読み取り専用) を上書きしようとして
target/ に書けずに失敗する (T0456 の候補)。ワークスペースの外 (ADR 0026) なので just のレシピの環境を通らないことがある。

## 受け入れ条件

- [ ] `build.mjs` が mbx の有無を見て cargo を呼ぶ (mbx を通すか、通さないときは別の target ディレクトリを使う) か、手順を docs/workflow/build-cache.md と bindings/godot/README.md に書いた (どちらにしたかと理由をメモに書いた)
- [ ] mbx の PATH なしのシェルで `node bindings/godot/build.mjs` が通ることを確かめた
- [ ] `just ci` が通る

## 参考

- T0456 のメモ (起票の候補)、docs/workflow/build-cache.md、ADR 0026

## メモ

- T0026 で起票した (元の候補: T0456 の mbx)。
