---
id: T0134
title: ランタイム: 受け手でも物理より後の段 (リップシンクなど) を同じ順に当てる
status: done
milestone: M8
crates: [ugoe, ugoe-ffi, ugoe-wasm, bindings/web, tests/conformance, docs]
depends_on: [T0133]
assignee: claude-worker-T0133
---

# T0134: 受け手の段の順をそろえる

## 目的

T0133 の `follow` は、指揮役が送った値 (リップシンクを済ませた後の値) を基準値にして物理を回す。1 つの Puppet では物理がリップシンクの前の値を読むので、
リップシンクの対象を物理の入力にするモデルでは、受け手の揺れが 1 つの Puppet と一致しない (T0133 のメモ・docs/spec/motion.md の制限)。
スタジオはマイクの口パク (T0803) を指揮役で当てるので、ADR 0028 の「受け手は物理まで同じ形」を守るために順序をそろえる。

## 受け入れ条件

- [x] 案 C (T0133 の担当の提案を team-lead が採用): フレームを {dt, values, lip_sync} にする。values は物理の段に入る直前の値 (Puppet に `values_before_physics()` を足す)、
      lip_sync は音量と強さの 2 つの f32。受け手の `follow(values, lip_sync, dt)` は、基準値の書き込み → 物理 → 自分のモデルの設定でリップシンク → ポーズ → 評価の順に進める
      (Puppet::update と同じ段の順)。既存の `update` と `set_base_values` の挙動は変えない
- [x] リップシンクの対象を物理の入力にするモデルで、指揮役と受け手の組み合わせが 1 つの Puppet と許容誤差内で一致するテスト (適合テストのケースも足す)
- [x] docs/spec/motion.md の制限の記述を消し、docs/studio.md のフレームの形を更新する
- [x] `just ci`・`just wasm-check`・`just ffi-check`・`just conformance --only c,wasm` が通る

## 参考

- docs/tasks/T0133-runtime-explicit-base-values.md のメモ、docs/adr/0028-studio.md

## メモ

- **やったこと:**
  - `Puppet::values_before_physics()` (直前の `update` で 8. 物理に入る直前の値) と `Puppet::lip_sync_applied()` (9. で当てた音量と強さ。
    `LipSyncInput { level, strength }`) を足した。`update` は値を写して覚えるだけで、挙動は変えていない。`set_base_values` も変えていない。
  - `Puppet::follow(values, lip_sync: Option<LipSyncInput>, dt)` にした: 基準値の書き込み → 物理 → リップシンク (受け手のモデルの対象に、渡した
    音量と強さで当てる。受け手自身の音量は使わず、変えない) → ポーズ → 評価。リップシンクの段は `Toggles::lip_sync` に従う。NaN の入力は
    `ParameterValuesError::LipSyncNaN` で、何も変えない。
  - C ABI: `ugoe_puppet_follow(puppet, values, count, lip_sync_level, lip_sync_strength, dt)` (音量が負ならリップシンクなし。
    `ugoe_puppet_set_lip_sync_level` と同じ約束)。T0133 で足したばかりで公開した版が無いので、ABI の版は上げずに引数を変えた。
    wasm: `follow(values, dt, lip_sync_level?, lip_sync_strength?)` (省いた強さは 1。T0133 の呼び方はそのまま使える)。
    スタブ・README・ヘッダを揃えた。指揮役側の 2 つの口は Rust だけに出した (指揮役はスタジオのバックエンドのネイティブの `Puppet`)。
  - テスト: `crates/ugoe/src/follow.rs` で、リップシンクの対象を物理の入力 (`AngleX`) にし、強さ 0.5 と音量を揺らして、指揮役 + 受け手が
    1 つの Puppet と許容誤差内で一致すること (静的と動的の物理)、T0133 の形 (リップシンク後の値を配る) では合わないことを確かめた。
    C ABI のリップシンクの当て方も `Puppet::follow` と比べた。
  - 適合テスト: 生成するモデル `lip-sync-hair` (動的物理の髪で、リップシンクの対象が `AngleX`) とケース `lip-sync-follow` を足した。
    計画に各フレームの `follow_lip_sync` (`[音量, 強さ]`) を持たせ、`plan.txt` の `follow` の行に音量と強さを足した。c・wasm とも通った (56 ケース)。
    リップシンクの入力を渡さない実行器は合わないことを単体テストで確かめた。
  - `docs/spec/motion.md#受け手として進める` の制限の記述を消して新しい順を書き、`docs/studio.md` の `frame` に `lip_sync` を足した。
- **確かめていないこと:** wasm-webgpu・Unity・Godot (Unity と Godot は T0133 と同じく `follow` のケースを飛ばす)。
- **メモ:** ADR 0028 の 3. には「物理を切った `update` の後の値 (物理の前の値 + リップシンク) を配る」とある。決定 (指揮役が値と `dt` を配り、受け手が物理を回す) は
  変えていないので ADR は直さず、配る値の形は docs/studio.md と docs/spec/motion.md を正にした。必要なら ADR に追記を。
- **後続の候補:** Unity・Godot に `follow` を出す (T0133 のメモと同じ)。

