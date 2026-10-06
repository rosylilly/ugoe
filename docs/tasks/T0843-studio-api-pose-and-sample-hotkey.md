---
id: T0843
title: "スタジオ: 操作 API の pose.list・pose.set と、見本のプロファイルのポーズのホットキー"
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee:
---

# T0843: スタジオ: 操作 API の pose.list・pose.set と、見本のプロファイルのポーズのホットキー

## 目的

T0842 で、スタジオのシーンにポーズの要素を覚え、操作ウィンドウの「ポーズ」のパネルとホットキー (`{ "kind": "pose", "part": … }`) で選べるようになった。
操作 API (Stream Deck などの外の道具) からは選べず、見本のプロファイルにもポーズのホットキーが無い (T0842 のやらなかったこと・候補)。

## 受け入れ条件

- [ ] 操作 API に `pose.list` (配信しているシーンのポーズのグループと要素と、選んでいる要素を返す) と `pose.set` (要素を選ぶ。操作ウィンドウの
      `show_pose` と同じ。モデルに無い要素は `scene.unknown_pose`) がある。権限は既存のメソッドの分け方に揃える。テストがある
- [ ] docs/studio-api.md と docs/studio-api.schema.json を直した (スキーマの検査が通る)
- [ ] 見本のプロファイル (`apps/studio/scripts/sample-profile.mjs`) の「着せ替え」のシーンに、手を振る差分 (`Wave`) のポーズのホットキーがあり、
      `sample-profile.test.mjs` で確かめる。docs/manual/studio.md の見本のホットキーの表を直した
- [ ] `just ci` が通る

## 参考

- T0842 のメモ (やったこと・やらなかったこと・起票の候補)
- docs/studio-api.md、docs/studio.md#ポーズの要素、apps/studio/scripts/sample-profile.mjs

## メモ

- T0040 で起票した (元の候補: T0842 (操作 API)・T0842 (見本のホットキー))。
