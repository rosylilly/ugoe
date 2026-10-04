---
id: T0833
title: "スタジオ: dressup.list で付けていないモジュールを毎回読み直さない"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0833
---

# T0833: スタジオ: dressup.list で付けていないモジュールを毎回読み直さない

## 目的

外部 API の `dressup.list` は、付けていないモジュール (1 つ最大 512 MiB) を呼ばれるたびにファイルから読み直して概要を作る (T0819 の候補)。
クライアントが頻繁に呼ぶと重い。パス・更新時刻・大きさで覚えておき、変わっていなければ読み直さない。

## 受け入れ条件

- [x] モジュールの概要をパス・更新時刻・大きさで覚え、変わらなければ読まないテスト (読んだ回数を数える)
- [x] ファイルが変わった・消えたときは読み直す・一覧から外すテスト
- [x] 覚える数か大きさに上限がある
- [x] `just ci` が通る

## 参考

- T0819 のメモ (起票の候補)
- apps/studio/src-tauri/src/api/、docs/studio-api.md (`dressup.list`)

## メモ

- T0019 で起票した (元の候補: T0819 の `dressup.list`)。
- T0833 (やったこと): `methods.rs` に `ModuleSummaries` (パス → 更新時刻・大きさ・概要。読めなかったものも `None` として覚える) を足し、
  `Studio::module_summaries` に持たせた。`dressup_list_with` は付けていないモジュールの時刻と大きさを `fs::metadata` で調べ、
  覚えた値と同じなら読まない。違えば読み直し (読む前に調べた時刻と大きさで覚える)、ファイルが無ければ一覧から外して忘れる。
  呼ぶたびに着せ替えに使えなくなったパスを忘れ、`MAX_MODULE_SUMMARIES` (256) を超えては覚えない (超えた分は毎回読む)。
  覚えの表のロックは読む間は握らない (T0819 のテストに検査を足した)。テスト: `remembers_the_wardrobe_until_the_file_changes`
  (読んだ回数・書き換え・消去)・`remembers_a_bounded_wardrobe` (上限・忘れる)。docs/studio.md#メインのスレッドを塞がない に書いた。
- T0833 (やらなかったこと): 時刻と大きさがどちらも変わらない書き換え (時刻の細かさより短い間に同じ大きさで書き換える) は見分けない。
  覚える数の上限を超えたときは古いものを追い出さず、新しいものを覚えないだけ (プロファイルのシーンとホットキーで 256 を超えるのは稀)。
  `just studio` を手で動かしての確認はしていない。Windows・macOS は確かめていない (`SystemTime` の比較だけで OS に依らない)。
- T0833 (起票の候補): なし。
