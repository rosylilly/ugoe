---
id: T0012
title: 検証していない格子の大きさと画素の数の積のあふれを塞ぐ (editor-core ほか)
status: in-progress
milestone: M9
crates: [ugoe-editor-core, ugoe-ai, ugoe-cli, ugoe-psd, ugoe-mesher, docs]
depends_on: []
assignee: claude-worker-T0012
---

# T0012: 検証していない格子の大きさと画素の数の積のあふれを塞ぐ (editor-core ほか)

## 目的

M9 のコードレビューの横断の所見 (T0136・T0211・T0526・T0528。まとめは docs/workflow/code-review.md#m9-の結果)。
キーフォーム格子の大きさ (軸のキー数の積) は `ugoe-model` の検証 (T0136) と `ugoe-rigging` (T0526) で飽和させるようにしたが、
次の箇所は検査なしで掛ける。

- `ugoe-editor-core/src/grid.rs` の `stride *= binding.keys.len()` (3 か所)。エディタは読み込んだファイルを検証しないので (T0668 の方針)、
  軸の多い格子では debug でパニックし、release では回り込む
- `ugoe-ai/src/rig.rs`・`ugoe-cli/src/sample.rs` の同じ形 (自分で作った格子だけなら問題ないことを確かめる)
- 画素の数を `u32` のまま `width * height * 4` と掛ける形 (T0528)。`ugoe-psd`・`ugoe-mesher`・エディタのテクスチャの処理に残っていないか

## 受け入れ条件

- [ ] 上の箇所を `checked_mul`・`saturating_mul`・`usize` での計算にし、軸の多い格子・大きな画像の失敗するテストを先に書いた
- [ ] 自分で作った値だけを掛けるので問題ない箇所は、理由をコメントかメモに書いた
- [ ] `just ci` が通る

## 参考

- docs/workflow/code-review.md#m9-の結果
- T0136・T0526・T0528 のメモの「横断」

## メモ
