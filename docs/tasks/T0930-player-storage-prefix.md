---
id: T0930
title: "@ugoe/player: localStorage のキーの接頭辞をアプリが渡せるようにする"
status: todo
milestone: M9
crates: [packages, apps/viewer, apps/studio, docs]
depends_on: [T0914]
assignee:
---

# T0930: @ugoe/player: localStorage のキーの接頭辞をアプリが渡せるようにする

## 目的

T0914 のレビューで見つけた。@ugoe/player が覚えるもの (光源・モデルごとの光源・着せ替えの組・待機モーション・
トラッキングのままにするパラメータ・自動で戻す・自動で流す・光源の印・覚えたものの並び) の localStorage のキーは
`ugoe-viewer.*` に固定されている。README は「スタジオで使うときに、キーの接頭辞をアプリが渡せるようにする」と書くが、
まだできていない。また、`loadToggle`・`loadLightMarkers`・`loadAutoRestore`・`loadRememberedOrder` などの
「try の中で localStorage を引き、読めなければ既定」という小さな関数が、ファイルごとに少しずつ違う形で重複している。

## 受け入れ条件

- [ ] `PlayerHost` (か `Player` の引数) でキーの接頭辞を渡せる。省くと今の `ugoe-viewer` (ビューアの覚えたものを失わない)
- [ ] 覚える関数が接頭辞と `Storage` を受け取る 1 つの小さな入れ物にまとまり、各ファイルの重複が無くなる
- [ ] スタジオの出力ウィンドウ (受け手) が覚える必要の無いもの (待機モーションの選択を最近使ったものにする書き込みなど) を書かない
- [ ] README とテストを合わせる
- [ ] `just ci` が通る

## 参考

- packages/player/README.md
- docs/tasks/T0914-review-player.md の所見の表

## メモ
