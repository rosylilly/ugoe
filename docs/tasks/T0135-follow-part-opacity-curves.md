---
id: T0135
title: ランタイム: 受け手でもモーションのパーツの不透明度のカーブを当てる
status: todo
milestone: M8
crates: [ugoe, ugoe-ffi, ugoe-wasm, bindings/web, tests/conformance, apps/studio, docs]
depends_on: [T0134, T0803]
assignee:
---

# T0135: 受け手のパーツの不透明度

## 目的

`Puppet::follow` はパラメータの値だけを受け取るので、モーションのパーツの不透明度のカーブ (パーツを切り替えるモーション) が受け手 (出力ウィンドウ・ブラウザソース・
仮想カメラ) に出ない (T0803 のメモ)。ADR 0028 の「受け手は 1 つの Puppet と同じ形」を守る。

## 受け入れ条件

- [ ] 指揮役が配るフレームに、モーションが上書きしたパーツの不透明度を足す (変わったパーツだけ・疎な形でよい)。受け手の `follow` は 1 つの Puppet と同じ段で当てる
- [ ] パーツの不透明度のカーブを持つモーションで、指揮役と受け手の組み合わせが 1 つの Puppet と一致するテスト (適合テストのケースも足す)
- [ ] C ABI・wasm・スタジオの指揮役と受け手を合わせ、docs/spec/motion.md の「受け手として進める」と docs/studio.md のフレームの形を更新する
- [ ] `just ci`・`just wasm-check`・`just ffi-check`・`just conformance --only c,wasm` が通る

## 参考

- docs/tasks/T0803-studio-inputs.md・T0134-follow-late-stages-order.md のメモ

## メモ
