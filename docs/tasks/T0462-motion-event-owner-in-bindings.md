---
id: T0462
title: モーションのイベントを発火したモーション (名前と番号) を C ABI・wasm・Unity・Godot から読む (判断が要る)
status: done
milestone: M9
crates: [ugoe-ffi, ugoe-wasm, bindings/web, bindings/unity, bindings/godot, docs]
depends_on: [T0461]
assignee: claude-worker-T0462
---

# T0462: モーションのイベントを発火したモーション (名前と番号) を C ABI・wasm・Unity・Godot から読む (判断が要る)

## 目的

T0821 で SDK のモーションのイベントに出どころ (どのモーションか) を持たせたが、組み込み先 (C ABI・wasm の `motion_events`・Unity・Godot) からは
イベントの名前しか読めない (T0821 の候補)。

**判断が要る:** 口の形。wasm は `motion_events` の戻り値を変える (互換を壊す) か、別の関数 (`motion_events_json` など) を足すか。
C ABI は `ugoe_puppet_motion_event_*` に添字で読む関数を足す形か。先に形を決めてから、4 つの組み込み先を揃える。

## 受け入れ条件

- [x] 口の形を決め、C ABI・wasm の README に書いた (利用者の判断を待つなら blocked にしてメモに案を書く)
- [x] C ABI・wasm・Unity・Godot からイベントのモーションの名前と番号を読め、それぞれテストがある (`@ugoe/runtime` の型も)
- [x] 適合テストにイベントの出どころを比べるケースがある
- [x] `just ci`・`just wasm-check`・`just unity-check`・`just godot-check`・`just conformance` が通る

## 参考

- T0821 のメモ (起票の候補)
- crates/ugoe-ffi/src/animation.rs、crates/ugoe-wasm/src/animation.rs

## メモ

- T0026 で起票した (元の候補: T0821)。C ABI・wasm の `animation.rs` を触る T0461 の後に着手する。大きければ「口の形 (C ABI・wasm)」と「Unity・Godot」に分ける。
- T0462 (claude-worker-T0462) のメモ:
  - **口の形 (決めたこと):** すべて「足すだけ」にし、既存の関数・型・シグナルは変えていない。判断の根拠は既存の規則で、
    C ABI は crates/ugoe-ffi/README.md#abi-の版 (T0409) の「構造体にフィールドを足したくなったら、既存の構造体は変えずに別の関数や構造体を足す」に
    そのまま従い (`UgoeMotionEvent` に足すと大きさが変わり ABI を壊す)、ほかの組み込み先もそれに揃えた。利用者の判断を待たずに進めた。
    | 組み込み先 | 足したもの |
    | --- | --- |
    | C ABI | `UgoeMotionEventSource { motion, number }` と `ugoe_puppet_motion_event_source(puppet, index, out)` (`index` は `ugoe_puppet_motion_event` と同じ。ABI の版は上げない) |
    | wasm | `WebPuppet.motion_events_json()` → `[{value, time, motion, number}]` (`motion_events(): string[]` はそのまま。形のある値は `*_json()` の既存の口に揃えた) |
    | `@ugoe/runtime` | `motionEvents(puppet)` と型 `UgoeMotionEvent` (`QUERY_KEYS` の `motion_event` も) |
    | Unity | `MotionEvent.Motion`・`Number` (`ulong`)。コンストラクタは省略できる引数で足した (`MotionEventFired` もそのまま値を持つ) |
    | Godot | `get_motion_events()` (`{value, time, motion, number}` の配列) とシグナル `motion_event_fired(value, time, motion, number)`。`motion_event(value)` は変えない (引数を変えると 1 引数でつないだ関数が動かなくなるため) |
  - テスト: ffi `motion_event_sources_name_the_motion_that_fired` (始め直してフェードアウト中のモーションの番号・範囲外・NULL)、
    wasm `motion_events_json_names_the_motion_that_fired`、`@ugoe/runtime` の `motionEvents` と `QueryKeyCheck`、Unity `MotionEventsNameTheMotionThatFired`
    (spec/examples/motion-crossfade の列)、Godot の smoke.gd (`get_motion_events` とシグナル)。
  - 適合テスト: ケースの `motions` (`{step, name, priority, accepted}`) と結果の `motion_events` を足し、ケース `motion-events-crossfade`
    (motion-crossfade のバンドルを 0.25 秒刻み。wave に替えた後のフェードアウト中の idle のイベントが 1 番、始まらない要求は番号を取らない、force の idle が 3 番) で
    C ABI・wasm (WebGL2・WebGPU)・Unity・Godot の出どころを参照と比べる (`plan.txt` は `start` の行)。`queries` に `motion-events` の項目を足した。
  - 文書: crates/ugoe-ffi/README.md・crates/ugoe-wasm/README.md (口の形と理由)、bindings/web・unity・godot の README、tests/conformance/README.md、
    docs/spec/motion.md#モーションマネージャ (「まだ出していない」を組み込み先の口に)。英語の spec/motion.md は組み込み先を書いていないので変えていない。
  - 通した検査 (main に追従後、2026-10-05): `just ci`・`just wasm-check`・`just ffi-check`・`just refs`・`just conformance` (66 ケース、すべてのバインディング)・
    `just unity-check` (94 件)・`just godot-check` (smoke と 66 ケース)。
  - やらなかったこと: packages/player (ビューア) の `motionEvents` 表示は値だけのまま (範囲外。出どころを見せるなら別タスク)。Unity は実際の Unity では
    確かめていない (dotnet の Runtime/Core のテストと Unity の部分のスタブの型検査だけ)。
  - 起票の候補: ビューア (packages/player) のモーションのイベントの欄に、発火したモーションの名前と番号を出す。
