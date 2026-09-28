---
id: T0303
title: モーションマネージャ・表情・エフェクト
status: done
milestone: M3
crates: [ugoe-motion, ugoe, ugoe-runtime]
depends_on: [T0103, T0302]
assignee: claude-worker-T0303
---

# T0303: モーションマネージャ・表情・エフェクト

## 目的

優先度付きのモーションマネージャ、表情、目パチ・呼吸・リップシンク・ポーズを実装し、`Puppet::update` の適用順を仕様どおりにする。

## 受け入れ条件

- [x] docs/spec/motion.md の各項目にテストがある
- [x] フレーム内の適用順の `> 未決:` を解消する
- [x] `just ci` が通る

## 参考

- `docs/spec/motion.md`

## メモ

- ugoe-motion に `MotionManager` (優先度 `Idle < Normal < Force`、同じ優先度以上なら受け付け、古いモーションは `stop()` でクロスフェード)、`ExpressionManager` (`set` / `add` / `stop` / `stop_all`)、`EyeBlinkPlayer` (SplitMix64、間隔は `[0.5, 1.5) * interval` の一様分布、開き具合を現在の値に掛ける)、`BreathPlayer` (位相を周期で折り返す)、`LipSyncPlayer` (ホストが音量を与える)、`PosePlayer` を追加した。
- `Puppet::update` の適用順: 基準値に戻す → モーション → (T0306 のトラッキング) → 基準値を保存 → 表情 → 目パチ → 呼吸 → 物理 → リップシンク → パーツの不透明度 (カーブとポーズ) → 評価。未決だった「保存した値に戻す」は採用した。ホストが `update` の合間に書き換えた値は、前フレームの出力とビット列で比べて検出し、基準値にする (wasm のスライダーや `reset()` はそのまま動く)。詳細は `docs/spec/motion.md#フレーム内の適用順`。
- `Puppet::play` を `start_motion(motion, priority) -> bool` に置き換えた (使っている箇所はなかった)。`set_expression` / `expressions_mut` / `set_effects` / `set_pose` / `pose_mut` / `set_lip_sync_level` / `motion_events` / `motions(_mut)` を足した。エフェクトとポーズはモデルの `effects` / `pose` から作る。
- **ugoe-runtime の変更 (オーケストレーターの承認済み、`crates:` に追加):** ランタイムにはパーツの不透明度を外から変える手段がなく、`fixtures::pose_and_effects` の `ArmB` のようにキーフォームの不透明度が 0 のパーツを表示できないため、上書き (乗算ではなく) を追加した。 パーツの不透明度のカーブとポーズのために、`ModelInstance::set_part_opacity_override(part, value, weight)` / `part_opacity_override` / `clear_part_opacity_overrides` と `CompiledModel::part_index` を追加した (追加のみ、既存 API は変えていない)。変更フラグは描画可能要素の出力の比較で作るので、上書きで不透明度が変わったメッシュだけが `OPACITY` になる (テスト `part_opacity_override_marks_affected_drawables`)。`own = lerp(補間した値, value, weight)`。`docs/spec/evaluation.md#2-評価の順序` に追記。`Puppet` は上書きを毎フレーム作り直すので、ホストが独自に上書きを使うことは想定していない。
- T0302 から引き継いだ件: パーツの不透明度・エフェクトグループのカーブは実装した (`docs/spec/motion.md#パーツの不透明度`・`#エフェクトグループ`)。長さ 0 のループしないモーションは「何も適用しない」のままにし、仕様に明記した。パラメータの添字のキャッシュはそのまま (1 つの `Puppet` の中でしか使わないので問題ない)。
- やらなかったこと: モーションのキュー (ホストが `is_playing()` を見て次を開始する)、ブレンドシェイプ (T0307)、wasm への公開 (T0309)。
- 後続の候補: T0309 で `start_motion` / `set_expression` / `set_lip_sync_level` / `motion_events` を JS に出す。T0306 は `Puppet::update` の `TODO(T0306)` の位置にトラッキングの書き込みを入れる。
