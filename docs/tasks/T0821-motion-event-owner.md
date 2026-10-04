---
id: T0821
title: "モーションのイベントに、発火したモーションを載せる (スタジオの motion.event の名前)"
status: todo
milestone: M9
crates: [ugoe-motion, ugoe, apps/studio, docs]
depends_on: [T0814]
assignee:
---

# T0821: モーションのイベントに、発火したモーションを載せる

## 目的

T0814 のレビューで見つけた。`MotionManager::fired_events` (`ugoe-motion`) はフェードアウト中のモーションのイベントも返すが、
どのモーションのイベントかを返さない。スタジオの外部 API の `motion.event` は、刻みを始めたときに再生していたモーションの名前を
付けて送る (T0814 で、その刻みで終わったモーションのイベントは正しい名前になった) ので、新しいモーションに替えた直後に
前のモーションがフェードアウトしながら発火したイベントは、新しいモーションの名前で届く。

## 受け入れ条件

- [ ] 発火したイベントと、それを発火したモーション (名前か、始めた順の番号) を `Puppet` から取れる
      (公開 API の変更。`ugoe-ffi`・`ugoe-wasm` に出すかを決める)
- [ ] スタジオの `motion.event` がそれを使う (`apps/studio/src-tauri/src/conductor.rs` の `FiredEvent`)
- [ ] フェードアウト中のモーションのイベントの名前のテスト
- [ ] `just ci` が通る

## 参考

- `crates/ugoe-motion/src/manager.rs`、`apps/studio/src-tauri/src/api/mod.rs` の `api_animation`
- docs/studio-api.md#events

## メモ
