---
id: T0834
title: "スタジオ: ホットキーと外部 API のシーンの切り替えで、読み込みを舞台のロックの外で行う"
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0833]
assignee: claude-worker-T0834
---

# T0834: スタジオ: ホットキーと外部 API のシーンの切り替えで、読み込みを舞台のロックの外で行う

## 目的

T0819 でスタジオのファイルの読み込みを非同期にしたが、ホットキーと外部 API の経路のシーンの切り替え (`with_stage`) は、
モデルやモジュールの読み込みの間ずっと舞台 (`Stage`) のロックを握る。大きなモデルの読み込み中は、ほかの操作 (パラメータの変更・
外部 API の読むだけの要求) も待たされる。読み込みをロックの外で行い、終わってから舞台に差し込む形に `Stage` を変える。

## 受け入れ条件

- [x] シーンの切り替えで、読み込み (ファイルの読み・解析) をロックの外で行い、差し込むときだけロックを取る
- [x] 読み込みの間に次の切り替えが来たら、古い方を捨てる (操作ウィンドウの経路と同じ規則)。テストがある
- [x] 読み込みの間もほかの外部 API の要求が答えるテスト
- [x] `docs/studio.md` (舞台とロックの説明) を更新した
- [x] `just ci` が通る

## 参考

- T0819 のメモ
- apps/studio/src-tauri/src/stage.rs

## メモ

- T0019 で起票した (元の候補: T0819 の舞台のロック)。同じ読み込みの経路を触る T0833 の後に着手する。
- T0834 (前提の訂正): T0819 で操作ウィンドウの `activate_scene` などは `spawn_blocking` に出したが、中身は `with_stage` のままで、
  読み込みの間ずっと舞台のロックを握っていた (メインのスレッドを塞がないだけ)。なので「操作ウィンドウの経路と同じ規則」は無く、
  このタスクで操作ウィンドウ・ホットキー・外部 API の経路をまとめて同じ形にした。
- T0834 (やったこと): `Stage` の切り替えを 始める (`begin_activate`・`begin_update_scene`・`begin_dress_up`・`begin_open_profile`。舞台を握る) →
  読む (`Pending::load`。握らない) → 差し込む (`Stage::finish`。舞台と指揮役を握る) に分けた。`lib.rs` の `Studio::switch_scene` が
  `with_stage` を 2 回に分けて呼び、間でファイルを読む。`activate_scene`・`update_scene`・`open_profile`・`restore_profile`・
  ホットキー (シーン・着せ替え)・外部 API の `scene.activate`・`dressup.set` がこれを通る。プロファイルの JSON もロックの外で読む。
  読むファイルが無いとき (見せ方だけの編集など) は握ったまま差し込む (通知を 1 回にまとめ、前と同じ順にする)。
- T0834 (決めた規則。docs/studio.md#シーンの切り替えと舞台のロック・docs/studio-api.md): 始めた順の番号を振り、差し込むときに後から始めた
  切り替え (切り替え・出しているシーンの編集・着せ替え・プロファイルを開く・新しいプロファイル) があれば読んだものを捨てる (最後に始めたものが勝つ)。
  捨てた要求は成功を返す (エラーの知らせを出さない)。読めなかった要求はエラーを返す。読んでいる間にシーンの設定が変わったら、
  切り替えは読み直し、編集と着せ替えは捨てる。出しているシーンの編集は始めるときに設定を置き換える (後の切り替えに捨てられても設定は残る)。
  着せ替えは差し込むときに書き換える (捨てられたら何も変えない)。
- T0834 (テスト): `stage.rs` の `a_later_switch_discards_the_earlier_one` (読み終えた順の両方・新しいプロファイル)・
  `a_scene_edited_while_loading_is_read_again`、`api/tests.rs` の `answers_while_a_scene_loads_and_keeps_the_later_switch`
  (読む関数を差し替え、読む間に舞台と指揮役のロックが空いていること・`scene.list`/`studio.state` が答えること・その間の `scene.activate` が勝つことを確かめた)。
  ロックを握ったまま読む `Stage::activate` などはテスト用 (`#[cfg(test)]`) に残した。
- T0834 (やらなかったこと): `Conductor::present` (パペットの組み立て・着せ替えの合成) は差し込む側でロックの中のまま。
  `just studio` を手で動かしての確認と `just e2e --only studio` はしていない (振る舞いはテストで確かめた)。Windows・macOS は確かめていない。
- T0834 (起票の候補): 大きなモデルの `Conductor::present` (組み立て) も重ければ、`Puppet` をロックの外で作ってから差し込む形にする。
  操作ウィンドウで、読み込み中を示す表示 (今は応答が返るまで何も出ない)。
