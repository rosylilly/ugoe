---
id: T0844
title: "スタジオ: 読み直したシーンで選んだポーズの要素から始め、follow_match にポーズの切り替えを入れる"
status: todo
milestone: M9
crates: [apps/studio, packages/player, docs]
depends_on: [T0329, T0843]
assignee:
---

# T0844: スタジオ: 読み直したシーンで選んだポーズの要素から始め、follow_match にポーズの切り替えを入れる

## 目的

T0842 で、スタジオの指揮役と受け手 (出力ウィンドウ・ブラウザソース・仮想カメラ) はシーンのポーズの要素を表示させるようになった。ただし:

- モデルを読み直したシーンでも、選んだ要素へ先頭の要素からフェードで切り替わる (T0329 でフェードを飛ばす口を足す)。
- 指揮役と受け手の一致の検査 (`apps/studio/src-tauri/tests/follow_match.rs`) にポーズの切り替えが入っていない。

(T0842 のやらなかったこと・候補)

## 受け入れ条件

- [ ] 指揮役と受け手が、シーンを読み直した (モデル・着せ替えを読み込んだ) 直後はシーンの要素をフェードせずに表示し、同じモデルのままの切り替え
      (パネル・ホットキー・操作 API) は今までどおりフェードする。`@ugoe/player` の `showPose` に口を足し、`Output.svelte.test.ts` と Rust のテストで確かめる
- [ ] `follow_match.rs` に、シーンの `pose` を替えた出し直しで、指揮役と受け手のパーツの不透明度が揃うことを確かめるケースがある
- [ ] docs/studio.md#ポーズの要素 を直した
- [ ] `just ci` が通る (仮想カメラの `Feed::set_scene` を触ったら、GPU のジョブのテスト `virtual_camera_gpu.rs` の扱いをメモに書く)

## 参考

- T0842 のメモ、T0329
- apps/studio/src-tauri/tests/follow_match.rs、apps/studio/src/Output.svelte、packages/player (`showPose`)

## メモ

- T0040 で起票した (元の候補: T0842 (フェードを飛ばす口のスタジオでの使い方)・T0842 (`follow_match.rs`))。T0843 と同じスタジオのポーズの処理を触るので、その後に着手する。
