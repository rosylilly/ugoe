---
id: T0323
title: モーションと表情の数値の検証 (NaN・無限大・負の長さ)
status: done
milestone: M9
crates: [ugoe-model, ugoe-format, ugoe-cli, docs]
depends_on: [T0317, T0141]
assignee: claude-worker-T0323
---

# T0323: モーションと表情の数値の検証 (NaN・無限大・負の長さ)

## 目的

モーション (`ugoe_model::motion::Motion`) と表情 (`ugoe_model::expression::Expression`) の数値は、どこでも検証していない
(`ugoe_model::validate` はモデルのエフェクト・ポーズ・物理だけを見る)。MessagePack のバンドルは NaN と無限大を表せるので、
カーブの点の値が NaN のモーションを再生すると、パラメータが NaN になり、描画まで壊れる。T0317 のレビューで見つけた。

ランタイム (`ugoe-motion`) は T0317 で、NaN の `dt`・NaN の `duration`・極端に短い `duration` で止まらないようにしたが、
データの値そのものの検証は読み込み・検証の側で行う。

## 受け入れ条件

- [x] モーションの検証を足す: `duration`・`fade_in`・`fade_out`・カーブの `fade_in` / `fade_out`・セグメントの点 (`t`・`v`、ベジェの制御点)・イベントの `t` が有限値であること。`duration` が負でないこと。セグメントの時刻が減らないこと (警告かエラーかを決める)
- [x] 表情の検証を足す: `fade_in`・`fade_out`・各項目の `value` が有限値であること
- [x] 検証をどこで呼ぶか決める (`ugoe validate`・バンドルの読み込み・`Puppet` にモーションを渡すとき)。読み込みで拒むなら前方互換 (ADR 0013) との関係を書く
- [x] `docs/spec/model.md` の検証の節と、英語の `spec/model.md#model-validation` を同じ PR で直す
- [x] `just ci` が通る

## 参考

- docs/tasks/T0317-review-motion.md (メモの所見の表)
- docs/spec/motion.md#カーブ、docs/spec/model.md#検証

## メモ

- T0008: 数値の有限性の検証の方針 (エラーにするかランタイムのフォールバックに任せるか) は T0141 (T0145・T0318 をまとめた) で決める。
  このタスクはデータ (モーション・表情) と呼び出す場所 (`ugoe-format`・`ugoe-cli`・`Puppet`) が違うのでまとめず、T0141 の方針に揃えて後で行う。

### 決めたこと (詳細は `docs/spec/model.md#モーションと表情の検証`)

- T0141 の方針に揃えた: 有限でない値はすべて `InvalidValue`。範囲は `duration` が 0 以上だけをエラーにした。
  負のフェード (フェードなし)・区間外のベジェの制御点 (クランプ)・長さの外や負の時刻のイベントと点はランタイムの扱いが決まっているのでエラーにしない。
- **時刻が減る点は警告** にした。評価は決まっている (`to.t <= p0.t` の区間は終点の値。`SegmentIndex` も同じ結果を保証している) ので
  方針の「エラーにしない」側。公開仕様の SHOULD NOT のまま、`motion_warnings` (`MotionWarning::DecreasingTime`) で `ugoe validate` が警告する。
- 呼ぶ場所: バンドルの読み込み (`InvalidMotion { name, errors }`・`InvalidExpression { name, errors }`)、`motion_from_json`・`expression_from_json`
  (モデルの JSON と違い読み込みで検査する。ビューア・wasm・C ABI・`ugoe render --motion` が読んだものをそのまま再生するため)、`ugoe validate`。
  `Puppet` は検査しない (公開 API を変えずに済み、ファイルからのものは検査済み)。
- 前方互換: 未知のセグメント・対象・`op` は検査しない (点がない、または `value` は有限の `f32` のまま) ので、新しい版のファイルを拒まない。仕様に書いた。

### やったこと

- `ugoe-model`: `validate/motion.rs` に `validate_motion`・`validate_expression`・`motion_warnings`・`MotionWarning`。テスト 6 つ。
- `ugoe-format`: `FormatError::InvalidMotion`・`InvalidExpression` を足し、`read_bundle` と JSON の読み込みで検査。テスト (MessagePack の NaN・無限大、JSON の `1e39`・負の長さ)。
  `tests/spec_bundles.rs` の読み込みの段の対応に新しいエラーを足した。
- `ugoe-cli`: `ugoe validate` がモーション・表情のエラーを `error: <名前>: ...`、時刻が減る点を警告で書く。テスト `validate_checks_motions_in_a_bundle`。README。
- 仕様: `docs/spec/model.md` (方針の節・新しい節)・`docs/spec/file-formats.md`・`spec/model.md` (Data conventions・Validation・Motion and expression validation)・
  `spec/bundle.md` (Reading の 7. Validation と JSON の読み込み)・`docs/spec/public-spec.md` の対応表。
- `just ci` は通った (exit 0)。ランタイム層 (`ugoe-model`・`ugoe-format`) を触ったので `just wasm-check` も通した。

### やらなかったこと・起票の候補

- エディタのメッセージコード: `ugoe-editor-core` の `message_codes.rs` は `FormatError` の未知の変種を `format.corrupt` (詳細は `to_string`) にまとめるので、
  新しい 2 つはそこに入る。`format.invalid_motion`・`format.invalid_expression` のコードと翻訳を足すのは起票の候補 (範囲外のため触らなかった)。
- `Puppet::start_motion` などで検査する (デバッグ時の assert か、`Result` を返す) のは起票の候補。公開 API の変更になるので見送った。
- エディタのプロジェクト (`.ugoeproj`) のモーション・表情は `ugoe-editor-core` の `project/files.rs` が `serde_json` で直接読むので検査されない (書き出しのバンドルは読み直せば拒まれる)。`validate_motion`・`validate_expression` を通すのは起票の候補 (範囲外)。
- `spec/examples` に、検証で拒まれるモーションの例 (`error: validation`) を足すのは起票の候補。
