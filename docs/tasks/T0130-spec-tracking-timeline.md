---
id: T0130
title: 公開仕様: トラッキングとホストの操作 (ポーズの表示・効果の差し替え) の時間の例
status: in-progress
milestone: M5
crates: [spec, tools/spec-check, docs]
depends_on: [T0126]
assignee: claude-worker-T0130
---

# T0130: 公開仕様: トラッキングとホストの操作の時間の例

## 目的

T0126 の後続の候補。時間の例 (`spec/examples/*/timeline.json`) に、トラッキングの入力 (spec/avatar-profile.md)、ホストがポーズの表示を直接選ぶ操作、効果の設定の差し替えが含まれていない。

## 受け入れ条件

- [ ] 作業者は `crates/` と `docs/spec/` を読まず、`spec/` だけを参照する (T0120・T0126 と同じ手順)
- [ ] timeline.json の操作にトラッキングの入力・ポーズの表示の選択・効果の差し替えを足し、例を足す。形式の変更は `spec/examples/README.md` に書く
- [ ] `tools/spec-check` の独立の実装を合わせる。参照実装での照合 (`crates/ugoe/tests/spec_timelines.rs`) の拡張は team-lead に頼む
- [ ] 見つかった仕様の抜け・曖昧さの一覧と `spec/` への修正 (`docs/spec/` の同期は team-lead に頼む)
- [ ] `Parameter.mirror` を省略した意味: 英語版は「対を記録していない」とだけ書くが、参照実装のミラーは省略を「自分自身の同じ値のキーに対応する」として扱う (T0129 の同期で見つかった)。spec/model.md に書き足すかを決める
- [ ] 描画の例 (spec/examples/render-*) に、鏡映・回転するビュー変換、9 個以上の光源、モデルの `lighting: false` の例を足す (T0127 の後続の候補。参照実装の照合は team-lead が拡張する)
- [ ] `just ci` が通る

## 参考

- `docs/tasks/T0126-spec-physics-motion-examples.md` のメモ

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
