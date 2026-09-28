---
id: T0313
title: 物理の出力の加算モード
status: done
milestone: M3.5
crates: [ugoe-model, ugoe-physics, ugoe-editor-core, ugoe-format, ugoe-cli, ugoe, apps/editor, docs]
depends_on: [T0312]
assignee: claude-worker-T0313
---

# T0313: 物理の出力の加算モード

## 目的

Live2D の実践では、トラッキングの値と物理の揺れを両立させるため、体のパラメータを「トラッキング用」と「物理用」に複製し、両方のキーを付けている (参考の記事 4)。
ugoe の物理のパラメータへの出力は、今の値を物理の値へ置き換える (`param + (value - param) * weight`) ので、同じ回避策が要る。
出力に「加算」を選べるようにし、トラッキングの値 (や手で動かした値) に物理の揺れを足せるようにする。

## 受け入れ条件

- [x] `PhysicsOutput` に合成の方法 (`overwrite` (既定、今と同じ) / `add`) を足す。`add` は物理を進める前の値 (フレーム内の順序で保存した基準値) に `(value - 静止時の値) * weight` を足し、値域にクランプする。フレームごとに累積しないこと。古いデータは `overwrite` として読む
- [x] トラッキングの `AngleX` から、同じ `BodyAngleX` に「トラッキング × 係数 + 物理の遅れ」を出すテスト (パラメータを複製しない)
- [x] エディタの物理のパネルで出力ごとに選べる
- [x] `docs/spec/physics.md`・`docs/spec/model.md`・`docs/spec/file-formats.md` (未知の値の扱い)。`just ci`・`just gen`・`just wasm-check` が通る

## 参考

- https://note.com/himono_vtuber/n/n9015f0030d53
- `docs/spec/motion.md#フレーム内の適用順`

## メモ

- **データ:** `PhysicsOutput::blend: PhysicsBlend` (`overwrite` 既定 / `add` / `Unknown`)。省略と `overwrite` は書き出さない。
  未知の値は `overwrite` として扱い (`effective()`)、`unknown::in_physics` が場所を返す。基準の機能 `physics.additive_output` を足した。
- **静止時の値:** そのステップの入力 (根元・回した重力・風) での静止姿勢 (`chain::rest_offset`) から、同じ `source`・`scale`・`reflect` で求めた値。
  衝突体は考えない。`outputs()` が `add` の出力について `value - rest` を返し、`blend_parameter` が `clamp(param + ずれ * weight)` を書く。
  静止していれば足す量は 0 (頭を傾けたまま・風があっても)。
- **累積しない:** `Puppet` はフレームごとに基準値へ戻すので累積しない。`PhysicsSystem::update` を直接使うホストは毎フレーム値を書き直す必要がある (physics.md に明記)。
- **中継:** 写しにも同じ規則で書くので、後ろのリグは足した後の値を読む (写しは各ステップの始めにフレームの始めの値へ戻すので累積しない)。
- **動的物理の出力先** は常に加算量なので `blend` を使わない (エディタでもパラメータの出力にだけ選択欄を出す)。
- **テスト:** フィクスチャ `physics_additive` / `physics_additive_settings` (AngleX → BodyAngleX、add)。`ugoe-physics/src/additive_tests.rs`
  (静止時 0・重力と風・トラッキング × 0.3 + 遅れと行き過ぎ・重み・クランプ・未知の値・中継)、`crates/ugoe/tests/m3_puppet.rs` の
  `physics_adds_sway_on_top_of_tracking` (実際のトラッキングで)、エディタの undo と未知の値の拒否、Vitest。
- **`crates:` に足したもの (オーケストレーターの承認済み):**
  - `ugoe-format`: テストの構造体のリテラルに `blend` を足した (フィールドの追加で必要)。基準の機能 `physics.additive_output` を `features.rs` に足した (仕様書の機能の表と一致させる)。
  - `ugoe-cli`: 見本のアバターを作る `sample.rs` の構造体のリテラルに `blend` を足した (`overwrite` は書き出さないので `assets/samples/avatar.ugoe` は変わらない)。
  - `ugoe` (テストだけ): `m3_puppet.rs` の構造体のリテラルに `blend` を足し、実際のトラッキングで確かめるテスト `physics_adds_sway_on_top_of_tracking` を足した。
- **後続の候補:** 見本のアバター (`assets/samples/avatar.ugoe`) の体の物理を `add` にして、トラッキングの体の回転と両立させる。
