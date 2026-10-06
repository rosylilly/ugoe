---
id: T0961
title: "スタジオ: api のテスト turning_the_api_on_saves_the_choice_and_listens が負荷の高いときに落ちる"
status: done
milestone: M9
crates: [apps/studio]
depends_on: []
assignee: claude-worker-T0961
---

# T0961: スタジオ: api のテスト turning_the_api_on_saves_the_choice_and_listens が負荷の高いときに落ちる

## 目的

`just ci` を並行の負荷の中で流すと、`ugoe-studio` の `api::tests::turning_the_api_on_saves_the_choice_and_listens`
(`apps/studio/src-tauri/src/api/tests.rs` の `assert!(reloaded.api_view().listening)`) が落ちることがある (T0178 で 3 回中 2 回)。
単独では通る。待ち受けの開始を待たずに `listening` を見ているか、前の `Studio` が同じポートを手放す前に次が待ち受けようとしている疑い。

## 受け入れ条件

- [x] 原因を確かめ、メモに書いた (テストの誤りか、スタジオの待ち受けの開始・停止の誤りか)
- [x] スタジオの誤りなら直してテストで確かめる。テストの誤りならテストを直す (時間の待ちを足すだけで隠さない)
- [x] 負荷をかけて (例: 別の重いビルドと並べて、または `--test-threads` を増やして) 何度か流し、落ちないことを確かめた
- [x] `just ci` が通る

## 参考

- apps/studio/src-tauri/src/api/、docs/studio.md

## メモ

- T0178 のワーカーの報告から起票した (2026-10-06)。
- 原因 (スタジオの誤り): `Server::stop` (apps/studio/src-tauri/src/api/server.rs) は止める印を立てて自分に接続し accept を起こすだけで、
  accept のスレッドが待ち受けの口 (`TcpListener`。スレッドが持っている) を落とすのを待たずに戻っていた。負荷が高いとスレッドが
  動く前に次の `Server::start` が同じポートに束ねようとして「使用中」で失敗し、`listening` が `false` になる
  (テストの `again.set_api_enabled(false)` → `reloaded.load_api` がちょうどこの順)。テストの誤りではない。
  実際のスタジオでも、ポートを変えずに API を切ってすぐ入れ直す・ブラウザソースと操作 API を切り替えると同じことが起こりうる。
- 直したこと: `Server` が accept のスレッドの `JoinHandle` を持ち、`stop` で終わるのを待つ (accept のスレッドは API の錠を取らないので、
  `sync_listener` が錠を持ったまま待っても行き詰まらない)。あわせて、`start` が途中 (2 つ目のスレッドを作れない等) で失敗したとき、
  先に始めたスレッドを止めるようにした (以前は待ち受けたまま残った)。
- テスト: `api::tests::a_stopped_listener_has_released_its_port` (止めて 50 回すぐに同じアドレスへ束ねる) を足した。
- 負荷での確認: CPU を回す 48 個のプロセスと並べ、`--test-threads 8` で流した。直す前は新しいテストが 10 回中 10 回、
  元のテストが 10 回中 1 回落ちた。直した後は 2 つのテストを 20 回、api のテストすべて (51) を `--test-threads 32` で 3 回流してすべて通った。
- `just ci` は通った (main に rebase した後)。
- 確かめていないこと: Windows・macOS (accept を起こす方法は変えていない。待つのを足しただけ)。
