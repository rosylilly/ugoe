---
id: T0135
title: ランタイム: 受け手でもモーションのパーツの不透明度のカーブを当てる
status: done
milestone: M8
crates: [ugoe, ugoe-ffi, ugoe-wasm, bindings/web, tests/conformance, apps/studio, packages/player, docs]
depends_on: [T0134, T0803]
assignee: claude-worker-T0135
---

# T0135: 受け手のパーツの不透明度

## 目的

`Puppet::follow` はパラメータの値だけを受け取るので、モーションのパーツの不透明度のカーブ (パーツを切り替えるモーション) が受け手 (出力ウィンドウ・ブラウザソース・
仮想カメラ) に出ない (T0803 のメモ)。ADR 0028 の「受け手は 1 つの Puppet と同じ形」を守る。

## 受け入れ条件

- [x] 指揮役が配るフレームに、モーションが上書きしたパーツの不透明度を足す (変わったパーツだけ・疎な形でよい)。受け手の `follow` は 1 つの Puppet と同じ段で当てる
- [x] パーツの不透明度のカーブを持つモーションで、指揮役と受け手の組み合わせが 1 つの Puppet と一致するテスト (適合テストのケースも足す)
- [x] C ABI・wasm・スタジオの指揮役と受け手を合わせ、docs/spec/motion.md の「受け手として進める」と docs/studio.md のフレームの形を更新する
- [x] `just ci`・`just wasm-check`・`just ffi-check`・`just conformance --only c,wasm` が通る

## 参考

- docs/tasks/T0803-studio-inputs.md・T0134-follow-late-stages-order.md のメモ

## メモ

- **やったこと:**
  - `Puppet::part_opacities_applied()` (直前の `update` でモーションのパーツの不透明度のカーブがパーツに求めた値と重み。カーブのあるパーツだけ、
    パーツの番号 (モデルの `parts` の順) で。`PartOpacityInput { part, value, weight }`) を足し、`Puppet::follow(values, lip_sync, part_opacities, dt)`
    にした。受け手は 10. で届いたカーブを `update` と同じに扱う (ポーズの要素へは受け手のポーズへの表示の要求、ほかは上書き)。NaN は
    `ParameterValuesError::PartOpacityNaN` で何も変えず、モデルに無い番号は無視する。`update` の挙動と数値は変えていない
    (`apply_part_opacities` がカーブの並びを受け取るようにしただけ)。着せ替えの付け外しの後は記録を空にする。
  - C ABI: `UgoePartOpacityInput` と `ugoe_puppet_follow(..., lip_sync_strength, part_opacities, part_opacity_count, dt)` (公開した版が無いので
    ABI の版は上げず、T0134 と同じ扱い)。ヘッダは cbindgen の CLI で作り直した。wasm: `follow(values, dt, level?, strength?, parts?: Uint32Array,
    opacities?: Float32Array)` (値と重みを交互に。数が合わなければ例外)。`bindings/web` のスタブと README、各クレートの README を揃えた。
  - スタジオ: フレームのバイト列の値の後に、パーツの不透明度の数・番号 (u32)・値と重み (f32 の組) を足した (docs/studio.md の表)。
    指揮役が `part_opacities_applied` を載せ、Rust の受け手 (`follower.rs` の `apply_frame`) と TS の受け手 (`packages/player/src/follower.ts` の
    `decodeFrame`・`applyFrame`。`Uint32Array`・`Float32Array` を写さずに重ねる) が `follow` に渡す。GOLDEN のバイト列を両方で更新。
  - テスト: `crates/ugoe/src/follow.rs` (ポーズのクロスフェード・フェードの重み・途中で止めたときの戻りで、指揮役 + 受け手と 1 つの Puppet の
    パーツの不透明度が一致。カーブを渡さない受け手は合わない。記録の形と入力の検査)、`crates/ugoe-ffi/tests/extended.rs`、
    `apps/studio/src-tauri/tests/follow_match.rs` (ポーズのモデルで指揮役がパーツを切り替えるモーションを再生し、パーツの不透明度の上書きも比べる)。
  - 適合テスト: 生成するモデル `part-switch` (ポーズの要素とポーズに属さない `Badge` にメッシュ。バンドルにモーション `switch`) とケース
    `part-switch-follow`、`cases.json` の `motion` (指揮役が再生するモーション)、計画の `follow_part_opacities`、`plan.txt` の `follow` の行に
    パーツの数と組を足した。c・wasm とも通った (57 ケース)。カーブを渡さない実行器は合わないことを単体テストで確かめた。Unity と Godot は
    今までどおり `follow` のケースを飛ばす。
  - docs/spec/motion.md#受け手として進める・docs/studio.md のフレームの形を更新。公開仕様 (`spec/`) は変えていない (T0133・T0134 と同じ判断。
    適用順は同じで Puppet の API の口を足しただけ)。
- **確かめたこと:** `just ci`・`just wasm-check`・`just ffi-check`・`just conformance --only c,wasm`、`@ugoe/player` と `@ugoe/studio` の test と check。
- **確かめていないこと:** wasm-webgpu・Unity・Godot、実際のスタジオの出力ウィンドウでの見え方 (ヘッドレスの Chrome での画面の確認はしていない)。
  仮想カメラ (T0807) の受け手は main にまだ無いので、入ったら `follower.rs` の `apply_frame` を通っているかを確かめる (通っていれば変更は要らない)。
- **制限:** 着せ替えの直後に前のシーンのフレームを付け直したモデルに当てると、モジュールのパーツの番号はずれうる (本体のパーツは変わらない。
  docs/studio.md に書いた)。
- **範囲外の変更:** `packages/player` (`follower.ts`・`follower.test.ts`・`wasm.ts`)。wasm の受け手のフレームの読み取りと `follow` の呼び出しが
  ここにあるため (team-lead に相談済み。返事待ちのまま review に出した)。
- **後続の候補:** Unity・Godot に `follow` を出す (T0133 のメモと同じ)。着せ替えをまたぐパーツの番号のずれを無くすなら、`scene` にパーツの ID の
  並びを載せて受け手が引き直す。
- 範囲外の変更 (team-lead が承認): packages/player の follower.ts・follower.test.ts・wasm.ts (wasm の受け手のフレームの形)。仮想カメラ (T0807) の受け手は follower.rs の Follower を通るので、rebase の後もパーツの不透明度が当たる (team-lead が確認)。
