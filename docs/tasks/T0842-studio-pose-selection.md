---
id: T0842
title: "スタジオ: プロファイルやシーンでポーズの要素を選んで覚える"
status: in-progress
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0842
---

# T0842: スタジオ: プロファイルやシーンでポーズの要素を選んで覚える

## 目的

T0735 で、`@ugoe/player` の `Player` にポーズのグループ (`poseGroups`)・表示中の要素 (`poseShown`)・`showPose` が入り、ビューアで選べるようになった。
スタジオでは選べず、配信で衣装や持ち物の差分を切り替えられない (T0735 の候補)。

## 受け入れ条件

- [ ] スタジオの操作ウィンドウでポーズの要素を選べ、出力ウィンドウ (受け手) の姿が切り替わる (指揮役から受け手への知らせ。部品か関数のテスト)
- [ ] 選んだ要素をプロファイルかシーン (どちらに持つかを決めて docs/studio.md に理由を書く) に覚え、読み直したときに戻る。モデルに無い要素は捨てる。テストがある
- [ ] ホットキーで要素を切り替えられるようにするかを決め、するならホットキーの動作に足した (しないなら理由をメモに書く)
- [ ] docs/studio.md と docs/manual のスタジオの章を直した。画像の撮り直しが要れば T0919 のメモに書き足した
- [ ] `just ci` が通る

## 参考

- T0735 のメモ (起票の候補)
- packages/player/src/pose.ts、apps/studio/src/lib/profile.ts・hotkeys.ts、docs/studio.md

## メモ

- T0038 で起票した (元の候補: T0735 (スタジオ))。
