---
id: T0825
title: スタジオ: 出力の受け手が読めなかったことを操作ウィンドウに知らせる
status: done
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0815]
assignee: claude-worker-T0825
---

# T0825: スタジオ: 出力の受け手が読めなかったことを操作ウィンドウに知らせる

## 目的

出力ウィンドウとブラウザソースの受け手 (`Output.svelte` の `applyScene`) は、シーンのモジュールやモデルのバイト列を取れないと
「もう次のシーンになった」とみなして黙って戻る (`.catch(() => null)`・空の `catch`)。モデルを読めなかったとき (`player.load` の失敗) は
`player.error` に書くだけで、誰にも見えない。次のシーンになったのではなく本当に読めなかったときも、画面は前のシーンのまま (または空) で、
配信者は理由を知る手段が無い (T0815 のレビューで見つけた)。

受け手が読めなかったこと (シーンの版・何を・理由のコード) を、描画の負荷と同じ口 (`report_render_stats` か新しいコマンド・
ブラウザソースは WebSocket) でバックエンドへ返し、操作ウィンドウに知らせる。次のシーンになったための失敗 (版が古い) とは分ける。

## 受け入れ条件

- [x] 受け手がモデル・モジュール・背景を読めなかったとき、操作ウィンドウに出す (出力には何も重ねない)。文言は翻訳する (ADR 0029)
- [x] 次のシーンに移ったための失敗は知らせない
- [x] IPC を変えたら `apps/studio/src/lib/host.ts` と docs/studio.md を同じ PR で直す
- [x] テストがある
- [x] `just ci` が通る

## 参考

- docs/studio.md#シーンの切り替えと受け手・#負荷の表示
- ADR 0028・ADR 0029

## メモ

- やったこと:
  - バックエンド: `src-tauri/src/load_failure.rs` (新規)。受け手の報告 (`revision`・`asset`・モジュールの `index`・`code` = `unavailable` / `unreadable` / `noRenderer`) を
    今のシーンの版のものだけ受け、受け手のラベルごとに 1 つ覚える。シーンが変わったら (`send_scene`) 前のシーンの報告を捨て、ブラウザソースのページが去ったときもその報告を捨てる。
    操作ウィンドウへは `Monitor::LoadFailures` (`kind: "loadFailures"`) で配り、`subscribe_status` の接続時にも送る。
  - 口: 出力ウィンドウは新しいコマンド `report_load_failure` (`acl.rs` の `COMMANDS`・`OUTPUT_COMMANDS`・`capabilities/output.json`)、ブラウザソースは WebSocket の `{"kind":"loadFailure",…}`。
    `stage::Asset` に `Deserialize` を足した。
  - 受け手 (`Output.svelte`): 失敗したとき最後に届いたシーンが変わっていなければ報告する (移ったための失敗は報告しない)。モデルは `player.error` で読めなかったことを見分け、
    読めなければ前のモデルにこのシーンのフレームを当てずに止める (以前は新しいシーンのフレームを前のモデルへ当てていた)。モジュールは 1 つずつ `loadModule` して読めなかった番号を返し、
    読めたものとモデルで出す。読めなかったものは次のシーンの通知で取り直す (付けたモジュールの版を覚えない)。Player が `unavailable` なら `noRenderer` を返してモデルを取らない。
  - 操作ウィンドウ (`Control.svelte`): 受け手ごとに危険の Notice (閉じられる) を出す。文は `lib/receivers.ts` (負荷のパネルの `receiverName` もここへ移した)。ja / en の文言を足した。
  - 文書: docs/studio.md に「受け手が読めなかったとき」の節、出力ウィンドウのコマンドとブラウザソースの「読むだけ」を直した。apps/studio/README.md のコマンドの表と構成。
  - テスト: `load_failure.rs` (JSON の形)・`api/tests.rs` (今のシーンだけ・番号の検査・シーンの切り替えと受け手が去ったときに捨てる・WebSocket で届く)・
    `src/Output.svelte.test.ts` (happy-dom と偽の Player で、受け取れない・読めない・移ったための失敗は知らせない・取り直し・`noRenderer`)・`lib/receivers.test.ts`・`lib/feed.test.ts`。
- やらなかったこと:
  - 背景の画像は CSS で描くので、受け取った画像が壊れていて描けないことは知らせない (受け取れないことだけ)。
  - 仮想カメラ (ネイティブの受け手) の読み込みの失敗は含めない。
  - 実機の Tauri の WebView での確認はしていない (IPC の形は host.ts とバックエンドのテストで合わせた)。
- 起票の候補:
  - 仮想カメラの受け手がシーンのモデル・モジュールを読めなかったときも、同じ `loadFailures` で知らせる。
  - 背景の画像を受け手がデコードできなかったときの知らせ (`Image.decode` などで確かめる)。
