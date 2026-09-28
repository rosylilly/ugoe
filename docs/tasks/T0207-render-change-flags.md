---
id: T0207
title: 参照レンダラ: 変更フラグによる転送の省略
status: done
milestone: M2
crates: [ugoe-render, ugoe-runtime]
depends_on: [T0205]
assignee: claude-worker-T0207
---

# T0207: 参照レンダラ: 変更フラグによる転送の省略

## 目的

T0106 の変更フラグ (`ModelInstance::changes()`) を使い、頂点位置やユニフォームが変わっていない描画可能要素の転送を省く。

## 受け入れ条件

- [x] 1 つの `Renderer` で同じモデルの複数のインスタンスを描く場合や、`encode` の間に `update` が複数回呼ばれる場合でも正しく描画される (インスタンスごとのバッファを持つ、または最後に転送した状態を記録するなど)。T0205 のメモにある懸念を解消する
- [x] 変わらないフレームで転送量が減ることを確認するテストがある (`FrameStats` に転送量を追加するなど)
- [x] `just ci` と `just test-gpu` が通る

## 参考

- `docs/spec/evaluation.md#8-出力`
- `docs/tasks/T0106-change-flags.md`、`docs/tasks/T0205-render-mask-semantics.md` のメモ

## メモ

- 方式: レンダラがバッファに転送済みの内容 (頂点位置・要素ごとのユニフォーム・ビュー) を CPU 側に控え、違う部分だけを転送する
  (`crates/ugoe-render/src/upload.rs`)。隣り合う要素・スロットの転送は 1 回の `write_buffer` にまとめる。
  - 頂点位置: 控えを書いたときの `(ModelInstance::id(), update_count())` を覚える。同じなら何も転送しない、`update_count` が
    ちょうど 1 つ後なら変更フラグ `POSITIONS` を信じる、それ以外 (別のインスタンス・update を 2 回以上挟んだ) は控えとビット列で比べる。
    別モデルへの切り替えはバッファを作り直すので全転送。`mark_all_changed` 後はフラグが ALL なので全転送。
  - ユニフォーム: マスクの割り当てやビューにも依存するのでフラグは使わず、毎フレーム作った値とスロットごとに比べる。
  - マスクアトラスの使い回し: アトラス (作り直していない)・一辺・内容の割り当て・ビュー・テクスチャの世代が前回と同じで、
    このフレームでマスクパスのユニフォームと使う内容のマスク用メッシュの頂点位置を転送していなければ、マスクパスを省く。
- ランタイムへの追加 (加算のみ): `ModelInstance::id()` (プロセス内で一意、`AtomicU64` の連番) と `update_count()`
  (`new` の中の 1 回を含む)。`docs/spec/evaluation.md#変更フラグ` に追記した。
- 公開 API の追加: `FrameStats::{masks_reused, position_bytes, uniform_bytes}`。既存の API は変えていない
  (`FrameStats` を構造体リテラルで作るコードはフィールドの追加が必要。GPU テストは `..image.stats` にした)。
- テスト: `upload.rs` の単体テストと、GPU テスト (静止フレームの転送 0 とマスクの使い回し、1 メッシュだけ動かす、動くマスク、
  2 つのインスタンスを交互に描く、encode の間に update を 2 回、モデルの切り替え、`mark_all_changed`、マスクのテクスチャの差し替え)。
- 残した課題: 控えの比較は頂点数に比例する (ランタイムの変更検出と同程度)。ユニフォームは毎フレーム全スロット分を作って比べている。
  インスタンスごとにバッファを持てば交互描画でも転送を減らせるが、必要になるまで見送った。
