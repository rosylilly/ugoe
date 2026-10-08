---
id: T0867
title: "スタジオ: 「トラッキングの待ち」に今の促しの余裕 (1〜4 ms) も出す"
status: todo
milestone: M8
crates: [apps/studio, docs]
depends_on: []
assignee:
---

# T0867: スタジオ: 「トラッキングの待ち」に今の促しの余裕も出す

## 目的

T0864 で、トラッキングの促しの余裕 (`Pacer` の `margin`) を 1〜4 ms の間で自分で合わせるようにしたが、今の余裕は画面から見えない。
docs/manual-check.md の S36 で、Windows・macOS で余裕が上限 (4 ms) に張り付いているか (スリープの揺れが余裕で吸収しきれていないか) を
見られるように、「フレームレートと負荷」の「トラッキングの待ち」の行に今の余裕も出す。

## 受け入れ条件

- [ ] 刻みのスレッドが今の余裕を負荷の報告に載せる (`pacing.rs` は時計とスレッドを持たない形のまま。仮の時刻のテストで、揺れたときに増えた余裕が報告に出る)
- [ ] 「トラッキングの待ち」の行に余裕を出す (例: 「平均 1.2 ms、最大 3 ms、遅れ 0%、余裕 1.5 ms」)。促しを寄せていないとき (値が無い・「—」) の表示を決めて書いた。
      文言は日英、区切りは T0862 の `load.separator`
- [ ] LoadPanel.svelte のテストで表示と読み上げの区切りを確かめた
- [ ] docs/studio.md#負荷の表示 の表、docs/manual/studio.md の「トラッキングの待ち」の説明、docs/manual-check.md の S36 (余裕も書く) を書き直した
- [ ] `just e2e --only studio` と `just ui-audit --only studio` が通る (画像が変わったら撮り直す)
- [ ] `just ci` が通る

## 参考

- T0863・T0864 のメモ、docs/studio.md#入力・#負荷の表示・#刻みの速さと正確さ、docs/manual-check.md の S36
- apps/studio/src-tauri の `pacing.rs`、apps/studio/src の LoadPanel.svelte

## メモ

- T0078 で起票した (元の候補: T0864 (「トラッキングの待ち」に今の余裕 (1〜4 ms) も出す。S36 で Windows・macOS の余裕が上限に張り付いているかを見る))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T0868 も `pacing.rs` を触るので、T0868 はこのタスクの後にする。
- Windows・macOS の実機の値は S36 で利用者が見る (作業者の環境では確かめられない)。
