---
id: T0961
title: "スタジオ: api のテスト turning_the_api_on_saves_the_choice_and_listens が負荷の高いときに落ちる"
status: in-progress
milestone: M9
crates: [apps/studio]
depends_on: []
assignee: claude-worker-T0961
---

# T0961: スタジオ: api のテスト turning_the_api_on_saves_the_choice_and_listens が負荷の高いときに落ちる

## 目的

`just ci` を並行の負荷の中で流すと、`ugoe-studio` の `api::tests::turning_the_api_on_saves_the_choice_and_listens`
(`apps/studio/src-tauri/src/api/tests.rs` の `assert!(reloaded.api_view().listening)`) が落ちることがある (T0178 で 3 回中 2 回)。
単独では通る。待ち受けの開始を待たずに `listening` を見ているか、前の `Studio` が同じポートを手放す前に次が待ち受けようとしている疑い。

## 受け入れ条件

- [ ] 原因を確かめ、メモに書いた (テストの誤りか、スタジオの待ち受けの開始・停止の誤りか)
- [ ] スタジオの誤りなら直してテストで確かめる。テストの誤りならテストを直す (時間の待ちを足すだけで隠さない)
- [ ] 負荷をかけて (例: 別の重いビルドと並べて、または `--test-threads` を増やして) 何度か流し、落ちないことを確かめた
- [ ] `just ci` が通る

## 参考

- apps/studio/src-tauri/src/api/、docs/studio.md

## メモ

- T0178 のワーカーの報告から起票した (2026-10-06)。
