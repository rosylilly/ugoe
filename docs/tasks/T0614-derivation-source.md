---
id: T0614
title: 推定したセルの由来 (推定の方法) を記録する
status: done
milestone: M2
crates: [ugoe-model, ugoe-rigging, ugoe-editor-core]
depends_on: [T0611]
assignee: claude-worker-T0614
---

# T0614: 推定したセルの由来 (推定の方法) を記録する

## 目的

T0611 の自動の再推定は、格子の `inference` の方法 (加算合成など) で `derived` のセルを作り直す。
そのため、ミラーで作ったセルは、ミラー先の格子を編集したときに加算合成で上書きされてしまう。
セルごとに「どの方法で推定したか」を記録し、再推定ではその方法で作り直すようにする。

## 受け入れ条件

- [x] `derived` のセルが、推定の方法 (加算合成・深度からの回転・ミラーとミラー元) を記録する。古いデータ (記録なし) は格子の `inference` の方法とみなす
- [x] ミラーで作ったセルは、ミラー元の格子が変わったときにミラーで作り直され、ミラー先の編集では上書きされない
- [x] 仕様 (`docs/spec/keyform-inference.md`) を更新する
- [x] `just ci` が通る

## 参考

- `docs/tasks/T0611-inference-commands.md` のメモ
- `docs/spec/keyform-inference.md#セルの由来`

## メモ

- データ: `KeyformOrigin` に方法の由来 `additive` / `depth_rotation` / `mirror` を足した (`derived` は「方法の記録なし = 格子の `inference` の方法」として残す。古いファイルはそのまま読めて以前と同じに作り直す)。ミラー元は格子ごとに 1 つなので、セルではなく `KeyformGrid::mirror_source` (`id`・明示した `axis` / `tolerance`) に持つ。`KeyformGrid::derivation(i)` がセルを作り直す方法を返す。未知の由来は従来どおり `authored` (上書きせず推定の元にする)。`validate` は `mirror_source.id` が同じ種類の要素に存在するか (`UnknownReference`、`field: keyforms.mirror_source`) と `axis` / `tolerance` が有限かを検査する。
- rigging: `InferScope` (`Own` / `AllDerived`) を `infer_additive`・`infer_additive_target`・`infer_depth_rotation` の引数と `MirrorOptions::scope` に足した。書き換えないセル (別の方法のセル) は推定の元にする (基準のセルがミラーのセルでもよい)。書いたセルに方法を記録し、ミラーは `mirror_source` を記録する。
- editor-core: `rederive` はセルごとの方法で作り直し (`Own`)、`mirror_source` をたどって依存する格子のミラーのセルをやり直す。値が変わらなくなるか 1 格子 8 回で止める (循環対策)。自動のミラーのやり直しの失敗 (頂点が対応しなくなった等) は黙ってミラーのセルを残す (元のメッシュの基準の頂点を大きく動かすと先をミラーできなくなり、拒否すると元を編集できなくなるため)。`InferKeyforms` に `overwrite_derived` を足し、推定のコマンドも `touched` に積むようにした。削除でミラー元がなくなったら `mirror_source` を外し、`mirror` のセルを `authored` にする。
- 課題: ミラー先の頂点の対応はミラーのたびに基準の位置から求め直す。元の基準の頂点を許容距離より大きく動かすと自動のミラーのやり直しは黙って止まる (UI で知らせるか、対応を記録するかは T0608 以降で検討)。
- 課題: T0611 の課題のうち、パラメータの範囲の変更・深度の編集など、セルを変えないコマンドでの作り直しは未対応のまま。
