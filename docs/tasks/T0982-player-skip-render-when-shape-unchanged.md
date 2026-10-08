---
id: T0982
title: "@ugoe/player: ビューアのループとエディタのプレビューでも、形が変わらないフレームは描かない"
status: todo
milestone: M8
crates: [packages/player, docs]
depends_on: []
assignee:
---

# T0982: @ugoe/player: 形が変わらないフレームは描かない

## 目的

T1402 で `WebPuppet.shape_changed()` (前の `render` の後に形が変わったか) を足し、`<ugoe-avatar>` は形が変わらない間は描かなくなった。
`@ugoe/player` の描画のループ (ビューアと、エディタのプレビュー) は今も `requestAnimationFrame` のたびに描く。止まったモデルや収まった物理で
描くのを省き、GPU と電力の使い方を減らす (スタジオの受け手は T0857 の別の仕組みで省いているので対象外)。

## 受け入れ条件

- [ ] `Player` のループ (player.svelte.ts の `#tick`) で、`shape_changed()` が `false` で、見せ方 (描画先の大きさ・DPR・ビュー・背景・光源・テクスチャ・
      着せ替え・スクリーンショット) を変えていないフレームは `render` を呼ばない。見せ方の変化を漏れなく拾う方法 (例: 変えたら印を立てる) を決め、
      player.svelte.ts のコメントに書いた
- [ ] テスト (偽の wasm。playerFollower.test.ts などの形) で、止まったモデルは 2 フレーム目から描かない・パラメータを動かす・ビューを動かす・大きさを変える・
      GPU を失って作り直した後は描く、を確かめた
- [ ] スクリーンショットや読み戻し (`render` の後に読むもの) が、描かなかったフレームでも正しい画を返す (テストで確かめた)
- [ ] 受け手のモード (スタジオ) の振る舞いは変わらない (既存のテストが通る)
- [ ] エディタ・ビューアの e2e (`just e2e --only editor`・`just e2e --only viewer`) が通る
- [ ] `just ci` が通る

## 参考

- T1402 のメモ (`shape_changed` の意味と起票の候補)、T0857 (受け手は変わったときだけ描く)、bindings/web/README.md#描画のループ
- packages/player/src/player.svelte.ts、crates/ugoe-wasm の `WebPuppet::shape_changed`、docs/spec/evaluation.md#見た目の変化

## メモ

- T0078 で起票した (元の候補: T1402 (`@ugoe/player` (ビューアのループ・エディタのプレビュー) でも `shape_changed` を使って描くのを省く))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- WebGPU・WebGL2 の canvas は、描かなければ前の画を出し続ける (T0857 のメモ)。`<ugoe-avatar>` (T1402) の扱いに合わせる。
- 実機のブラウザ・Tauri で電力・GPU の使用率が下がることは測れない見込み (描く・描かないの振る舞いだけを確かめる)。
