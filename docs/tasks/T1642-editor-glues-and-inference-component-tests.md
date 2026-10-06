---
id: T1642
title: "エディタ: グルーのパネル (GluesPanel) と推定の節 (InferenceSection) の部品のテストを足す"
status: todo
milestone: M10
crates: [apps/editor]
depends_on: []
assignee:
---

# T1642: エディタ: グルーのパネル (GluesPanel) と推定の節 (InferenceSection) の部品のテストを足す

## 目的

T0949 でグルーのパネルの編集の欄 (キー・軸) とインスペクタの推定の節の欄を `@ugoe/ui` の `NumberField` などに移したが、部品のテストが無く、
`glues.test.ts`・`inference.test.ts` は純粋な関数だけを見ている (T0949 の候補)。欄の振る舞いを部品のテストで固める。

## 受け入れ条件

- [ ] `GluesPanel` の部品のテスト: 欄を空にしたとき・範囲の外の値のとき、確定のボタンが押せない (`oninput` で可否が変わる)。確定で送るコマンドとその値
- [ ] `InferenceSection` の部品のテスト: 同じく空の欄・`oninput` でのボタンの可否と、確定で送るコマンド
- [ ] 振る舞いは変えない (テストを書いて分かった誤りは、直すなら先に失敗するテストを書き、メモに書いた)
- [ ] `pnpm --filter editor test` と `just ci` が通る

## 参考

- T0949 のメモ (やったこと・起票の候補)
- apps/editor/src/lib/panels/glues/、apps/editor/src/lib/panels/inspector/inference/

## メモ

- T0043 で起票した (元の候補: T0949 (GluesPanel・InferenceSection の部品のテスト))。同じ候補の ui-audit の場面は T0044 にした。
