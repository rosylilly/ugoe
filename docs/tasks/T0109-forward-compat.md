---
id: T0109
title: 前方互換: 機能の宣言と未知の値の読み込み
status: done
milestone: M2
crates: [ugoe-model, ugoe-format, ugoe-runtime, ugoe-render, ugoe-motion, ugoe-editor-core]
depends_on: []
assignee: claude-worker-T0109
---

# T0109: 前方互換: 機能の宣言と未知の値の読み込み

## 目的

ADR 0013 の仕組み (機能 ID の宣言、`FEAT` チャンク、拡張可能な enum の `Unknown`、フォールバック) を実装する。以降の機能追加はすべてこの仕組みに乗るので、早めに入れる。

## 受け入れ条件

- [x] 機能 ID の一覧とフォールバックを定数として持ち、`docs/spec/file-formats.md` の表と一致させる (テストで検査する)
- [x] `FEAT` チャンクの読み書き。知らない必須機能は `UnsupportedFeatures` エラー、知らない任意機能は警告の一覧を返す
- [x] 合成モード・デフォーマの種類・マスクの結合・カーブの対象など、拡張されうる enum に `Unknown` を追加し、未知のタグを読んでもエラーにならない
- [x] ランタイムとレンダラが `Unknown` にフォールバック (恒等変換・通常の合成・和集合など) を適用する
- [x] 書き出し時に、使っている機能を自動で宣言する (1.0 の基準の機能は宣言しない。ADR 0018)
- [x] 1.0 以降の機能を想定した試験用の機能 ID で、任意機能のフォールバックと必須機能の拒否を確かめる
- [x] 未知のタグ・未知の機能を含むバンドルを読むテストがある
- [x] `just ci` が通る

## 参考

- `docs/adr/0013-forward-compatibility.md`
- `docs/spec/file-formats.md#機能の宣言と段階的な劣化`

## メモ

- 機能の一覧は `ugoe_format::features::FEATURES` (`Feature { id, description, fallback, baseline }`)。
  テスト `registry_matches_spec_tables` が `docs/spec/file-formats.md` の 2 つの表を読んで一致を検査する
  (内容の列は他の文書へのリンクを除いて比べる)。現在の機能はすべて基準 (`baseline: true`) で、宣言しない。
- `FEAT` のペイロードは `{ features: [{ id, required }] }` の MessagePack。書き出しは `features::detect`
  (自動検出のフック。今は常に空) の宣言があるときだけ `FEAT` を先頭に書く。読み込みは `FEAT` を他のチャンクの
  デコードより先に調べる。警告は `read_bundle_with_report` の `ReadReport` で返す (`read_bundle` は従来どおり)。
  試験用の機能 ID は `write_with_features` / `read_with_known` (crate 内部) で確かめる。
- `Unknown` はすべてユニットバリアントで、元のタグと中身は捨てる。単純な enum と内部タグ付きの enum
  (`DeformerKind` `Segment` `KeyformInference`) は `#[serde(other)]` で読める (中身は読み捨てられる)。
  隣接タグ付きの `CurveTarget` は `#[serde(other)]` が中身を読めないので、`Deserialize` を手で書いた。
  `#[serde(skip_serializing)]` で `Unknown` のシリアライズはエラーになり、`write_bundle` と `model_to_json` は
  先に `ugoe_model::unknown` で調べて `FormatError::UnknownValues(場所)` を返す (再保存の扱いは仕様書の「再保存」)。
- フォールバック: デフォーマは恒等変換 (`world_transform` が親をたどる)、合成モードは `BlendMode::effective`
  (ランタイムの `DrawableState::blend` で適用、レンダラの `blend_state` も `Normal` と同じ)、マスクの結合は
  `CompiledModel::mask_mode` で和集合、未知のセグメントを含むカーブは `ugoe_motion::curve::evaluate` が `None`。
- 範囲外の変更: `Segment::Unknown` の追加で `ugoe-motion` の `curve.rs` の網羅的な match を直す必要があったので、
  `crates:` に `ugoe-motion` を足した。main に入った T0601 の `ugoe-editor-core` の `DeformerKind` の match も
  同じ理由で直した (未知の種類はキーフォームの設定で種類の不一致エラー、軸の削除では何もしない)。
- 後続: 物理 (T0301) は `PhysicsSource::Unknown` の入出力を、表情・モーション (T0302 など) は
  `ExpressionOp::Unknown` の項目と `CurveTarget::Unknown` のカーブを無視する実装が必要 (仕様書の表に記載済み)。
  互換性テスト用の資料 (`assets/compat/`) は T0115。
