---
id: T0056
title: "CI: godot のジョブで GDExtension の依存 (godot-rust) をキャッシュに残す"
status: in-progress
milestone: M10
crates: [bindings/godot, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0056
---

# T0056: CI: godot のジョブで GDExtension の依存 (godot-rust) をキャッシュに残す

## 目的

`godot` のジョブは、GDExtension (bindings/godot。ワークスペースの外、ADR 0026) の依存 (godot-rust など 22 個) を、キャッシュが完全一致でも
毎回コンパイルしている (dev 3 分 34 秒・release 2 分 28 秒。T0051 で調べた)。rust-cache は保存の前に、`cargo metadata` のワークスペースの
依存でないものを target から消すため (src/cleanup.ts)。target を共有したまま 2 つの workspace を書くと、workspace ごとに相手の依存を消すので残らない。

## 受け入れ条件

- [ ] `bindings/godot/build.mjs` が CI では GDExtension を別の target (`bindings/godot/target` など) でビルドする (今の「mbx を通さないときは分ける」
      規則との関係を整理し、手元の `just godot-check` (mbx あり・なし) の振る舞いを変えるなら理由をメモに書く)
- [ ] ci.yml の `godot` のジョブの rust-cache に `workspaces` (`bindings/godot -> <その target>`) を足し、main でだけ保存する。
      参照の画像を描く ugoe のクレートは今のとおり conformance のキャッシュを読む (1 つの手順で両方を書くか、2 つに分けるかを決め、理由を書く)
- [ ] 手元で、別の target でのビルドと `just godot-check` が通る。キャッシュの大きさの見込み (1 組の合計への増分) をメモに書いた
- [ ] docs/testing.md の「CI のキャッシュ」と bindings/godot/README.md を直した
- [ ] actionlint で増えた指摘が無い。`just ci` が通る
- 確かめるのは push の後 (このタスクでは確かめられない): main の 2 回目の run で godot のジョブが godot-rust をコンパイルしないこと。
  メモに「CI で確かめること」として書く

## 参考

- T0051 のメモ (調べたこと・起票の候補)、docs/testing.md の「CI のキャッシュ」
- bindings/godot/build.mjs、.github/workflows/ci.yml (`godot`)、https://github.com/Swatinem/rust-cache (`workspaces`)

## メモ

- T0052 で起票した (元の候補: T0051 (godot の依存がキャッシュに残らない))。
