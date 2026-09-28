---
id: T0107
title: マスクのデータ定義とランタイム対応 (ADR 0009)
status: in-progress
milestone: M1
crates: [ugoe-model, ugoe-runtime, ugoe-format (テストの追従のみ), ugoe-render (コンパイルを通す最小限の追従のみ)]
depends_on: []
assignee: claude-worker-T0107
---

# T0107: マスクのデータ定義とランタイム対応 (ADR 0009)

## 目的

[ADR 0009](../adr/0009-clipping-mask-semantics.md) で確定したマスクの意味論に合わせて、データ定義とランタイムの出力を変える。レンダラの対応は T0205 で行う。

## 受け入れ条件

- [ ] `Mesh.masks` を `Vec<MaskRef { mesh, invert }>` に変え、`invert_mask` を廃止する。`Mesh.mask_mode` (`union` / `intersection`、既定 `union`) と `Mesh.mask_only` (既定 `false`) を追加する
- [ ] `MeshKeyform.mask_strength` (既定 1) を追加し、ランタイムで補間して `DrawableState` に出す (0〜1 にクランプ)
- [ ] `mask_only` のメッシュは `DrawableState.visible` が常に `false` になる
- [ ] `validate` がマスクの参照の循環 (自分自身を含む) を検出する。`docs/spec/model.md` の ★ を消す
- [ ] ランタイムが、マスクの依存関係の順序 (マスクされる側より先にマスクを評価できる順) を `CompiledModel` で提供する (T0205 で使う)
- [ ] 既存のフィクスチャとテストを新しい形式に追従させる。入れ子のマスク・反転との併用・共通部分・マスク専用メッシュを使うフィクスチャを追加する
- [ ] `ugoe-render` は、旧形式と同じ結果になる範囲 (通常マスクだけ、または全参照が反転) でコンパイルとテストを通す最小限の追従にとどめる。新しい意味論のレンダリングは T0205
- [ ] `docs/spec/model.md` の「実装待ち」の注記を消す
- [ ] `just ci` が通る

## 参考

- `docs/adr/0009-clipping-mask-semantics.md`
- `docs/spec/rendering.md#マスク`
- `docs/spec/model.md#メッシュ`

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
