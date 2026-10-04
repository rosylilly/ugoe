---
id: T0821
title: "モーションのイベントに、発火したモーションを載せる (スタジオの motion.event の名前)"
status: done
milestone: M9
crates: [ugoe-motion, ugoe, apps/studio, docs]
depends_on: [T0814]
assignee: claude-worker-T0821
---

# T0821: モーションのイベントに、発火したモーションを載せる

## 目的

T0814 のレビューで見つけた。`MotionManager::fired_events` (`ugoe-motion`) はフェードアウト中のモーションのイベントも返すが、
どのモーションのイベントかを返さない。スタジオの外部 API の `motion.event` は、刻みを始めたときに再生していたモーションの名前を
付けて送る (T0814 で、その刻みで終わったモーションのイベントは正しい名前になった) ので、新しいモーションに替えた直後に
前のモーションがフェードアウトしながら発火したイベントは、新しいモーションの名前で届く。

## 受け入れ条件

- [x] 発火したイベントと、それを発火したモーション (名前か、始めた順の番号) を `Puppet` から取れる
      (公開 API の変更。`ugoe-ffi`・`ugoe-wasm` に出すかを決める)
- [x] スタジオの `motion.event` がそれを使う (`apps/studio/src-tauri/src/conductor.rs` の `FiredEvent`)
- [x] フェードアウト中のモーションのイベントの名前のテスト
- [x] `just ci` が通る

## 参考

- `crates/ugoe-motion/src/manager.rs`、`apps/studio/src-tauri/src/api/mod.rs` の `api_animation`
- docs/studio-api.md#events

## メモ

- **やったこと (追加だけで、既存の型とシグネチャは変えていない):**
  - `ugoe-motion`: `MotionManager` が開始したモーションに 1 から番号を振る (開始しなかった要求では進まない)。
    `motion_numbers()` (`players()` と同じ並び)・`current_motion_number()`・`fired_event_sources()` (`fired_events()` と同じ並びの
    `EventSource { number, name }`) を足した。`EventSource` を再輸出。テスト `event_sources_name_the_motion_that_fired`・
    `motion_numbers_follow_the_players`。
  - `ugoe`: `Puppet::motion_event_sources()` を足した (`ugoe::motion::EventSource`)。
  - スタジオ: `FiredEvent` を「刻みを始めたときのモーション」から「発火したモーションの名前と、刻みの後も今のモーションか (`current`)」に
    変えた。今のモーションでないもの (終わった・替えられた) のイベントは、従来どおりその刻みの `motion.finished` の前 (無ければ先頭) に送る。
    使わなくなった `Animation::current_motion`・`motion_start` を消した。先に失敗するテスト `events_of_a_motion_fading_out_carry_its_name`
    (テストのモデルに `Bow` を足した) を書いてから直した。
  - 文書: docs/spec/motion.md#モーションマネージャ、spec/motion.md の Playing several motions (Informative の 1 行。新しい MUST/SHOULD は足していない)、
    docs/studio.md・docs/studio-api.md の `motion.event`。`spec/` はタスクの `crates:` の外だが、AGENTS.md の同期の規則で同じ PR に入れた。
- `just ci` と `just wasm-check` は通った (main に追従済み、2026-10-04)。
- **決めたこと:** `ugoe-ffi`・`ugoe-wasm` (と Unity・Godot) には今は出さない。スタジオは Rust の `Puppet` を直接使うので要らず、
  出すと C ABI の構造体 (`UgoeMotionEvent`) か新しい関数と、TS・C#・GDScript の型に波及する。必要になったら別タスクで。
- **起票の候補:** C ABI・wasm (`motion_events` の JSON に `motion` と番号を足す、または別の関数)・Unity・Godot にイベントの出どころを出す。

