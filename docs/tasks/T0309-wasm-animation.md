---
id: T0309
title: wasm: 物理・モーション・表情・エフェクトの公開
status: done
milestone: M3
crates: [ugoe-wasm, ugoe, ugoe-format]
depends_on: [T0303, T0305, T0308, T0110]
assignee: claude-worker-T0309
---

# T0309: wasm: 物理・モーション・表情・エフェクトの公開

## 目的

`WebPuppet` から、`Puppet::update` と同じ順で物理・モーション・表情・目パチ・呼吸・リップシンク・ポーズを動かせるようにする。
エディタの物理の設定画面 (T0629) とタイムライン (T0630)、ビューア (T07xx) のプレビューに使う。

## 受け入れ条件

- [x] モーション・表情を JSON で渡して再生・停止できる。物理と各エフェクトを有効・無効にできる。リップシンクの入力値を渡せる
- [x] 物理の設定だけを差し替えられる (エディタで設定を変えながらプレビューするため)
- [x] 手で動かしたパラメータ (スライダー) と、モーション・物理の出力の関係を決めて書く
- [x] 当たり判定を標準の名前 (`Model.avatar.hit_areas` の `Head`・`Body`) で引く API を `Puppet` と `WebPuppet` に足す (T0308 の後続)
- [x] 読み込み時に `validate_physics` を実行する (T0305 の後続。`ugoe-format` または `Puppet` の組み立て)
- [x] TS の型 (`ugoe-wasm` の生成物) と、ネイティブでの単体テスト
- [x] `just ci`・`just wasm-check`・`just wasm` が通る

## 参考

- `docs/spec/motion.md`、`docs/spec/physics.md`
- `crates/ugoe-wasm/src/web.rs`

## メモ

- **SDK (`ugoe`)**: `Toggles { physics, eye_blink, breath, lip_sync, pose }` と `Puppet::set_toggles` / `toggles`
  (止めた段はパラメータ・パーツの不透明度に触らない。物理を止めると動的物理の加算量を 0 に戻し、再開で静止姿勢から)。
  `Puppet::set_physics(Option<PhysicsSettings>) -> Result<(), Vec<ValidationError>>` (`validate_physics` で検査し、
  エラーなら何も変えない)・`physics()`・`reset_physics()`。`with_physics` は検査しないまま (doc に明記)。
  標準の当たり判定: `standard_hit_area(name)`・`hit_test_standard(name, p)`・`standard_hit_areas_at(p)`。
- **ugoe-format**: `read_bundle` が `PHYS` を `validate_physics` で検査し、エラーなら `FormatError::InvalidPhysics`
  (モデルの `Invalid` と同じ扱い)。単体 JSON の `physics_from_json` / `physics_to_json` / `motion_from_json` /
  `expression_from_json` を追加。
- **ugoe-wasm**: GPU に触れない `animation` モジュール (JSON の受け渡し・名前の解決・読み直し時の引き継ぎ。
  ネイティブでテスト) と、JS へ出す `src/web/animation.rs` (`WebPuppet` の別の `impl` ブロック)。
  `WebPuppet` は元から `Puppet` を包んでいたので、`update(dt)` は `Puppet::update` そのもの。
  API: `motion_names`・`start_motion(name, priority?)`・`start_motion_json`・`stop_motions`・`is_motion_playing`・
  `motion_events`・`expression_names`・`set_expression`・`set_expression_json`・`stop_expressions`・
  `set_effects_json`・`set_lip_sync_level`・`set_enabled(stage, bool)`・`is_enabled`・`set_physics_json(json?)`・
  `physics_json`・`has_physics`・`is_dynamic_physics`・`reset_physics`・`standard_hit_area`・`hit_test_standard`・
  `standard_hit_areas_at`。`load_bundle` はバンドルの物理・モーション・表情を読む。`load_json` (エディタの読み直し) は
  物理の設定・一覧・段の有効無効を引き継ぎ、新しいモデルと合わない物理は外して警告を返す。
- **スライダーとの関係** (`docs/spec/motion.md#手で動かしたパラメータとの関係`): T0303 の「合間の書き換えは基準値」から、
  モーションの対象は上書きされる (再生中の書き換えは失われる)、表情・目パチ・呼吸は重なる、物理の入力は物理が読む、
  物理の出力は物理が上書きする (見るには物理を止める)、と表にした。
- 仕様書: `docs/spec/motion.md` (段の有効無効・スライダーとの関係)、`evaluation.md` (当たり判定の未決を解消し API を追記)、
  `physics.md` (検証)、`file-formats.md` (読み込み時の検査と JSON の API)。`crates/ugoe-wasm/README.md` も更新。
- 範囲外の変更: `apps/editor/src/lib/viewport/ugoe-wasm-stub.d.ts` に新しいメソッドの型を足した (`just wasm` の生成物と一致)。
  ビューアのスタブはビューアが使うものだけを書く方針なので触っていない。
- やらなかったこと・後続の候補
  - モデルを読み直すと、モーション・表情の再生状態は初めからになる (エディタのタイムライン T0630 で編集のたびに
    読み直すなら、`Puppet` の再生状態を引き継ぐ仕組みか、モデルを差し替えずに済む経路が要る)。
  - バンドルの一覧へ JS からモーション・表情を登録する API (今は JSON を直接再生する)。
  - エフェクトの設定の取得 (`effects_json`)。`Puppet` がエフェクトの設定を返さないので出していない。
  - ビューアの UI (モーション・表情の一覧・当たり判定のタップ) は T07xx。
