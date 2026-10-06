---
id: T0482
title: "Godot: UgoePuppet に表示中のポーズの要素を読む関数を足す"
status: done
milestone: M9
crates: [bindings/godot, docs]
depends_on: []
assignee: claude-worker-T0482
---

# T0482: Godot: UgoePuppet に表示中のポーズの要素を読む関数を足す

## 目的

T0478 で、wasm・C ABI・Unity・`@ugoe/runtime` に表示中のポーズの要素を読む口が入った。Godot の `UgoePuppet` は `show_pose` を持つが、
読む口が無い (T0478 の候補)。ほかのバインディングと揃える。

## 受け入れ条件

- [x] `UgoePuppet` に表示中の要素を読む関数 (`visible_pose_parts()` など。グループごとに 1 つ、要素の無いグループは空。並びは wasm の `pose_visible_json` と同じ) がある
- [x] Godot の smoke (か単体テスト) で、`show_pose` の直後とモーションのカーブで切り替わった後の値を確かめる
- [x] bindings/godot/README.md の API の表を直した
- [x] `just ci` と `just godot-check` が通る

## 参考

- T0478 のメモ (wasm の `pose_visible_json` の並び・C ABI の口)
- bindings/godot/src (`show_pose`)、docs/spec/motion.md#ポーズ

## メモ

- T0040 で起票した (元の候補: T0478 (Godot))。

### やったこと

- `UgoePuppet2D.get_visible_pose_parts()` (`PackedStringArray`。Godot の getter の名前に揃えて `get_` を付けた)。グループごとに 1 つで、
  並びは wasm の `pose_visible_json` (`pose_json` の `groups`。付けたモジュールのグループも入る) と同じ。要素の無いグループは空の文字列
  (wasm の `null`・Unity の null に当たる。GDScript の `PackedStringArray` に null を入れられないため)。モデルが無ければ空。
  中身は `src/node/sdk.rs` の `visible_pose_parts` (`ugoe-wasm` の `pose_visible_parts` と同じ組み立て)。
- 単体テスト (`cargo test --manifest-path bindings/godot/Cargo.toml`): `show_pose` の直後・モーションのパーツの不透明度のカーブの要求
  (次の更新の後)・要素の無いグループ・付けたモジュールのグループ。
- smoke (`example/tests/smoke.gd`): pose-select で読み込みの直後と `show_pose` の直後、dress-up の `wave` で `HandsDown` → `Wave` →
  `HandsDown` (開始の直後は変わらず、次の `advance` の後に変わる) と、dress-up での `show_pose`。
- bindings/godot/README.md の API の表と単体テストの説明、docs/spec/motion.md#ポーズ の口の一覧に Godot を足した (英語の spec/ は口の名前を持たないので変えていない)。
- 検査: `just ci`・`just godot-check` (smoke と適合テスト 69 ケース)・`just conformance` (全バインディング)・`just refs`。

### やらなかったこと

- Godot に `show_pose_immediately` (T0329) とポーズの定義を読む口 (wasm の `pose_json`・C ABI の T0483) は無い。範囲外なので足していない。

### 起票の候補

- Godot の `UgoePuppet2D` に `show_pose_immediately` (T0329) とポーズの定義 (グループごとの要素のパーツ。`get_pose_entry_parts()` など) を読む口。
