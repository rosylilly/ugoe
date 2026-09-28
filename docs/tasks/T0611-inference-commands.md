---
id: T0611
title: 推定・ミラーのエディタコマンド
status: done
milestone: M2
crates: [ugoe-editor-core]
depends_on: [T0610, T0601, T0612]
assignee: claude-worker-T0611
---

# T0611: 推定・ミラーのエディタコマンド

## 目的

`InferKeyforms` / `MirrorKeyforms` / `MarkKeyformAuthored` / `MarkKeyformDerived` を実装し、元のセルが変わったときに `derived` のセルを自動で作り直す。

## 受け入れ条件

- [x] 各コマンドに成功・失敗・Undo のテストがある
- [x] 元のセルを編集するコマンドの後、影響を受ける `derived` のセルが同じ Undo 単位で作り直される
- [x] `derived` のセルを編集すると `authored` になる
- [x] `just ci` が通る

## 参考

- `docs/spec/keyform-inference.md#推定の実行`

## メモ

- コマンドは `infer_keyforms` (`method`: `from_grid` 既定 / `additive` / `depth_rotation`)、`mirror_keyforms` (`axis`・`tolerance`・`overwrite_authored`)、`mark_keyform_authored` / `mark_keyform_derived` (`cells` はセルの指定の配列)。対象は `KeyformTarget` (`{"kind": "mesh", "id": ..}`)。`ugoe_rigging::KeyformTarget` は serde を持たないので、editor-core に serde 版を置いて `From` で変換する。
- 自動の作り直し: `apply_to` がセルを変えた格子を `touched` に積み、`Document::apply` が適用 (Batch は全体) の後に `inference::rederive` で格子の `inference` の方法で作り直す (検証の前、同じ Undo 単位)。失敗したらコマンドを拒否する。`derived` のセルがない格子・未知の方法の格子は飛ばす。新しいコマンドはセルを変えたら `touched` に積めばよい (T0606 / T0608 の編集コマンドも)。
- `set_*_keyforms` で格子を丸ごと置き換えるとき、軸が同じなら、値が変わったのに `derived` と指定されたセルを `authored` にする (`grid::replace`)。1 セルの編集は `grid::set_authored` を使う。
- 課題: ミラーで作ったセルは由来では区別できないので、元の要素を編集しても自動では作り直さない。先の格子を編集するとその格子の方法 (加算合成など) で上書きされる。ミラーの由来を記録するか、要素の `mirror` を見て自動でミラーし直すかは後続タスクで決める。
- 課題: パラメータの範囲の変更・`remove_parameter` の軸の畳み込み・深度の編集など、セルを変えないが推定の結果に影響するコマンドでは作り直さない (`infer_keyforms` を明示的に実行する)。
