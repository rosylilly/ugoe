---
id: T0848
title: "スタジオ: 指揮役の表示中のポーズの要素を操作ウィンドウに出し、操作 API に pose.changed を足す (判断が要る)"
status: todo
milestone: M9
crates: [apps/studio, docs]
depends_on: []
assignee:
---

# T0848: スタジオ: 指揮役の表示中のポーズの要素を操作ウィンドウに出し、操作 API に pose.changed を足す (判断が要る)

## 目的

T0736 で、`@ugoe/player` の Player とエディタのプレビューのポーズの欄は、モーションが切り替えた要素に追従するようになった。スタジオの操作ウィンドウの
「ポーズ」のパネルは、指揮役 (Rust の側) のシーンに覚えた選択を出すので、モーションで手が変わっても欄は選んだ要素のまま
(docs/studio.md「パネルの表示はモーションに追従しない」。T0736 の候補)。
また操作 API (T0843 の `pose.list`・`pose.set`) には、ポーズの要素が変わったときの通知が無く、Stream Deck などのボタンの状態を合わせられない (T0843 の候補)。
どちらも指揮役の Puppet の表示中の要素 (`PosePlayer::visible_parts`) と、シーンに覚える選択をどう分けて外に見せるかで決まるので、1 つのタスクで扱う。

**判断が要る:** 案:

- A. 欄と `pose.changed` は表示中の要素 (モーションの切り替えを含む) を出す。シーンに覚える選択は `pose.list` の別の欄 (`selected` など) にする。
- B. 欄は Player と同じ規則 (後から来たほうが勝つ) で表示中の要素を出し、`pose.changed` は選択が変わったとき (操作ウィンドウ・ホットキー・`pose.set`・
  シーンの切り替え) だけ送る。モーションによる表示の変化は送らない。
- C. 欄はシーンの選択のまま (今のまま) にし、`pose.changed` だけを選択の変化で足す。

推奨は、指揮役が表示中の要素を読む頻度 (毎フレームか変化だけか) と、受け手 (出力ウィンドウ) との食い違いを調べてから書く。

## 受け入れ条件

- [ ] 案と推奨をメモに書き、利用者の判断を得た
- [ ] 決めた形で、操作ウィンドウの「ポーズ」のパネルの表示が変わる (C なら変えない)。指揮役の単体テストと部品のテストがある
- [ ] 操作 API に `pose.changed` があり (`events.subscribe` で選べる)、決めた場合に送る。`api/` のテストがある
- [ ] docs/studio-api.md の Events の表と docs/studio-api.schema.json (スキーマの検査が通る)、docs/studio.md#ポーズの要素・docs/manual/studio.md を直した
- [ ] `just ci` が通る

## 参考

- T0736 のメモ (やったこと・やらなかったこと・起票の候補)、T0843 のメモ (起票の候補)、T0844 (読み直したシーンの要素をフェードせずに出す)
- apps/studio/src-tauri/src (`conductor.rs`・`api/methods.rs`)、apps/studio/src/PosePanel.svelte、docs/studio-api.md#events

## メモ

- T0041 で起票した (元の候補: T0736 (操作ウィンドウのポーズの欄の追従)・T0843 (`pose.changed`))。
