---
id: T0843
title: "スタジオ: 操作 API の pose.list・pose.set と、見本のプロファイルのポーズのホットキー"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee: claude-worker-T0843
---

# T0843: スタジオ: 操作 API の pose.list・pose.set と、見本のプロファイルのポーズのホットキー

## 目的

T0842 で、スタジオのシーンにポーズの要素を覚え、操作ウィンドウの「ポーズ」のパネルとホットキー (`{ "kind": "pose", "part": … }`) で選べるようになった。
操作 API (Stream Deck などの外の道具) からは選べず、見本のプロファイルにもポーズのホットキーが無い (T0842 のやらなかったこと・候補)。

## 受け入れ条件

- [x] 操作 API に `pose.list` (配信しているシーンのポーズのグループと要素と、選んでいる要素を返す) と `pose.set` (要素を選ぶ。操作ウィンドウの
      `show_pose` と同じ。モデルに無い要素は `scene.unknown_pose`) がある。権限は既存のメソッドの分け方に揃える。テストがある
- [x] docs/studio-api.md と docs/studio-api.schema.json を直した (スキーマの検査が通る)
- [x] 見本のプロファイル (`apps/studio/scripts/sample-profile.mjs`) の「着せ替え」のシーンに、手を振る差分 (`Wave`) のポーズのホットキーがあり、
      `sample-profile.test.mjs` で確かめる。docs/manual/studio.md の見本のホットキーの表を直した
- [x] `just ci` が通る

## 参考

- T0842 のメモ (やったこと・やらなかったこと・起票の候補)
- docs/studio-api.md、docs/studio.md#ポーズの要素、apps/studio/scripts/sample-profile.mjs

## メモ

- T0040 で起票した (元の候補: T0842 (操作 API)・T0842 (見本のホットキー))。

### やったこと

- **操作 API (`api/methods.rs`):** `pose.list` (読む) は `groups: [{parts, module, shown}]` を返す。`parts`・`module` は操作ウィンドウの
  `poseGroups` と同じ (`Conductor::pose_groups`)、`shown` は指揮役が表示させている要素 (`scene` の `look.pose` のうちそのグループのもの)。
  モデルが無ければ空。`pose.set` (操作) は `{part}` を取り、`show_pose` と同じ `Stage::begin_show_pose` を `switch_scene` で行う
  (シーンに覚える)。モデルに無い要素は `-32010` (`unknownPose`・`messageCode` は `scene.unknown_pose`)。出しているシーンが無いときは
  `dressup.set` に揃えて `noModel` を返す (操作ウィンドウの `show_pose` は何もしないが、API では呼んだ側に分かるように)。
  権限は既存の分け方 (`*.list` は読む、変えるものは操作)。
- テスト: `api/tests.rs` の `lists_and_sets_the_pose_of_the_scene_on_air` (シーンが無いとき・一覧・読む権限で拒む・選ぶとシーンに覚える・
  未知の要素の `error.data`・引数の誤り)。`the_schema_covers_every_method_and_event` の並びに 2 つを足した (応答も Schema と比べる)。
- 文書: docs/studio-api.md (「Pose」の節・`-32010` の理由に `unknownPose`・冒頭)、docs/studio-api.schema.json (`pose.list`・`pose.set` の
  引数と結果・要求の `oneOf`・`error.data` の説明)、docs/studio.md (外部 API の表・ポーズの要素の「選ぶ」・舞台のロック)、
  docs/manual/studio.md (ポーズの節と操作 API の節から案内)。
- **見本のプロファイル:** 「着せ替え」のシーンの本体 (`dress-up.ugoe`) のポーズに、Ctrl+Alt+W で `Wave` (手を振る)、Ctrl+Alt+D で
  `HandsDown` (戻す) を足した (戻す口が無いとパネルを使うしかないので、2 つにした)。`sample-profile.test.mjs` は `dress-up.ugoe` の
  `MODL` (圧縮していない MessagePack) からポーズの要素を拾い、`["HandsDown", "Wave"]` であること・`Wave` のホットキーがあること・
  ポーズのホットキーの要素がモデルにあることを確かめる。マニュアルの見本のホットキーの表を直した。
- `just ci` と `just vcam-check` が通った。

### やらなかったこと

- ポーズの変化の通知 (`pose.changed` のようなイベント) は足していない (受け入れ条件に無い。シーンの切り替えでは `scene.changed` だけ)。
- 実機の Stream Deck などからは試していない (テストの接続と Schema の検査だけ)。

### 起票の候補

- 操作 API に、配信しているシーンのポーズの要素が変わったときの通知 (`pose.changed`。操作ウィンドウ・ホットキー・`pose.set`・シーンの切り替え)
  を足す (Stream Deck のボタンの状態を合わせるため)。
- `sample-profile.test.mjs` の MessagePack の拾い方は並びに頼る。ほかの見本のテストでもモデルの中身を見るなら、`@ugoe/runtime` (wasm) で読む
  手段を用意する。
