---
id: T0107
title: マスクのデータ定義とランタイム対応 (ADR 0009)
status: done
milestone: M1
crates: [ugoe-model, ugoe-runtime, ugoe-format (テストの追従のみ), ugoe-render (コンパイルを通す最小限の追従のみ)]
depends_on: []
assignee: claude-worker-T0107
---

# T0107: マスクのデータ定義とランタイム対応 (ADR 0009)

## 目的

[ADR 0009](../adr/0009-clipping-mask-semantics.md) で確定したマスクの意味論に合わせて、データ定義とランタイムの出力を変える。レンダラの対応は T0205 で行う。

## 受け入れ条件

- [x] `Mesh.masks` を `Vec<MaskRef { mesh, invert }>` に変え、`invert_mask` を廃止する。`Mesh.mask_mode` (`union` / `intersection`、既定 `union`) と `Mesh.mask_only` (既定 `false`) を追加する
- [x] `MeshKeyform.mask_strength` (既定 1) を追加し、ランタイムで補間して `DrawableState` に出す (0〜1 にクランプ)
- [x] `mask_only` のメッシュは `DrawableState.visible` が常に `false` になる
- [x] `validate` がマスクの参照の循環 (自分自身を含む) を検出する。`docs/spec/model.md` の ★ を消す
- [x] ランタイムが、マスクの依存関係の順序 (マスクされる側より先にマスクを評価できる順) を `CompiledModel` で提供する (T0205 で使う)
- [x] 既存のフィクスチャとテストを新しい形式に追従させる。入れ子のマスク・反転との併用・共通部分・マスク専用メッシュを使うフィクスチャを追加する
- [x] `ugoe-render` は、旧形式と同じ結果になる範囲 (通常マスクだけ、または全参照が反転) でコンパイルとテストを通す最小限の追従にとどめる。新しい意味論のレンダリングは T0205
- [x] `docs/spec/model.md` の「実装待ち」の注記を消す
- [x] `just ci` が通る

## 参考

- `docs/adr/0009-clipping-mask-semantics.md`
- `docs/spec/rendering.md#マスク`
- `docs/spec/model.md#メッシュ`

## メモ

- 循環の検出は、宣言順・`masks` の並び順の深さ優先探索で後退辺ごとに 1 件 (`ValidationError::MaskCycle`、マスクされる側 → マスクの順)。同じ循環を重複した参照でたどった分はまとめる。循環が入り組んでいる場合、すべての単純な循環を列挙するわけではない。
- `mask_strength` は有限値だけを検証し (`InvalidValue`)、0〜1 へのクランプはランタイムで行う。
- ランタイムの API (追加のみ): `DrawableState::mask_strength`、`CompiledModel::{mask_refs, mask_mode, mask_depth, mask_order}`、`ResolvedMaskRef { mesh, invert }`。`mask_order` はマスクとして参照されるメッシュだけを深さの昇順 (同じ深さは宣言順) に並べる。仕様は `docs/spec/evaluation.md#8-出力`。
- `mask_only` のメッシュは `visible = false`。参照レンダラはマスク用メッシュを表示状態に関係なくマスクパスで描くので、マスク専用メッシュは今のレンダラでもマスクとして効く。
- レンダラ (T0205 まで): 全参照が反転なら旧 `invert_mask = true` と同じ。通常と反転の併用は全参照を通常マスクとして、`intersection` は和集合として扱い、入れ子の継承と `mask_strength` は無視する (`TODO(T0205)`)。
- 追加したフィクスチャ `nested_masks` / `mixed_masks` / `intersection_masks` は、doc コメントに見える範囲を書いてあるので T0205 の GPU テストの入力に使える。
