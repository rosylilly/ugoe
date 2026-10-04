---
id: T0833
title: "スタジオ: dressup.list で付けていないモジュールを毎回読み直さない"
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee:
---

# T0833: スタジオ: dressup.list で付けていないモジュールを毎回読み直さない

## 目的

外部 API の `dressup.list` は、付けていないモジュール (1 つ最大 512 MiB) を呼ばれるたびにファイルから読み直して概要を作る (T0819 の候補)。
クライアントが頻繁に呼ぶと重い。パス・更新時刻・大きさで覚えておき、変わっていなければ読み直さない。

## 受け入れ条件

- [ ] モジュールの概要をパス・更新時刻・大きさで覚え、変わらなければ読まないテスト (読んだ回数を数える)
- [ ] ファイルが変わった・消えたときは読み直す・一覧から外すテスト
- [ ] 覚える数か大きさに上限がある
- [ ] `just ci` が通る

## 参考

- T0819 のメモ (起票の候補)
- apps/studio/src-tauri/src/api/、docs/studio-api.md (`dressup.list`)

## メモ

- T0019 で起票した (元の候補: T0819 の `dressup.list`)。
