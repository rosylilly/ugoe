---
id: T0146
title: ID から添字への引き当てを毎フレームの線形探索にしない
status: todo
milestone: M9
crates: [ugoe-runtime, ugoe, ugoe-ffi, docs]
depends_on: [T0137]
assignee:
---

# T0146: ID から添字への引き当てを毎フレームの線形探索にしない

## 目的

T0137 (ugoe-runtime のコードレビュー) の性能の所見。

`ParameterValues::index_of` と `CompiledModel::part_index`・`deformer_index`・`mesh_index`・`hit_area_index` は、ID を先頭から比べる線形探索。
ロード時や問い合わせで使うぶんには問題ないが、毎フレームの経路でも呼ばれている。

- `ugoe::Puppet::apply_part_opacities` (`crates/ugoe/src/lib.rs`): モーションのパーツの不透明度のカーブごとに `part_index` を呼ぶ (カーブ数 × パーツ数の文字列比較)
- C ABI の ID によるパラメータの一括設定 (`crates/ugoe-ffi/src/puppet.rs`): 値ごとに `index_of` を呼ぶ (値の数 × パラメータ数)

ランタイムの目標 (1 体 1ms、roadmap.md の M7) に対して、パラメータとパーツが数百あるモデルで無視できない可能性がある。
まず中規模のモデル (`crates/ugoe-runtime/examples/compose_timing.rs` の規模) で毎フレームの費用を測り、効くなら次のどちらか (または両方) にする。

- `CompiledModel` に ID → 添字の表をコンパイル時に作り、`*_index` を定数時間にする (`compile` は既に ID の表を作っているので、それを持ち続ける)。
  `ParameterValues` は複製される型なので、表は `CompiledModel` 側に置き、`Arc` で共有するなどして複製の費用を増やさない
- 毎フレームの呼び出し側で、添字の解決をロード時 (モーションの開始時など) に前倒しする

## 受け入れ条件

- [ ] 変更の前後で、ID による引き当てを含む 1 フレームの費用を測り、結果をメモに書く
- [ ] 効果があれば、公開 API を変えずに引き当てを定数時間にするか、毎フレームの呼び出しを前倒しする (テストを足す)
- [ ] `ModelInstance::update` の毎フレームの確保がないこと (既存のテスト) を保つ
- [ ] `just ci` と `just wasm-check` が通る

## 参考

- docs/tasks/T0137-review-runtime.md (メモの所見の表)
- docs/roadmap.md (M7 の性能の目標)
- docs/spec/evaluation.md#デフォーマのワールド変換を読む (ID からの添字の引き当て)

## メモ
